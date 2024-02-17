use minifb::{Key, MouseButton, MouseMode, Window};

const COLOR_OOB: u32 = 0x00000000;
const COLOR_LINE: u32 = 0x00cccc00;
const COLOR_UNOPENED: u32 = 0x00ffff00;
const COLOR_OPENED: u32 = 0x00777700;

fn main() {
    const WIDTH: usize = (CELL_SIZE + 1) * 10 + 1;
    const HEIGHT: usize = (CELL_SIZE + 1) * 10 + 1;

    let mut buffer = vec![0u32; WIDTH * HEIGHT];
    let mut window = Window::new("Rust test", WIDTH, HEIGHT, Default::default()).unwrap();

    const cell_cols: usize = 10;
    const cell_rows: usize = 10;
    let board_width = (CELL_SIZE + 1) * cell_cols;
    let board_height = (CELL_SIZE + 1) * cell_rows;

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

    const CELL_SIZE: usize = 32;

    let mut mouse_held_cell: Option<(usize, usize)> = None;
    while window.is_open() && !window.is_key_down(Key::Escape) {
        let pos_to_cell = |(x, y)| {
            if x < board_width
                && x % (CELL_SIZE + 1) != 0
                && y < board_height
                && y % (CELL_SIZE + 1) != 0
            {
                Some((x / (CELL_SIZE + 1), y / (CELL_SIZE + 1)))
            } else {
                None
            }
        };
        let pos_to_cell_f = |(x, y)| {
            let x = x as usize;
            let y = y as usize;
            pos_to_cell((x, y))
        };
        if window.get_mouse_down(MouseButton::Left) {
            if let Some(cell) = mouse_held_cell {
                // The mouse was clicked in a previous frame. We're waiting for it to be released.
            } else if let Some(pos) = window.get_mouse_pos(MouseMode::Discard) {
                mouse_held_cell = pos_to_cell_f(pos);
            }
        } else if let Some((cell_x, cell_y)) = mouse_held_cell {
            if let Some((new_cell_x, new_cell_y)) = window
                .get_mouse_pos(MouseMode::Discard)
                .and_then(pos_to_cell_f)
            {
                if cell_x == new_cell_x && cell_y == new_cell_y {
                    println!("clicked {cell_x},{cell_y}");
                    match cells[cell_y][cell_x] {
                        Cell::Unopened => {
                            cells[cell_y][cell_x] = Cell::Opened;
                        }
                        Cell::Opened => {}
                        Cell::Flagged => {}
                    }
                }
            }
            mouse_held_cell = None;
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
        window.update_with_buffer(&buffer, WIDTH, HEIGHT).unwrap();
    }
}
