use ab_glyph::{Font, FontRef, Glyph, ScaleFont};
use glam::IVec2;
use minifb::{Key, Window};
use std::collections::HashMap;

use crate::shared::{self, Config, Lang};
use crate::text;

const WINDOW_WIDTH: usize = 200;
const WINDOW_HEIGHT: usize = 200;

pub fn run() -> Config {
    let font_en = FontRef::try_from_slice(shared::FIRA_CODE_BYTES).unwrap();
    let font_en = font_en.as_scaled(20.0);
    let font_jp = FontRef::try_from_slice(shared::NOTO_SANS_JP_BYTES).unwrap();
    let font_jp = font_jp.as_scaled(20.0);

    let mut lang = Lang::En;

    let mut state = GuiState {
        window: Window::new(
            "Minesweeper - Setup",
            WINDOW_WIDTH,
            WINDOW_HEIGHT,
            Default::default(),
        )
        .unwrap(),
        buffer: vec![0u32; WINDOW_WIDTH * WINDOW_HEIGHT],
        buffer_width: WINDOW_WIDTH,
        font: Box::new(font_en),
        glyphs_cache: HashMap::new(),
        caret: IVec2::new(5, 5),
    };

    while state.window.is_open() && !state.window.is_key_down(Key::Escape) {
        state.buffer.fill(shared::COLOR_MESSAGE_BOX);

        /*shared::draw_rectangle(
            IVec2::new(5, 5),
            IVec2::new(100, state.font.scale().y as i32 + 10),
            shared::COLOR_BUTTON,
            &mut state.buffer,
            state.buffer_width,
        );
        let mut btn_glyphs = Vec::new();
        _ = text::layout_paragraph(
            state.font.as_ref(),
            ab_glyph::point(10.0, 10.0),
            f32::INFINITY,
            "English",
            &mut btn_glyphs,
        );
        text::draw_glyphs(
            btn_glyphs.into_iter(),
            (0, 0),
            state.font.as_ref(),
            shared::COLOR_BUTTON_TEXT,
            &mut state.buffer,
            state.buffer_width,
        );*/

        button(&mut state, "English");

        state
            .window
            .update_with_buffer(&state.buffer, WINDOW_WIDTH, WINDOW_HEIGHT)
            .unwrap();
    }

    Config::default()
}

struct GuiState<'a> {
    window: minifb::Window,
    buffer: Vec<u32>,
    buffer_width: usize,
    font: Box<ab_glyph::PxScaleFont<&'a FontRef<'static>>>,
    glyphs_cache: HashMap<Box<str>, Vec<Glyph>>,
    caret: IVec2,
}

const BORDER_SIZE: i32 = 2;
const BUTTON_PADDING_HORIZONTAL: i32 = 10;
const BUTTON_PADDING_VERTICAL: i32 = 10;

fn button(state: &mut GuiState, text: &str) -> bool {
    let glyphs = state.glyphs_cache.get_mut_or_create(text);
    let glyphs_bounds = text::layout_paragraph(
        state.font.as_ref(),
        ab_glyph::point(0.0, 0.0),
        f32::INFINITY,
        text,
        glyphs,
    );
    let glyphs_width = glyphs_bounds.width() as i32;
    let glyphs_height = glyphs_bounds.height() as i32;
    let size = IVec2 {
        x: glyphs_width + BUTTON_PADDING_HORIZONTAL * 2,
        y: glyphs_height + BUTTON_PADDING_VERTICAL * 2,
    };
    let mut caret = state.caret;
    shared::draw_rectangle(
        caret,
        size + IVec2::splat(BORDER_SIZE * 2),
        shared::COLOR_BUTTON_BORDER,
        &mut state.buffer,
        state.buffer_width,
    );
    caret += IVec2::splat(BORDER_SIZE);
    shared::draw_rectangle(
        caret,
        size,
        shared::COLOR_BUTTON,
        &mut state.buffer,
        state.buffer_width,
    );
    caret += IVec2::new(BUTTON_PADDING_HORIZONTAL, BUTTON_PADDING_VERTICAL);
    text::draw_glyphs(
        glyphs.iter().cloned(),
        caret,
        state.font.as_ref(),
        shared::COLOR_BUTTON_TEXT,
        &mut state.buffer,
        state.buffer_width,
    );
    // TODO: check for click and return it
    // TODO: change button appearance depending on clicking
    false
}

trait StrHashMap {
    /// Returns a mutable reference to the value, inserting its default value
    /// if necessary, and cloning the key if so.
    fn get_mut_or_create(&mut self, key: &str) -> &mut Vec<Glyph>;
}
impl StrHashMap for HashMap<Box<str>, Vec<Glyph>> {
    fn get_mut_or_create(&mut self, key: &str) -> &mut Vec<Glyph> {
        if !self.contains_key(key) {
            self.insert(key.into(), Vec::new());
        }
        self.get_mut(key).unwrap()
    }
}
