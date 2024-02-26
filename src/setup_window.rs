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

        mouse_pos: None,
        _mouse_was_oob: false,
        is_left_click_down: false,
        left_click_down_pos: None,

        font: Box::new(font_en),
        glyphs_cache: HashMap::new(),
        caret: IVec2::new(5, 5),
    };

    let mut first_loop = true;
    while state.window.is_open() && !state.window.is_key_down(Key::Escape) {
        let was_input = state.update_input();

        if was_input || first_loop {
            state.buffer.fill(shared::COLOR_MESSAGE_BOX);

            if button(&mut state, "English") {
                println!("button clicked");
            }
        }

        first_loop = false;

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

    /// The current mouse position. Present if there was a click (now or on the previous frame) and it was within the window.
    mouse_pos: Option<IVec2>,
    _mouse_was_oob: bool,
    /// Whether the mouse button is currently down.
    is_left_click_down: bool,
    /// The position of the mouse when the mouse button was first pressed down.
    left_click_down_pos: Option<IVec2>,

    font: Box<ab_glyph::PxScaleFont<&'a FontRef<'static>>>,
    glyphs_cache: HashMap<Box<str>, Vec<Glyph>>,
    /// Current position for drawing widgets
    caret: IVec2,
}

impl GuiState<'_> {
    /// Returns whether there was any input.
    fn update_input(&mut self) -> bool {
        let was_left_click_down = self.is_left_click_down;
        self.is_left_click_down = self.window.get_mouse_down(minifb::MouseButton::Left);
        let mut mouse_is_oob = false;
        // Skip getting the mouse position if there aren't clicks to handle.
        self.mouse_pos = if was_left_click_down || self.is_left_click_down {
            if let Some((x_f, y_f)) = self.window.get_mouse_pos(minifb::MouseMode::Discard) {
                Some(IVec2 {
                    x: x_f as i32,
                    y: y_f as i32,
                })
            } else {
                mouse_is_oob = true;
                None
            }
        } else {
            None
        };
        match (was_left_click_down, self.is_left_click_down) {
            // The mouse button is being held, keep the initial value
            (true, true) => {}
            // Keep the value around for the next frame, unless...
            (true, false) => {
                if self._mouse_was_oob {
                    // minifb doesn't notice mouse btn releases until it returns to the window; fix for
                    // https://github.com/emoon/rust_minifb/issues/345
                    self.left_click_down_pos = None;
                }
            }
            // Record initial click position
            (false, true) => self.left_click_down_pos = self.mouse_pos,
            // We're done with any value in it now
            (false, false) => self.left_click_down_pos = None,
        }
        self._mouse_was_oob = mouse_is_oob;

        was_left_click_down || self.is_left_click_down
    }

    /// Gets the current mouse position, cached.
    // TODO: remove if unneeded
    fn get_mouse_pos(&mut self) -> IVec2 {
        if let Some(pos) = self.mouse_pos {
            pos
        } else {
            let (x_f, y_f) = self
                .window
                .get_mouse_pos(minifb::MouseMode::Discard)
                .unwrap();
            let pos = IVec2 {
                x: x_f as i32,
                y: y_f as i32,
            };
            self.mouse_pos = Some(pos);
            pos
        }
    }
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
    let btn_pos_min = caret;
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

    let btn_bounds = (btn_pos_min, btn_pos_min + size);
    let mut clicked = false;
    if !state.is_left_click_down {
        if let Some(down_pos) = state.left_click_down_pos {
            let up_pos = state.mouse_pos.unwrap();
            if point_in_rect(down_pos, btn_bounds) && point_in_rect(up_pos, btn_bounds) {
                clicked = true;
            }
        }
    }
    clicked
    // TODO: change button appearance depending on clicking
}

fn point_in_rect(IVec2 { x, y }: IVec2, (min, max): (IVec2, IVec2)) -> bool {
    min.x <= x && x <= max.x && min.y <= y && y <= max.y
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
