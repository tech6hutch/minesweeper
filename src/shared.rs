use glam::IVec2;

pub static FIRA_CODE_BYTES: &[u8] = include_bytes!("../fonts/Fira_Code/FiraCode-Regular.ttf");
pub static NOTO_EMOJI_BYTES: &[u8] = include_bytes!("../fonts/Noto_Emoji/NotoEmoji-Regular.ttf");
pub static NOTO_SANS_JP_BYTES: &[u8] =
    include_bytes!("../fonts/Noto_Sans_JP/NotoSansJP-Regular.ttf");

pub const COLOR_OOB: u32 = 0x00000000;
pub const COLOR_LINE: u32 = 0x00cccc00;
pub const COLOR_UNOPENED: u32 = 0x00ffff00;
pub const COLOR_OPENED: u32 = 0x00777700;

pub const COLOR_TEXT_LIGHT: u32 = COLOR_UNOPENED;
pub const COLOR_TEXT_DARK: u32 = COLOR_OPENED;
pub const COLOR_TEXT_WRONG_FLAG: u32 = 0x00ff0000;

pub const COLOR_MESSAGE_BOX: u32 = 0x00223377;
pub const COLOR_MESSAGE_BORDER: u32 = 0x00ffffff;
pub const COLOR_MESSAGE_TEXT: u32 = COLOR_MESSAGE_BORDER;

pub const COLOR_BUTTON: u32 = 0x00CFD495;
pub const COLOR_BUTTON_BORDER: u32 = COLOR_MESSAGE_BORDER;
pub const COLOR_BUTTON_TEXT: u32 = 0x00000000;
pub const COLOR_BUTTON_SHADE: u32 = {
    let bytes = COLOR_BUTTON.to_be_bytes();
    u32::from_be_bytes([
        0, // always zero
        bytes[1] / 2,
        bytes[2] / 2,
        bytes[3] / 2,
    ])
};

// In pixels
pub const CELL_SIZE: usize = 32;
pub const CELL_SIZE_F: f32 = 32.0;

#[derive(Default)]
pub struct Config {
    pub lang: Lang,
    pub cell_cols: usize,
    pub cell_rows: usize,
    pub mine_count: usize,
    pub buffer_width: usize,
    pub buffer_height: usize,
}

impl Config {
    pub fn board_width(&self) -> usize {
        (CELL_SIZE + 1) * self.cell_cols
    }
    pub fn board_height(&self) -> usize {
        (CELL_SIZE + 1) * self.cell_rows
    }

    #[inline]
    pub fn cell_coords_to_idx(&self, x: usize, y: usize) -> usize {
        y * self.cell_cols + x
    }

    /// Converts pixel coords to cell coords.
    pub fn pos_to_cell(&self, (x, y): (usize, usize)) -> Option<(usize, usize)> {
        if x < self.board_width()
            && x % (CELL_SIZE + 1) != 0
            && y < self.board_height()
            && y % (CELL_SIZE + 1) != 0
        {
            Some((x / (CELL_SIZE + 1), y / (CELL_SIZE + 1)))
        } else {
            None
        }
    }
    pub fn pos_to_cell_f(&self, (x, y): (f32, f32)) -> Option<(usize, usize)> {
        // Truncate the floats
        self.pos_to_cell((x as usize, y as usize))
    }
}

pub enum Lang {
    En,
    Jp,
}

impl Default for Lang {
    fn default() -> Lang {
        Lang::En
    }
}

pub fn draw_rectangle(
    top_left: IVec2,
    size: IVec2,
    color: u32,
    buffer: &mut [u32],
    buffer_width: usize,
) {
    let left = top_left.x as usize;
    let top = top_left.y as usize;
    let width = size.x as usize;
    let height = size.y as usize;
    for y in top..top + height + 1 {
        let row_start_idx = y * buffer_width + left;
        let row_end_idx = row_start_idx + width;
        buffer[row_start_idx..row_end_idx + 1].fill(color);
    }
}

pub fn lerp_colors(min: u32, max: u32, amt: f32) -> u32 {
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
