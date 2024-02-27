use ab_glyph::{Font, FontRef, Glyph, PxScaleFont};
use glam::IVec2;
use minifb::{Key, Window};
use std::collections::HashMap;

use crate::shared::{self, Config, Lang};
use crate::text;

const WINDOW_WIDTH: usize = 200;
const WINDOW_HEIGHT: usize = 200;
const WINDOW_PADDING: i32 = 5;

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

        font: &font_en,
        font_en: &font_en,
        font_jp: &font_jp,
        glyphs_cache: HashMap::new(),
        caret: IVec2::splat(WINDOW_PADDING),
        caret_start: IVec2::splat(WINDOW_PADDING),
        line_height: 0,
        padding: IVec2::splat(WINDOW_PADDING),
    };

    let mut first_loop = true;
    while state.window.is_open() && !state.window.is_key_down(Key::Escape) {
        let was_input = state.update_input();

        if was_input || first_loop {
            state.buffer.fill(shared::COLOR_MESSAGE_BOX);
            state.caret = state.caret_start;

            let mut lang_btn = 0;
            if button_set(
                &mut state,
                &["English".into(), ("日本語", Lang::Jp).into()],
                &mut lang_btn,
            ) {
                println!("button {lang_btn} clicked");
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

type GuiFontRef<'a> = &'a PxScaleFont<&'a FontRef<'static>>;

struct GuiState<'f> {
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

    font: GuiFontRef<'f>,
    font_en: GuiFontRef<'f>,
    font_jp: GuiFontRef<'f>,
    glyphs_cache: HashMap<Box<str>, Vec<Glyph>>,
    /// Current position for drawing widgets
    caret: IVec2,
    /// Initial position of `caret`
    caret_start: IVec2,
    /// Height of the current line; gets added to `caret.y`
    line_height: i32,
    /// Space inserted between widgets
    padding: IVec2,
}

impl<'f> GuiState<'f> {
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

    fn new_line(&mut self) {
        self.caret.x = self.caret_start.x;
        self.caret.y += self.line_height + self.padding.y;
        self.line_height = 0;
    }

    #[inline]
    fn wrap_if_needed(&mut self, widget_size: IVec2) {
        if widget_size.y > self.line_height {
            self.line_height = widget_size.y;
        }
        if (self.caret.x + widget_size.x) as usize > self.buffer_width {
            self.new_line();
        }
    }

    #[inline]
    fn font_for(&self, str_in_lang: StrInLang) -> GuiFontRef<'f> {
        match str_in_lang.lang {
            None => self.font,
            Some(Lang::En) => self.font_en,
            Some(Lang::Jp) => self.font_jp,
        }
    }
}

#[derive(Copy, Clone)]
struct StrInLang<'a> {
    str: &'a str,
    lang: Option<Lang>,
}
impl<'a> From<&'a str> for StrInLang<'a> {
    fn from(str: &'a str) -> Self {
        Self { str, lang: None }
    }
}
impl<'a> From<(&'a str, Lang)> for StrInLang<'a> {
    fn from((str, lang): (&'a str, Lang)) -> Self {
        Self {
            str,
            lang: Some(lang),
        }
    }
}

const BORDER_SIZE: i32 = 2;
const BUTTON_PADDING_HORIZONTAL: i32 = 10;
const BUTTON_PADDING_VERTICAL: i32 = 10;

fn button(state: &mut GuiState, text: StrInLang) -> bool {
    let font = state.font_for(text);
    let glyphs = state.glyphs_cache.get_mut_or_create(text.str);
    let glyphs_bounds = text::layout_paragraph(
        font,
        ab_glyph::point(0.0, 0.0),
        f32::INFINITY,
        text.str,
        glyphs,
    );
    let glyphs_width = glyphs_bounds.width() as i32;
    let glyphs_height = glyphs_bounds.height() as i32;

    let inner_size = IVec2 {
        x: glyphs_width + BUTTON_PADDING_HORIZONTAL * 2,
        y: glyphs_height + BUTTON_PADDING_VERTICAL * 2,
    };
    let outer_size = inner_size + IVec2::splat(BORDER_SIZE * 2);
    state.wrap_if_needed(outer_size);

    let mut caret = state.caret;
    shared::draw_rectangle(
        caret,
        outer_size,
        shared::COLOR_BUTTON_BORDER,
        &mut state.buffer,
        state.buffer_width,
    );

    caret += IVec2::splat(BORDER_SIZE);
    let btn_bounds = (caret, caret + inner_size);
    let mut held = false;
    let mut clicked = false;
    // This can look slightly less stupid when let-chains are stabilized:
    // https://github.com/rust-lang/rust/issues/53667
    if let Some(down_pos) = state.left_click_down_pos {
        if point_in_rect(down_pos, btn_bounds) {
            if let Some(current_pos) = state.mouse_pos {
                if point_in_rect(current_pos, btn_bounds) {
                    if state.is_left_click_down {
                        held = true;
                    } else {
                        clicked = true;
                    }
                }
            }
        }
    }

    if held {
        shared::draw_rectangle(
            caret,
            inner_size,
            shared::COLOR_BUTTON_SHADE,
            &mut state.buffer,
            state.buffer_width,
        );
        shared::draw_rectangle(
            caret + IVec2::new(1, 1),
            inner_size - IVec2::new(2, 1),
            shared::COLOR_BUTTON,
            &mut state.buffer,
            state.buffer_width,
        );
        caret.y += 1;
    } else {
        shared::draw_rectangle(
            caret,
            inner_size,
            shared::COLOR_BUTTON,
            &mut state.buffer,
            state.buffer_width,
        );
    }
    caret += IVec2::new(BUTTON_PADDING_HORIZONTAL, BUTTON_PADDING_VERTICAL);

    text::draw_glyphs(
        // Get it anew so that `state` isn't retroactively mutably borrowed for the whole function
        state
            .glyphs_cache
            .get(text.str)
            .expect("we just generated glyphs for this text")
            .iter()
            .cloned(),
        caret,
        font,
        shared::COLOR_BUTTON_TEXT,
        &mut state.buffer,
        state.buffer_width,
    );

    state.caret.x += outer_size.x + state.padding.x;

    clicked
}

fn button_set(state: &mut GuiState, texts: &[StrInLang], active_button: &mut u8) -> bool {
    let mut clicked_any = false;
    for (i, &text) in texts.iter().enumerate() {
        if button(state, text) {
            *active_button = i.try_into().unwrap();
            clicked_any = true;
        }
    }
    clicked_any
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
