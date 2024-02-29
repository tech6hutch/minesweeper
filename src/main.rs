use ab_glyph::{Font, FontRef, ScaleFont};
use glam::IVec2;
use minifb::{Key, MouseButton, MouseMode, Window};
use std::time::{Duration, Instant};

mod setup_window;
mod shared;
mod text;

use shared::*;

/// The number of cells that *must* be free of mines at the start of the game.
const SAFE_CELLS_FOR_FIRST_CLICK: usize = 9; // 1 + 8 surrounding cells

static DIGITS_EN: [char; 10] = ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'];
// Unlike English, these aren't in order in Unicode, so we can't just add a constant to convert.
static DIGITS_JP: [char; 10] = ['0', '一', '二', '三', '四', '五', '六', '七', '八', '九'];

fn main() {
    let Some(mut cfg) = setup_window::run() else {
        return;
    };
    cfg.buffer_width = (CELL_SIZE + 1) * cfg.cell_cols + 1;
    cfg.buffer_height = (CELL_SIZE + 1) * cfg.cell_rows + 1;

    let font = FontRef::try_from_slice(cfg.en_jp(FIRA_CODE_BYTES, NOTO_SANS_JP_BYTES)).unwrap();
    let emoji_font = FontRef::try_from_slice(NOTO_EMOJI_BYTES).unwrap();
    let digits = cfg.en_jp(DIGITS_EN, DIGITS_JP);
    let mut buffer = vec![0u32; cfg.buffer_width * cfg.buffer_height];
    let mut window = Window::new(
        "Minesweeper",
        cfg.buffer_width,
        cfg.buffer_height,
        Default::default(),
    )
    .unwrap();

    let mut showing_message_since: Option<Instant> = None;

    let mut rng = fastrand::Rng::new();

    // Whether each cell has a mine. Gets initialized on first click so we can ensure the player doesn't immediately lose.
    let mut mines: Box<[bool]> = vec![false; cfg.cell_rows * cfg.cell_cols].into_boxed_slice();
    let mut mine_counts: Box<[u8]> = vec![0; cfg.cell_rows * cfg.cell_cols].into_boxed_slice();

    let mut cells: Vec<Vec<Cell>> = vec![vec![Cell::Unopened; cfg.cell_cols]; cfg.cell_rows];

    let mut mouse_left = CellsMouseState {
        button: MouseButton::Left,
        held: None,
    };
    let mut mouse_middle = CellsMouseState {
        button: MouseButton::Middle,
        held: None,
    };
    let mut mouse_right = CellsMouseState {
        button: MouseButton::Right,
        held: None,
    };

    let mut move_count = 0;
    let mut is_game_over = false;
    let mut just_won = false;
    let mut just_lost = false;
    while window.is_open() && !window.is_key_down(Key::Escape) {
        // Skip processing clicks when the game is over.
        let mut was_input = !is_game_over;
        'input_block: {
            let accept_input = !is_game_over
                || showing_message_since
                    .map(|d| Instant::now() - d > Duration::from_secs_f32(1.0))
                    .unwrap_or(false);
            if accept_input {
                let mut left_click_cell = mouse_left.check(&cfg, &window);
                let mut middle_click_cell = mouse_middle.check(&cfg, &window);
                let mut right_click_cell = mouse_right.check(&cfg, &window);
                if showing_message_since.is_some() && left_click_cell.is_some() {
                    was_input = true; // update window
                    showing_message_since = None;
                    break 'input_block;
                }
                if middle_click_cell.is_none()
                    && ((left_click_cell.is_some() && right_click_cell.is_some())
                        || (left_click_cell.is_some()
                            && (window.is_key_down(Key::LeftShift)
                                || window.is_key_down(Key::RightShift))))
                {
                    middle_click_cell = left_click_cell.take();
                    right_click_cell = None;
                }
                if let Some((cell_x, cell_y)) = middle_click_cell {
                    // Chording/multi-open/whatever-you-want-to-call-it: if there are enough flags, open the cells all around.
                    match cells[cell_y][cell_x] {
                        Cell::Unopened | Cell::Flagged => {}
                        Cell::Opened => {
                            let mut flag_count = 0;
                            do_surrounding(&cfg, cell_x, cell_y, |sx, sy| {
                                if cells[sy][sx] == Cell::Flagged {
                                    flag_count += 1;
                                }
                            });
                            let mine_count = mine_counts[cfg.cell_coords_to_idx(cell_x, cell_y)];
                            if flag_count == mine_count {
                                let mut opened_any_mines = false;
                                do_surrounding(&cfg, cell_x, cell_y, |sx, sy| {
                                    let cell = &mut cells[sy][sx];
                                    match cell {
                                        Cell::Unopened => {
                                            open_cell(&cfg, sx, sy, &mut cells, &mine_counts);
                                            if mines[cfg.cell_coords_to_idx(sx, sy)] {
                                                opened_any_mines = true;
                                            }
                                        }
                                        Cell::Flagged | Cell::Opened => {}
                                    }
                                });
                                move_count += 1;
                                // Don't return/break so that the board gets updated one last time.
                                just_lost = opened_any_mines;
                                just_won =
                                    !just_lost && all_safe_cells_opened(&cfg, &mines, &cells);
                                is_game_over = is_game_over || just_lost || just_won;
                            } else {
                                play_bell();
                            }
                        }
                    }
                } else {
                    // If the *other* button is clicked, it seems like a misclick.
                    left_click_cell = left_click_cell.filter(|_| !mouse_right.held.is_some());
                    right_click_cell = right_click_cell.filter(|_| !mouse_left.held.is_some());
                    if left_click_cell.is_some()
                        && right_click_cell.is_none()
                        && (window.is_key_down(Key::LeftCtrl) || window.is_key_down(Key::RightCtrl))
                    {
                        right_click_cell = left_click_cell.take();
                    }
                    if let Some((cell_x, cell_y)) = left_click_cell {
                        let cell = &mut cells[cell_y][cell_x];
                        match cell {
                            Cell::Unopened => {
                                if move_count == 0 {
                                    initialize_mines(&cfg, &mut rng, &mut mines, (cell_x, cell_y));
                                    count_cell_mines(&cfg, &mut mine_counts, &mines);
                                }

                                open_cell(&cfg, cell_x, cell_y, &mut cells, &mine_counts);

                                move_count += 1;

                                // Don't return/break so that the board gets updated one last time.
                                just_lost = mines[cfg.cell_coords_to_idx(cell_x, cell_y)];
                                just_won =
                                    !just_lost && all_safe_cells_opened(&cfg, &mines, &cells);
                                is_game_over = is_game_over || just_lost || just_won;
                            }
                            Cell::Opened => {}
                            Cell::Flagged => {}
                        }
                    } else if let Some((cell_x, cell_y)) = right_click_cell {
                        let cell = &mut cells[cell_y][cell_x];
                        match cell {
                            Cell::Unopened => {
                                *cell = Cell::Flagged;
                            }
                            Cell::Opened => {}
                            Cell::Flagged => {
                                *cell = Cell::Unopened;
                            }
                        }
                    } else {
                        was_input = false;
                    }
                }
            }
        }

        // Skip updating the buffer until there is input.
        if move_count == 0 || was_input {
            for (i, px) in buffer.iter_mut().enumerate() {
                let row = i / cfg.buffer_width;
                let col = i % cfg.buffer_width;
                *px = if row > cfg.cell_rows * (CELL_SIZE + 1)
                    || col > cfg.cell_cols * (CELL_SIZE + 1)
                {
                    COLOR_OOB
                } else if row % (CELL_SIZE + 1) == 0 || col % (CELL_SIZE + 1) == 0 {
                    COLOR_LINE
                } else {
                    let (cell_x, cell_y) = cfg.pos_to_cell((col, row)).expect("somehow OoB");
                    match cells[cell_y][cell_x] {
                        Cell::Unopened => COLOR_UNOPENED,
                        Cell::Opened => COLOR_OPENED,
                        Cell::Flagged => COLOR_UNOPENED,
                    }
                };
            }

            let mut mines_left = cfg.mine_count;
            for (cell_y, cell_row) in cells.iter().enumerate() {
                for (cell_x, &cell) in cell_row.iter().enumerate() {
                    let i = cfg.cell_coords_to_idx(cell_x, cell_y);
                    match cell {
                        Cell::Unopened => {
                            if is_game_over && mines[i] {
                                draw_char_in_cell(
                                    &cfg,
                                    &emoji_font,
                                    '💣',
                                    COLOR_TEXT_DARK,
                                    cell_x,
                                    cell_y,
                                    buffer.as_mut_slice(),
                                );
                            }
                        }

                        Cell::Opened => {
                            if mines[i] {
                                draw_char_in_cell(
                                    &cfg,
                                    &emoji_font,
                                    '💣',
                                    COLOR_TEXT_LIGHT,
                                    cell_x,
                                    cell_y,
                                    buffer.as_mut_slice(),
                                );
                                continue;
                            }

                            let mine_count = mine_counts[i];
                            if mine_count > 0 {
                                draw_char_in_cell(
                                    &cfg,
                                    &font,
                                    digits[usize::from(mine_count)],
                                    COLOR_TEXT_LIGHT,
                                    cell_x,
                                    cell_y,
                                    buffer.as_mut_slice(),
                                );
                            }
                        }

                        Cell::Flagged => {
                            mines_left -= 1;
                            draw_char_in_cell(
                                &cfg,
                                &emoji_font,
                                '🚩',
                                if is_game_over && !mines[i] {
                                    COLOR_TEXT_WRONG_FLAG
                                } else {
                                    COLOR_TEXT_DARK
                                },
                                cell_x,
                                cell_y,
                                buffer.as_mut_slice(),
                            );
                        }
                    }
                }
            }

            if just_won || just_lost {
                let font = font.as_scaled(CELL_SIZE_F);
                show_message(
                    &cfg,
                    if just_won {
                        cfg.en_jp("You won!", "やった！")
                    } else {
                        cfg.en_jp("You lost!", "負けました。")
                    },
                    font,
                    &mut buffer,
                );
                showing_message_since = Some(Instant::now());
                just_won = false;
                just_lost = false;
            }

            window.set_title(&format!("Minesweeper - {mines_left}💣"));
        }

        window
            .update_with_buffer(&buffer, cfg.buffer_width, cfg.buffer_height)
            .unwrap();
    }
}

fn initialize_mines(
    cfg: &Config,
    rng: &mut fastrand::Rng,
    mines: &mut [bool],
    first_click: (usize, usize),
) {
    let (click_x, click_y) = first_click;
    let mut safe_zone = Vec::with_capacity(SAFE_CELLS_FOR_FIRST_CLICK);
    safe_zone.push(cfg.cell_coords_to_idx(click_x, click_y));
    do_surrounding(&cfg, click_x, click_y, |sx, sy| {
        safe_zone.push(cfg.cell_coords_to_idx(sx, sy))
    });
    let mut mine_squares = rng.choose_multiple(
        0..cfg.cell_rows * cfg.cell_cols,
        cfg.mine_count + SAFE_CELLS_FOR_FIRST_CLICK,
    );
    mine_squares.sort();
    let mut mines_placed = 0;
    if cfg.mine_count > 0 {
        for i in 0..mines.len() {
            if !safe_zone.contains(&i) && mine_squares.binary_search(&i).is_ok() {
                mines[i] = true;
                mines_placed += 1;
                if mines_placed == cfg.mine_count {
                    break;
                }
            }
        }
    }
    debug_assert_eq!(cfg.mine_count, mines.iter().filter(|&&b| b).count());
}

fn count_cell_mines(cfg: &Config, mine_counts: &mut [u8], mines: &[bool]) {
    for cell_y in 0..cfg.cell_rows {
        for cell_x in 0..cfg.cell_cols {
            let cnt = &mut mine_counts[cfg.cell_coords_to_idx(cell_x, cell_y)];
            do_surrounding(&cfg, cell_x, cell_y, |sx, sy| {
                if mines[cfg.cell_coords_to_idx(sx, sy)] {
                    *cnt += 1;
                }
            });
        }
    }
}

/// Auto-opens the cells surrounding a 0, recursively.
fn open_cell(
    cfg: &Config,
    sx: usize,
    sy: usize,
    cells: &mut Vec<Vec<Cell>>,
    mine_counts: &[u8],
) {
    let cell = &mut cells[sy][sx];
    if *cell != Cell::Unopened {
        return;
    }
    *cell = Cell::Opened;
    if mine_counts[cfg.cell_coords_to_idx(sx, sy)] == 0 {
        do_surrounding(&cfg, sx, sy, |ssx, ssy| {
            open_cell(&cfg, ssx, ssy, cells, mine_counts)
        });
    }
}

fn all_safe_cells_opened(cfg: &Config, mines: &[bool], cells: &Vec<Vec<Cell>>) -> bool {
    !cells
        .iter()
        .enumerate()
        .flat_map(|(y, row)| row.iter().enumerate().map(move |(x, _)| (x, y)))
        .filter(|&(x, y)| !mines[cfg.cell_coords_to_idx(x, y)])
        .any(|(x, y)| {
            if cells[y][x] == Cell::Unopened {
                if cfg!(debug_assertions) {
                    println!("Some cells are still unopened (e.g., {x},{y}).");
                }
                true
            } else {
                false
            }
        })
}

fn show_message<F, FS>(cfg: &Config, msg: &str, font: FS, mut buffer: &mut [u32])
where
    F: Font,
    FS: ScaleFont<F>,
{
    let mut glyphs = Vec::new();
    let glyphs_size =
        text::layout_paragraph(&font, cfg.buffer_width as f32 / 2.0, msg, &mut glyphs);
    let left_margin = (cfg.buffer_width - glyphs_size.x as usize) / 2;
    let top_margin = (cfg.buffer_height - glyphs_size.y as usize) / 2;
    // Draw box with outline
    {
        let box_padding_left_right = font.scale().x as usize;
        let box_padding_top_bottom = font.scale().y as usize;
        let box_left = left_margin - box_padding_left_right;
        let box_top = top_margin - box_padding_top_bottom;
        let box_width = box_padding_left_right * 2 + glyphs_size.x as usize;
        let box_height = box_padding_top_bottom * 2 + glyphs_size.y as usize;
        let outline = 2;
        draw_rectangle(
            IVec2::new(box_left as i32, box_top as i32),
            IVec2::new(box_width as i32, box_height as i32),
            COLOR_MESSAGE_BORDER,
            &mut buffer,
            cfg.buffer_width,
        );
        draw_rectangle(
            IVec2::new((box_left + outline) as i32, (box_top + outline) as i32),
            IVec2::new(
                (box_width - outline * 2) as i32,
                (box_height - outline * 2) as i32,
            ),
            COLOR_MESSAGE_BOX,
            &mut buffer,
            cfg.buffer_width,
        );
    }
    text::draw_glyphs(
        glyphs.into_iter(),
        IVec2::new(left_margin as i32, top_margin as i32),
        &font,
        COLOR_MESSAGE_TEXT,
        &mut buffer,
        cfg.buffer_width,
    );
}

#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(u8)]
enum Cell {
    Unopened,
    Opened,
    Flagged,
}

struct CellsMouseState {
    button: MouseButton,
    held: Option<(usize, usize)>,
}
impl CellsMouseState {
    fn check(&mut self, cfg: &Config, window: &Window) -> Option<(usize, usize)> {
        if window.get_mouse_down(self.button) {
            if let Some(_) = self.held {
                // The mouse was clicked in a previous frame. We're waiting for it to be released.
            } else if let Some(pos) = window.get_mouse_pos(MouseMode::Discard) {
                self.held = cfg.pos_to_cell_f(pos);
            }
            return None;
        }
        if let Some((cell_x, cell_y)) = self.held {
            self.held = None;
            if let Some((new_cell_x, new_cell_y)) = window
                .get_mouse_pos(MouseMode::Discard)
                .and_then(|pos| cfg.pos_to_cell_f(pos))
            {
                if cell_x == new_cell_x && cell_y == new_cell_y {
                    return Some((cell_x, cell_y));
                }
            }
            return None;
        }
        return None;
    }
}

#[inline]
fn do_surrounding(cfg: &Config, cell_x: usize, cell_y: usize, mut f: impl FnMut(usize, usize)) {
    if cell_x > 0 {
        if cell_y > 0 {
            f(cell_x - 1, cell_y - 1);
        }
        f(cell_x - 1, cell_y);
        if cell_y < cfg.cell_rows - 1 {
            f(cell_x - 1, cell_y + 1);
        }
    }
    {
        if cell_y > 0 {
            f(cell_x, cell_y - 1);
        }
        // Obviously no need to do f(cell_x, cell_y)
        if cell_y < cfg.cell_rows - 1 {
            f(cell_x, cell_y + 1);
        }
    }
    if cell_x < cfg.cell_cols - 1 {
        if cell_y > 0 {
            f(cell_x + 1, cell_y - 1);
        }
        f(cell_x + 1, cell_y);
        if cell_y < cfg.cell_rows - 1 {
            f(cell_x + 1, cell_y + 1);
        }
    }
}

/// Draws a char at x,y in the (flat) buffer.
fn draw_char_in_cell(
    cfg: &Config,
    font: impl Font,
    c: char,
    color: u32,
    cell_x: usize,
    cell_y: usize,
    buffer: &mut [u32],
) {
    let board_x = cell_x * (CELL_SIZE + 1);
    let board_y = cell_y * (CELL_SIZE + 1);
    let glyph = font.glyph_id(c).with_scale(CELL_SIZE_F);
    let outlined = font.outline_glyph(glyph).expect("couldn't outline glyph");
    let offset_x: usize = ((CELL_SIZE_F - outlined.px_bounds().width()) * 0.5) as usize + 1;
    let offset_y: usize = ((CELL_SIZE_F - outlined.px_bounds().height()) * 0.5) as usize + 1;
    outlined.draw(|x, y, c| {
        let mut x: usize = x.try_into().unwrap();
        x += board_x;
        x += offset_x;
        let mut y: usize = y.try_into().unwrap();
        y += board_y;
        y += offset_y;
        let i = y * cfg.buffer_width + x;
        // Sometimes c is > 1.0 🤷
        buffer[i] = lerp_colors(buffer[i], color, f32::min(c, 1.0));
    });
}

fn play_bell() {
    use std::io::Write;
    let mut stdout = std::io::stdout();
    stdout.write(b"\x07").unwrap();
    stdout.flush().unwrap();
}
