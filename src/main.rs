use ab_glyph::{Font, FontRef};
use minifb::{Key, MouseButton, MouseMode, Window};

static FIRA_CODE_BYTES: &[u8] = include_bytes!("../fonts/Fira_Code/FiraCode-Regular.ttf");
static NOTO_EMOJI_BYTES: &[u8] = include_bytes!("../fonts/Noto_Emoji/NotoEmoji-Regular.ttf");

const COLOR_OOB: u32 = 0x00000000;
const COLOR_LINE: u32 = 0x00cccc00;
const COLOR_UNOPENED: u32 = 0x00ffff00;
const COLOR_OPENED: u32 = 0x00777700;

const COLOR_TEXT_LIGHT: u32 = COLOR_UNOPENED;
const COLOR_TEXT_DARK: u32 = COLOR_OPENED;

const WIDTH: usize = (CELL_SIZE + 1) * 10 + 1;
const HEIGHT: usize = (CELL_SIZE + 1) * 10 + 1;

// In pixels
const CELL_SIZE: usize = 32;
const CELL_SIZE_F: f32 = 32.0;

const cell_cols: usize = 10;
const cell_rows: usize = 10;
const board_width: usize = (CELL_SIZE + 1) * cell_cols;
const board_height: usize = (CELL_SIZE + 1) * cell_rows;

fn main() {
    let font = FontRef::try_from_slice(FIRA_CODE_BYTES).unwrap();
    let emoji_font = FontRef::try_from_slice(NOTO_EMOJI_BYTES).unwrap();
    let mut buffer = vec![0u32; WIDTH * HEIGHT];
    let mut window = Window::new("Rust test", WIDTH, HEIGHT, Default::default()).unwrap();

    let mines: [[bool; cell_cols]; cell_rows] = [
        [
            false, false, false, true, false, false, false, false, true, true,
        ],
        [
            false, false, true, false, false, false, true, true, false, true,
        ],
        [
            true, false, false, true, false, false, false, true, false, true,
        ],
        [
            false, false, false, true, true, true, true, true, false, true,
        ],
        [
            true, true, false, true, true, false, false, false, true, true,
        ],
        [
            true, true, false, true, false, false, true, false, true, true,
        ],
        [true, true, false, true, true, true, true, false, true, true],
        [
            false, true, true, true, true, true, false, true, true, false,
        ],
        [
            false, true, true, false, true, false, true, true, false, true,
        ],
        [
            true, true, true, true, false, false, false, false, false, true,
        ],
    ];

    let mut cells: [[Cell; cell_cols]; cell_rows] = Default::default();
    #[derive(Copy, Clone)]
    #[repr(u8)]
    enum Cell {
        Unopened,
        Opened,
        Flagged,
    }
    impl Default for Cell {
        fn default() -> Cell {
            Cell::Unopened
        }
    }

    let mut mouse_left = CellsMouseState {
        button: MouseButton::Left,
        held: None,
    };
    let mut mouse_right = CellsMouseState {
        button: MouseButton::Right,
        held: None,
    };
    while window.is_open() && !window.is_key_down(Key::Escape) {
        let left_clicked_cell = mouse_left.check(&window);
        let right_clicked_cell = mouse_right.check(&window);
        if left_clicked_cell.is_some() && right_clicked_cell.is_some() {
            todo!("chording, or w/e it's called");
        } else {
            if let Some((cell_x, cell_y)) = left_clicked_cell {
                let cell = &mut cells[cell_y][cell_x];
                match cell {
                    Cell::Unopened => {
                        *cell = Cell::Opened;
                    }
                    Cell::Opened => {}
                    Cell::Flagged => {}
                }
            }
            if let Some((cell_x, cell_y)) = right_clicked_cell {
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
            }
        }

        for (i, px) in buffer.iter_mut().enumerate() {
            let row = i / WIDTH;
            let col = i % WIDTH;
            *px = if row > mines.len() * (CELL_SIZE + 1) || col > mines[0].len() * (CELL_SIZE + 1) {
                COLOR_OOB
            } else if row % (CELL_SIZE + 1) == 0 || col % (CELL_SIZE + 1) == 0 {
                COLOR_LINE
            } else {
                let (cell_x, cell_y) = pos_to_cell((col, row)).expect("somehow OoB");
                match cells[cell_y][cell_x] {
                    Cell::Unopened => COLOR_UNOPENED,
                    Cell::Opened => COLOR_OPENED,
                    Cell::Flagged => COLOR_UNOPENED,
                }
            };
        }

        for (cell_y, cell_row) in cells.iter().enumerate() {
            for (cell_x, &cell) in cell_row.iter().enumerate() {
                match cell {
                    Cell::Unopened => {}
                    Cell::Opened => {
                        if mines[cell_y][cell_x] {
                            draw_char_in_cell(
                                &emoji_font,
                                '💣',
                                COLOR_TEXT_LIGHT,
                                cell_x,
                                cell_y,
                                buffer.as_mut_slice(),
                            );
                            continue;
                        }

                        let mut mine_count = 0;
                        if cell_x > 0 {
                            if cell_y > 0 && mines[cell_y - 1][cell_x - 1] {
                                mine_count += 1;
                            }
                            if mines[cell_y][cell_x - 1] {
                                mine_count += 1;
                            }
                            if cell_y < mines.len() - 1 && mines[cell_y + 1][cell_x - 1] {
                                mine_count += 1;
                            }
                        }
                        {
                            if cell_y > 0 && mines[cell_y - 1][cell_x] {
                                mine_count += 1;
                            }
                            // Obviously no need to check mines[cell_y][cell_x]
                            if cell_y < mines.len() - 1 && mines[cell_y + 1][cell_x] {
                                mine_count += 1;
                            }
                        }
                        if cell_x < mines[0].len() - 1 {
                            if cell_y > 0 && mines[cell_y - 1][cell_x + 1] {
                                mine_count += 1;
                            }
                            if mines[cell_y][cell_x + 1] {
                                mine_count += 1;
                            }
                            if cell_y < mines.len() - 1 && mines[cell_y + 1][cell_x + 1] {
                                mine_count += 1;
                            }
                        }
                        draw_char_in_cell(
                            &font,
                            char::from_digit(mine_count, 10).unwrap(),
                            COLOR_TEXT_LIGHT,
                            cell_x,
                            cell_y,
                            buffer.as_mut_slice(),
                        );
                    }
                    Cell::Flagged => {
                        draw_char_in_cell(
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

        window.update_with_buffer(&buffer, WIDTH, HEIGHT).unwrap();
    }
}

struct CellsMouseState {
    button: MouseButton,
    held: Option<(usize, usize)>,
}
impl CellsMouseState {
    fn check(&mut self, window: &Window) -> Option<(usize, usize)> {
        if window.get_mouse_down(self.button) {
            if let Some(_) = self.held {
                // The mouse was clicked in a previous frame. We're waiting for it to be released.
            } else if let Some(pos) = window.get_mouse_pos(MouseMode::Discard) {
                self.held = pos_to_cell_f(pos);
            }
            return None;
        }
        if let Some((cell_x, cell_y)) = self.held {
            self.held = None;
            if let Some((new_cell_x, new_cell_y)) = window
                .get_mouse_pos(MouseMode::Discard)
                .and_then(pos_to_cell_f)
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

fn pos_to_cell((x, y): (usize, usize)) -> Option<(usize, usize)> {
    if x < board_width && x % (CELL_SIZE + 1) != 0 && y < board_height && y % (CELL_SIZE + 1) != 0 {
        Some((x / (CELL_SIZE + 1), y / (CELL_SIZE + 1)))
    } else {
        None
    }
}
fn pos_to_cell_f((x, y): (f32, f32)) -> Option<(usize, usize)> {
    // Truncate the floats
    let x = x as usize;
    let y = y as usize;
    pos_to_cell((x, y))
}

/// Draws a char at x,y in the (flat) buffer.
fn draw_char_in_cell(
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
        if c < 0.5 {
            return;
        }
        let mut x: usize = x.try_into().unwrap();
        x += board_x;
        x += offset_x;
        let mut y: usize = y.try_into().unwrap();
        y += board_y;
        y += offset_y;
        let i = y * WIDTH + x;
        buffer[i] = color;
    });
}
