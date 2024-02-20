use ab_glyph::{Font, FontRef};
use minifb::{Key, MouseButton, MouseMode, Window};

static FIRA_CODE_BYTES: &[u8] = include_bytes!("../fonts/Fira_Code/FiraCode-Regular.ttf");
static NOTO_EMOJI_BYTES: &[u8] = include_bytes!("../fonts/Noto_Emoji/NotoEmoji-Regular.ttf");
static NOTO_SANS_JP_BYTES: &[u8] = include_bytes!("../fonts/Noto_Sans_JP/NotoSansJP-Regular.ttf");

const COLOR_OOB: u32 = 0x00000000;
const COLOR_LINE: u32 = 0x00cccc00;
const COLOR_UNOPENED: u32 = 0x00ffff00;
const COLOR_OPENED: u32 = 0x00777700;

const COLOR_TEXT_LIGHT: u32 = COLOR_UNOPENED;
const COLOR_TEXT_DARK: u32 = COLOR_OPENED;

// In pixels
const CELL_SIZE: usize = 32;
const CELL_SIZE_F: f32 = 32.0;

static DIGITS_EN: [char; 10] = ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'];
// Unlike English, these aren't in order in Unicode, so we can't just add a constant to convert.
static DIGITS_JP: [char; 10] = ['0', '一', '二', '三', '四', '五', '六', '七', '八', '九'];

// Note to self: only things that are constant for the duration of the window should go here.
#[derive(Default)]
struct Config {
    rng: fastrand::Rng,
    cell_cols: usize,
    cell_rows: usize,
    mine_count: usize,
    buffer_width: usize,
    buffer_height: usize,
}
impl Config {
    fn board_width(&self) -> usize {
        (CELL_SIZE + 1) * self.cell_cols
    }
    fn board_height(&self) -> usize {
        (CELL_SIZE + 1) * self.cell_rows
    }
}

fn main() {
    let mut cfg = Config {
        rng: fastrand::Rng::new(),
        cell_cols: 10,
        cell_rows: 10,
        mine_count: 3,
        ..Config::default()
    };
    cfg.buffer_width = (CELL_SIZE + 1) * cfg.cell_cols + 1;
    cfg.buffer_height = (CELL_SIZE + 1) * cfg.cell_rows + 1;

    let font = FontRef::try_from_slice(NOTO_SANS_JP_BYTES).unwrap();
    let emoji_font = FontRef::try_from_slice(NOTO_EMOJI_BYTES).unwrap();
    let digits = DIGITS_JP;
    let mut buffer = vec![0u32; cfg.buffer_width * cfg.buffer_height];
    let mut window = Window::new(
        "Minesweeper",
        cfg.buffer_width,
        cfg.buffer_height,
        Default::default(),
    )
    .unwrap();

    // TODO: Would this be simpler & more performant as a HashSet, replacing `mines`, I wonder?
    // TODO: Squares could be represented by their flat index instead of their x,y coords.
    let mut mine_squares = cfg.rng.choose_multiple(
        (0..cfg.cell_rows).flat_map(|y| (0..cfg.cell_cols).map(move |x| (x, y))),
        cfg.mine_count,
    );
    mine_squares.sort();
    let mines: Vec<Vec<bool>> = (0..cfg.cell_rows)
        .map(|y| {
            (0..cfg.cell_cols)
                .map(|x| mine_squares.binary_search(&(x, y)).is_ok())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    let mine_counts: Vec<Vec<u8>> = mines
        .iter()
        .enumerate()
        .map(|(cell_y, row)| {
            row.iter()
                .enumerate()
                .map(|(cell_x, _)| {
                    let mut cnt = 0;
                    do_surrounding(&cfg, cell_x, cell_y, |sx, sy| {
                        if mines[sy][sx] {
                            cnt += 1;
                        }
                    });
                    cnt
                })
                .collect::<Vec<_>>()
        })
        .collect();

    let mut cells: Vec<Vec<Cell>> = vec![vec![Cell::Unopened; cfg.cell_cols]; cfg.cell_rows];
    #[derive(Copy, Clone, PartialEq)]
    #[repr(u8)]
    enum Cell {
        Unopened,
        Opened,
        Flagged,
    }

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

    let mut first_loop = true;
    while window.is_open() && !window.is_key_down(Key::Escape) {
        let mut was_input = true;
        let left_click_cell = mouse_left.check(&cfg, &window);
        let middle_click_cell = mouse_middle.check(&cfg, &window);
        let right_click_cell = mouse_right.check(&cfg, &window);
        if (left_click_cell.is_some() && right_click_cell.is_some())
            || middle_click_cell.is_some()
            || (left_click_cell.is_some()
                && (window.is_key_down(Key::LeftShift) || window.is_key_down(Key::RightShift)))
        {
            let (cell_x, cell_y) = middle_click_cell.or(left_click_cell).unwrap();
            match cells[cell_y][cell_x] {
                Cell::Unopened | Cell::Flagged => {}
                Cell::Opened => {
                    let mut flag_count = 0;
                    do_surrounding(&cfg, cell_x, cell_y, |sx, sy| {
                        if cells[sy][sx] == Cell::Flagged {
                            flag_count += 1;
                        }
                    });
                    let mine_count = mine_counts[cell_y][cell_x];
                    if flag_count == mine_count {
                        do_surrounding(&cfg, cell_x, cell_y, |sx, sy| {
                            let cell = &mut cells[sy][sx];
                            match cell {
                                Cell::Unopened => {
                                    *cell = Cell::Opened;
                                }
                                Cell::Flagged | Cell::Opened => {}
                            }
                        });
                    } else {
                        play_bell();
                    }
                }
            }
        } else {
            // If the *other* button is clicked, it seems like a misclick.
            let left_click_cell = left_click_cell.filter(|_| !mouse_right.held.is_some());
            let right_click_cell = right_click_cell.filter(|_| !mouse_left.held.is_some());
            if let Some((cell_x, cell_y)) = left_click_cell {
                let cell = &mut cells[cell_y][cell_x];
                match cell {
                    Cell::Unopened => {
                        *cell = Cell::Opened;
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
            } else if !first_loop {
                was_input = false;
            }
        }

        // Skip updating the buffer until there is input.
        if first_loop || was_input {
            for (i, px) in buffer.iter_mut().enumerate() {
                let row = i / cfg.buffer_width;
                let col = i % cfg.buffer_width;
                *px = if row > mines.len() * (CELL_SIZE + 1)
                    || col > mines[0].len() * (CELL_SIZE + 1)
                {
                    COLOR_OOB
                } else if row % (CELL_SIZE + 1) == 0 || col % (CELL_SIZE + 1) == 0 {
                    COLOR_LINE
                } else {
                    let (cell_x, cell_y) = pos_to_cell(&cfg, (col, row)).expect("somehow OoB");
                    match cells[cell_y][cell_x] {
                        Cell::Unopened => COLOR_UNOPENED,
                        Cell::Opened => COLOR_OPENED,
                        Cell::Flagged => COLOR_UNOPENED,
                    }
                };
            }

            let mut mines_left = mines.iter().flatten().filter(|&&b| b).count() as isize;
            for (cell_y, cell_row) in cells.iter().enumerate() {
                for (cell_x, &cell) in cell_row.iter().enumerate() {
                    match cell {
                        Cell::Unopened => {}
                        Cell::Opened => {
                            if mines[cell_y][cell_x] {
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

                            let mine_count = mine_counts[cell_y][cell_x];
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
                                COLOR_TEXT_DARK,
                                cell_x,
                                cell_y,
                                buffer.as_mut_slice(),
                            );
                        }
                    }
                }
            }

            window.set_title(&format!("Minesweeper - {mines_left}💣"));
        }

        first_loop = false;
        window
            .update_with_buffer(&buffer, cfg.buffer_width, cfg.buffer_height)
            .unwrap();
    }
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
                self.held = pos_to_cell_f(cfg, pos);
            }
            return None;
        }
        if let Some((cell_x, cell_y)) = self.held {
            self.held = None;
            if let Some((new_cell_x, new_cell_y)) = window
                .get_mouse_pos(MouseMode::Discard)
                .and_then(|pos| pos_to_cell_f(cfg, pos))
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

fn pos_to_cell(cfg: &Config, (x, y): (usize, usize)) -> Option<(usize, usize)> {
    if x < cfg.board_width()
        && x % (CELL_SIZE + 1) != 0
        && y < cfg.board_height()
        && y % (CELL_SIZE + 1) != 0
    {
        Some((x / (CELL_SIZE + 1), y / (CELL_SIZE + 1)))
    } else {
        None
    }
}
fn pos_to_cell_f(cfg: &Config, (x, y): (f32, f32)) -> Option<(usize, usize)> {
    // Truncate the floats
    let x = x as usize;
    let y = y as usize;
    pos_to_cell(cfg, (x, y))
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

fn lerp_colors(min: u32, max: u32, amt: f32) -> u32 {
    let min_bytes = min.to_be_bytes();
    let max_bytes = max.to_be_bytes();
    u32::from_be_bytes([
        0, // always zero
        lerp_u8(min_bytes[1], max_bytes[1], amt),
        lerp_u8(min_bytes[2], max_bytes[2], amt),
        lerp_u8(min_bytes[3], max_bytes[3], amt),
    ])
}

fn lerp_u8(min: u8, max: u8, amt: f32) -> u8 {
    let min = i16::from(min);
    let max = i16::from(max);
    (min + ((max - min) as f32 * amt) as i16) as u8
}

fn play_bell() {
    use std::io::Write;
    let mut stdout = std::io::stdout();
    stdout.write(b"\x07").unwrap();
    stdout.flush().unwrap();
}
