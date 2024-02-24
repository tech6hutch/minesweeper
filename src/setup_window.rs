use ab_glyph::{point, Font, FontRef, ScaleFont};
use minifb::{Key, Window};

use crate::shared::*;
use crate::text;

const WINDOW_WIDTH: usize = 200;
const WINDOW_HEIGHT: usize = 200;

pub fn run() -> Config {
    let mut buffer = vec![0u32; WINDOW_WIDTH * WINDOW_HEIGHT];
    let mut window = Window::new(
        "Minesweeper - Setup",
        WINDOW_WIDTH,
        WINDOW_HEIGHT,
        Default::default(),
    )
    .unwrap();

    let font_en = FontRef::try_from_slice(FIRA_CODE_BYTES).unwrap();
    let font_jp = FontRef::try_from_slice(NOTO_SANS_JP_BYTES).unwrap();
    let mut font = font_en.as_scaled(20.0);

    let mut lang = Lang::En;

    while window.is_open() && !window.is_key_down(Key::Escape) {
        buffer.fill(COLOR_MESSAGE_BOX);

        draw_rectangle(
            (5, 5),
            (100, font.scale().y as usize + 10),
            COLOR_BUTTON,
            &mut buffer,
            WINDOW_WIDTH,
        );
        let mut btn_glyphs = Vec::new();
        _ = text::layout_paragraph(
            &font,
            point(10.0, 10.0),
            f32::INFINITY,
            "English",
            &mut btn_glyphs,
        );
        text::draw_glyphs(
            btn_glyphs.into_iter(),
            (0, 0),
            &font,
            COLOR_BUTTON_TEXT,
            &mut buffer,
            WINDOW_WIDTH,
        );

        window
            .update_with_buffer(&buffer, WINDOW_WIDTH, WINDOW_HEIGHT)
            .unwrap();
    }

    Config::default()
}
