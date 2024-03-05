// It complains about `&Vec<Vec<_>>`. I'm not incurring the complexity of making
// them _and_ their contents generic.
#![allow(clippy::ptr_arg)]

use ab_glyph::{Font, FontRef, ScaleFont};
use glam::IVec2;
use minifb::{Key, MouseButton, MouseMode, Window};
use std::time::{Duration, Instant};

use crate::{shared, text};
use shared::{Config, CELL_SIZE, CELL_SIZE_F};

/// The number of cells that *must* be free of mines at the start of the game.
pub const SAFE_CELLS_FOR_FIRST_CLICK: usize = 9; // 1 + 8 surrounding cells

// static DIGITS_EN: [char; 10] = ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'];
// Unlike English, these aren't in order in Unicode, so we can't just add a constant to convert.
static DIGITS_JP: [char; 10] = ['0', '一', '二', '三', '四', '五', '六', '七', '八', '九'];

pub enum GameEnd {
    // Restart,
    Quit,
}

pub fn run(cfg: &mut Config) -> GameEnd {
    cfg.buffer_width = (CELL_SIZE + 1) * cfg.cell_cols + 1;
    cfg.buffer_height = (CELL_SIZE + 1) * cfg.cell_rows + 1;

    // let font_en = FontRef::try_from_slice(shared::FIRA_CODE_BYTES).unwrap();
    let font_jp = FontRef::try_from_slice(shared::NOTO_SANS_JP_BYTES).unwrap();

    let font = &font_jp;
    let emoji_font = FontRef::try_from_slice(shared::NOTO_EMOJI_BYTES).unwrap();
    let digits = DIGITS_JP;
    let mut buffer = vec![0u32; cfg.buffer_width * cfg.buffer_height];
    let mut window = Window::new(
        "Minesweeper",
        cfg.buffer_width,
        cfg.buffer_height,
        Default::default(),
    )
    .unwrap();

    // const MENU_ID_NEW_GAME: usize = 1;
    // const MENU_ID_QUIT: usize = 2;
    // const MENU_ID_LANG_EN: usize = 3;
    // const MENU_ID_LANG_JP: usize = 4;

    // fn create_menubar(cfg: &Config, window: &mut Window) -> Vec<MenuHandle> {
    //     let mut menu_handles = Vec::new();

    //     let mut game_menu = Menu::new(cfg.en_jp("Game", "ゲーム")).unwrap();
    //     game_menu
    //         .add_item(cfg.en_jp("New Game", "新しいゲーム"), MENU_ID_NEW_GAME)
    //         .shortcut(Key::N, minifb::MENU_KEY_CTRL)
    //         .build();
    //     game_menu
    //         .add_item(cfg.en_jp("Quit", "ゲームをやめる"), MENU_ID_QUIT)
    //         .shortcut(Key::F4, minifb::MENU_KEY_ALT)
    //         .build();
    //     menu_handles.push(window.add_menu(&game_menu));

    //     let mut options_menu = Menu::new(cfg.en_jp("Options", "設定")).unwrap();
    //     let mut lang_menu = Menu::new(cfg.en_jp("Language", "言語")).unwrap();
    //     lang_menu
    //         .add_item(cfg.en_jp("English", "English（英語）"), MENU_ID_LANG_EN)
    //         .build();
    //     lang_menu
    //         .add_item(cfg.en_jp("日本語 (Japanese)", "日本語"), MENU_ID_LANG_JP)
    //         .build();
    //     options_menu.add_sub_menu(cfg.en_jp("Language", "言語"), &lang_menu);
    //     menu_handles.push(window.add_menu(&options_menu));

    //     menu_handles
    // }
    // fn destroy_menubar(window: &mut Window, menu_handles: Vec<MenuHandle>) {
    //     for menu_handle in menu_handles {
    //         window.remove_menu(menu_handle);
    //     }
    // }

    // let mut menu_handles = create_menubar(cfg, &mut window);

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

    let mut needs_update = true;
    let mut move_count = 0;
    let mut is_game_over = false;
    let mut just_won = false;
    let mut just_lost = false;
    while window.is_open() && !window.is_key_down(Key::Escape) {
        // if let Some(menu_id) = window.is_menu_pressed() {
        //     match menu_id {
        //         MENU_ID_NEW_GAME => return GameEnd::Restart,
        //         MENU_ID_QUIT => return GameEnd::Quit,
        //         MENU_ID_LANG_EN | MENU_ID_LANG_JP => {
        //             cfg.lang = if menu_id == MENU_ID_LANG_EN {
        //                 Lang::En
        //             } else {
        //                 Lang::Jp
        //             };
        //             // font = cfg.en_jp(&font_en, &font_jp);
        //             // digits = cfg.en_jp(DIGITS_EN, DIGITS_JP);
        //             needs_update = true;
        //             destroy_menubar(&mut window, menu_handles);
        //             menu_handles = create_menubar(cfg, &mut window);
        //         }
        //         _ => {}
        //     }
        // }

        // Skip processing clicks when the game is over.
        let mut was_input = !is_game_over;
        'input_block: {
            let accept_input = !is_game_over
                || showing_message_since
                    .map(|d| Instant::now() - d > Duration::from_secs_f32(1.0))
                    .unwrap_or(false);
            if accept_input {
                let mut left_click_cell = mouse_left.check(cfg, &window);
                let mut middle_click_cell = mouse_middle.check(cfg, &window);
                let mut right_click_cell = mouse_right.check(cfg, &window);
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
                            do_surrounding(cfg, cell_x, cell_y, |sx, sy| {
                                if cells[sy][sx] == Cell::Flagged {
                                    flag_count += 1;
                                }
                            });
                            let mine_count = mine_counts[cfg.cell_coords_to_idx(cell_x, cell_y)];
                            if flag_count == mine_count {
                                let mut opened_any_mines = false;
                                do_surrounding(cfg, cell_x, cell_y, |sx, sy| {
                                    let cell = &mut cells[sy][sx];
                                    match cell {
                                        Cell::Unopened => {
                                            open_cell(
                                                cfg,
                                                sx,
                                                sy,
                                                &mut cells,
                                                &mut mine_counts,
                                                &mut mines,
                                            );
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
                                just_won = !just_lost && all_safe_cells_opened(cfg, &mines, &cells);
                                is_game_over = is_game_over || just_lost || just_won;
                            } else {
                                play_bell();
                            }
                        }
                    }
                } else {
                    // If the *other* button is clicked, it seems like a misclick.
                    left_click_cell = left_click_cell.filter(|_| mouse_right.held.is_none());
                    right_click_cell = right_click_cell.filter(|_| mouse_left.held.is_none());
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
                                    initialize_mines(cfg, &mut rng, &mut mines, (cell_x, cell_y));
                                    generate_mine_counts(cfg, &mut mine_counts, &mines);
                                }

                                open_cell(
                                    cfg,
                                    cell_x,
                                    cell_y,
                                    &mut cells,
                                    &mut mine_counts,
                                    &mut mines,
                                );

                                move_count += 1;

                                // Don't return/break so that the board gets updated one last time.
                                just_lost = mines[cfg.cell_coords_to_idx(cell_x, cell_y)];
                                just_won = !just_lost && all_safe_cells_opened(cfg, &mines, &cells);
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
        needs_update |= was_input;
        if needs_update {
            for (i, px) in buffer.iter_mut().enumerate() {
                let row = i / cfg.buffer_width;
                let col = i % cfg.buffer_width;
                *px = if row > cfg.cell_rows * (CELL_SIZE + 1)
                    || col > cfg.cell_cols * (CELL_SIZE + 1)
                {
                    shared::COLOR_OOB
                } else if row % (CELL_SIZE + 1) == 0 || col % (CELL_SIZE + 1) == 0 {
                    shared::COLOR_LINE
                } else {
                    let (cell_x, cell_y) = cfg.pos_to_cell((col, row)).expect("somehow OoB");
                    match cells[cell_y][cell_x] {
                        Cell::Unopened => shared::COLOR_UNOPENED,
                        Cell::Opened => shared::COLOR_OPENED,
                        Cell::Flagged => shared::COLOR_UNOPENED,
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
                                    cfg,
                                    &emoji_font,
                                    '💣',
                                    shared::COLOR_TEXT_DARK,
                                    cell_x,
                                    cell_y,
                                    buffer.as_mut_slice(),
                                );
                            }
                        }

                        Cell::Opened => {
                            if mines[i] {
                                draw_char_in_cell(
                                    cfg,
                                    &emoji_font,
                                    '💣',
                                    shared::COLOR_TEXT_LIGHT,
                                    cell_x,
                                    cell_y,
                                    buffer.as_mut_slice(),
                                );
                                continue;
                            }

                            let mine_count = mine_counts[i];
                            if mine_count > 0 {
                                draw_char_in_cell(
                                    cfg,
                                    font,
                                    digits[usize::from(mine_count)],
                                    shared::COLOR_TEXT_LIGHT,
                                    cell_x,
                                    cell_y,
                                    buffer.as_mut_slice(),
                                );
                            }
                        }

                        Cell::Flagged => {
                            mines_left -= 1;
                            draw_char_in_cell(
                                cfg,
                                &emoji_font,
                                '🚩',
                                if is_game_over && !mines[i] {
                                    shared::COLOR_TEXT_WRONG_FLAG
                                } else {
                                    shared::COLOR_TEXT_DARK
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
                    cfg,
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

            window.set_title(&format!(
                "{} - {mines_left}💣",
                cfg.en_jp("Minesweeper", "マインスイーパ")
            ));

            needs_update = false;
        }

        window
            .update_with_buffer(&buffer, cfg.buffer_width, cfg.buffer_height)
            .unwrap();
    }

    GameEnd::Quit
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
    do_surrounding(cfg, click_x, click_y, |sx, sy| {
        safe_zone.push(cfg.cell_coords_to_idx(sx, sy))
    });
    let random_indices = rng.choose_multiple(
        0..cfg.cell_rows * cfg.cell_cols,
        cfg.mine_count + SAFE_CELLS_FOR_FIRST_CLICK,
    );
    let mut mines_placed = 0;
    mines.fill(false);
    if cfg.mine_count > 0 {
        for i in random_indices {
            if safe_zone.contains(&i) {
                continue;
            }
            mines[i] = true;
            mines_placed += 1;
            if mines_placed == cfg.mine_count {
                break;
            }
        }
    }
    debug_assert_eq!(cfg.mine_count, mines.iter().filter(|&&b| b).count());
}

fn generate_mine_counts(cfg: &Config, mine_counts: &mut [u8], mines: &[bool]) {
    for cell_y in 0..cfg.cell_rows {
        for cell_x in 0..cfg.cell_cols {
            mine_counts[cfg.cell_coords_to_idx(cell_x, cell_y)] =
                count_nearby_mines(cfg, cell_x, cell_y, mines);
        }
    }
}

fn count_nearby_mines(cfg: &Config, cell_x: usize, cell_y: usize, mines: &[bool]) -> u8 {
    let mut count = 0;
    do_surrounding(cfg, cell_x, cell_y, |sx, sy| {
        if mines[cfg.cell_coords_to_idx(sx, sy)] {
            count += 1;
        }
    });
    count
}

/// Opens the cell. If it's a 0, auto-opens the surrounding cells, etc. If it's
/// a mine, tries to move it to a neighboring cell, if it wouldn't change the
/// revealed information, to help reduce the need for the player to guess.
fn open_cell(
    cfg: &Config,
    sx: usize,
    sy: usize,
    cells: &mut Vec<Vec<Cell>>,
    mine_counts: &mut [u8],
    mines: &mut Box<[bool]>,
) {
    let mut cells_to_process = Vec::new();
    if cells[sy][sx] == Cell::Unopened {
        cells_to_process.push((sx, sy));
    }
    while let Some((x, y)) = cells_to_process.pop() {
        if !mines[cfg.cell_coords_to_idx(x, y)] {
            _ = try_move_mine(cfg, x, y, cells, mine_counts, mines);
        }
        cells[y][x] = Cell::Opened;
        if mine_counts[cfg.cell_coords_to_idx(x, y)] == 0 {
            do_surrounding(cfg, x, y, |ssx, ssy| {
                if cells[ssy][ssx] == Cell::Unopened {
                    cells_to_process.push((ssx, ssy));
                }
            });
        }
    }
}

fn try_move_mine(
    cfg: &Config,
    cell_x: usize,
    cell_y: usize,
    cells: &Vec<Vec<Cell>>,
    mine_counts: &mut [u8],
    mines: &mut Box<[bool]>,
) -> bool {
    let mut found_solution = false;
    let mut new_mines = vec![false; mines.len()].into_boxed_slice();
    do_surrounding(cfg, cell_x, cell_y, |sx, sy| {
        // Can't grab a mine from a non-mine.
        if found_solution || !mines[cfg.cell_coords_to_idx(sx, sy)] {
            if shared::DEBUG_ANTI_GUESS && !found_solution {
                println!("Anti-guess: rejected {sx},{sy} because there's no mine there.");
            }
            return;
        }

        new_mines.clone_from_slice(mines);
        new_mines[cfg.cell_coords_to_idx(cell_x, cell_y)] = true;
        new_mines[cfg.cell_coords_to_idx(sx, sy)] = false;

        let mut any_changes_to_revealed_numbers = false;
        let mut numbers_that_would_be_changed = Vec::new();
        for (x, y) in [(cell_x, cell_y), (sx, sy)] {
            do_surrounding(cfg, x, y, |ssx, ssy| {
                if cells[ssy][ssx] == Cell::Opened
                    && mine_counts[cfg.cell_coords_to_idx(ssx, ssy)]
                        != count_nearby_mines(cfg, ssx, ssy, &new_mines)
                {
                    any_changes_to_revealed_numbers = true;
                    if shared::DEBUG_ANTI_GUESS {
                        numbers_that_would_be_changed.push((
                            ssx,
                            ssy,
                            mine_counts[cfg.cell_coords_to_idx(ssx, ssy)],
                        ));
                    }
                }
            });
        }
        if any_changes_to_revealed_numbers {
            if shared::DEBUG_ANTI_GUESS {
                println!(
                    "Anti-guess: rejected {sx},{sy} because it would change the {}.",
                    numbers_that_would_be_changed
                        .into_iter()
                        .map(|(_ssx, _ssy, num)| format!("{num}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }
            return;
        }

        found_solution = true;
    });

    if !found_solution {
        return false;
    }

    if shared::DEBUG_ANTI_GUESS {
        let differences: Vec<_> = (0..cfg.cell_rows)
            .flat_map(|y| (0..cfg.cell_cols).map(move |x| (x, y)))
            .filter(|&(x, y)| {
                mines[cfg.cell_coords_to_idx(x, y)] != new_mines[cfg.cell_coords_to_idx(x, y)]
                    && (x, y) != (cell_x, cell_y)
            })
            .collect();
        let (new_x, new_y) = match *differences {
            [coords] => coords,
            [] => panic!("Moved a mine, but the board is identical?"),
            [_, _, ..] => panic!("Moved a mine, but multiple cells changed?"),
        };
        println!("Anti-guess: moved a mine to {cell_x},{cell_y} from {new_x},{new_y}.");
    }
    *mines = new_mines;
    // Regenerate the whole board so we can notice bugs more easily.
    generate_mine_counts(cfg, mine_counts, mines);
    true
}

fn all_safe_cells_opened(cfg: &Config, mines: &[bool], cells: &Vec<Vec<Cell>>) -> bool {
    !cells
        .iter()
        .enumerate()
        .flat_map(|(y, row)| row.iter().enumerate().map(move |(x, _)| (x, y)))
        .filter(|&(x, y)| !mines[cfg.cell_coords_to_idx(x, y)])
        .any(|(x, y)| {
            if cells[y][x] == Cell::Unopened {
                if shared::DEBUG_PRINTS {
                    println!("Some cells are still unopened (e.g., {x},{y}).");
                }
                true
            } else {
                false
            }
        })
}

fn show_message<F, FS>(cfg: &Config, msg: &str, font: FS, buffer: &mut [u32])
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
        shared::draw_rectangle(
            IVec2::new(box_left as i32, box_top as i32),
            IVec2::new(box_width as i32, box_height as i32),
            shared::COLOR_MESSAGE_BORDER,
            buffer,
            cfg.buffer_width,
        );
        shared::draw_rectangle(
            IVec2::new((box_left + outline) as i32, (box_top + outline) as i32),
            IVec2::new(
                (box_width - outline * 2) as i32,
                (box_height - outline * 2) as i32,
            ),
            shared::COLOR_MESSAGE_BOX,
            buffer,
            cfg.buffer_width,
        );
    }
    text::draw_glyphs(
        glyphs.into_iter(),
        IVec2::new(left_margin as i32, top_margin as i32),
        &font,
        shared::COLOR_MESSAGE_TEXT,
        buffer,
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
            if self.held.is_some() {
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
        None
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
        buffer[i] = shared::lerp_colors(buffer[i], color, f32::min(c, 1.0));
    });
}

fn play_bell() {
    use std::io::Write;
    let mut stdout = std::io::stdout();
    stdout.write_all(b"\x07").unwrap();
    stdout.flush().unwrap();
}
