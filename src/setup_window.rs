use ab_glyph::{Font, FontRef, Glyph, PxScaleFont};
use glam::IVec2;
use minifb::{Key, Window};
use std::collections::HashMap;

use crate::shared::{self, Config, Lang};
use crate::text;

const WINDOW_WIDTH: usize = 300;
const WINDOW_HEIGHT: usize = 200;
const WINDOW_PADDING: i32 = 5;

pub fn run() -> Config {
    let font_en = FontRef::try_from_slice(shared::FIRA_CODE_BYTES).unwrap();
    let font_en = font_en.as_scaled(20.0);
    let font_jp = FontRef::try_from_slice(shared::NOTO_SANS_JP_BYTES).unwrap();
    let font_jp = font_jp.as_scaled(20.0);

    let mut lang = Lang::En;

    let mut gui = GuiState {
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

    let mut needs_update = true;
    while gui.window.is_open() && !gui.window.is_key_down(Key::Escape) {
        let was_input = gui.update_input();

        if was_input || needs_update {
            needs_update = false;
            gui.buffer.fill(shared::COLOR_MESSAGE_BOX);
            gui.caret = gui.caret_start;

            gui.label(lang.en_jp("Language:", "言語："));

            let mut lang_btn = 0;
            if gui.button_set(
                ["English".of(Lang::En), "日本語".of(Lang::Jp)],
                &mut lang_btn,
            ) {
                println!("button {lang_btn} clicked");
                lang = [Lang::En, Lang::Jp][usize::from(lang_btn)];
                gui.font = [gui.font_en, gui.font_jp][usize::from(lang_btn)];
                needs_update = true;
            }
        }

        gui.window
            .update_with_buffer(&gui.buffer, WINDOW_WIDTH, WINDOW_HEIGHT)
            .unwrap();
    }

    Config {
        lang,
        cell_cols: 10,
        cell_rows: 10,
        mine_count: 10,
        ..Config::default()
    }
}

const BORDER_SIZE: i32 = 2;
const BUTTON_PADDING_HORIZONTAL: i32 = 10;
const BUTTON_PADDING_VERTICAL: i32 = 10;

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

    // Widgets

    fn label<'a>(&mut self, text: impl Into<StrInLang<'a>>) {
        let text = text.into();
        let glyphs_size = self._cache_glyphs(text);

        let inner_size = glyphs_size
            + IVec2 {
                x: 0,
                y: BUTTON_PADDING_VERTICAL * 2,
            };
        let outer_size = inner_size + IVec2::splat(BORDER_SIZE * 2);
        self.wrap_if_needed(outer_size);

        self._draw_cached_glyphs_at(
            text,
            self.caret
                + IVec2 {
                    x: 0,
                    y: BORDER_SIZE + BUTTON_PADDING_VERTICAL,
                },
            shared::COLOR_BUTTON_TEXT,
        );

        self.caret.x += outer_size.x + self.padding.x;
    }

    fn button<'a>(&mut self, text: impl Into<StrInLang<'a>>) -> bool {
        let text = text.into();
        let glyphs_size = self._cache_glyphs(text);

        let inner_size = glyphs_size
            + IVec2 {
                x: BUTTON_PADDING_HORIZONTAL * 2,
                y: BUTTON_PADDING_VERTICAL * 2,
            };
        let outer_size = inner_size + IVec2::splat(BORDER_SIZE * 2);
        self.wrap_if_needed(outer_size);

        let mut caret = self.caret;
        shared::draw_rectangle(
            caret,
            outer_size,
            shared::COLOR_BUTTON_BORDER,
            &mut self.buffer,
            self.buffer_width,
        );

        caret += IVec2::splat(BORDER_SIZE);
        let btn_bounds = (caret, caret + inner_size);
        let mut held = false;
        let mut clicked = false;
        // This can look slightly less stupid when let-chains are stabilized:
        // https://github.com/rust-lang/rust/issues/53667
        if let Some(down_pos) = self.left_click_down_pos {
            if point_in_rect(down_pos, btn_bounds) {
                if let Some(current_pos) = self.mouse_pos {
                    if point_in_rect(current_pos, btn_bounds) {
                        if self.is_left_click_down {
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
                &mut self.buffer,
                self.buffer_width,
            );
            shared::draw_rectangle(
                caret + IVec2::new(1, 1),
                inner_size - IVec2::new(2, 1),
                shared::COLOR_BUTTON,
                &mut self.buffer,
                self.buffer_width,
            );
            caret.y += 1;
        } else {
            shared::draw_rectangle(
                caret,
                inner_size,
                shared::COLOR_BUTTON,
                &mut self.buffer,
                self.buffer_width,
            );
        }
        caret += IVec2::new(BUTTON_PADDING_HORIZONTAL, BUTTON_PADDING_VERTICAL);

        self._draw_cached_glyphs_at(text, caret, shared::COLOR_BUTTON_TEXT);

        self.caret.x += outer_size.x + self.padding.x;

        clicked
    }

    fn button_set<'a, const N: usize>(
        &mut self,
        texts: [impl Into<StrInLang<'a>>; N],
        active_button: &mut u8,
    ) -> bool {
        let mut clicked_any = false;
        for (i, text) in texts.into_iter().enumerate() {
            if self.button(text) {
                *active_button = i.try_into().unwrap();
                clicked_any = true;
            }
        }
        clicked_any
    }

    fn _cache_glyphs(&mut self, text: StrInLang<'_>) -> IVec2 {
        let font = self.font_for(text);
        let glyphs = self.glyphs_cache.get_mut_or_create(text.str);
        let glyphs_bounds = text::layout_paragraph(
            font,
            ab_glyph::point(0.0, 0.0),
            f32::INFINITY,
            text.str,
            glyphs,
        );
        IVec2 {
            x: glyphs_bounds.width() as i32,
            y: glyphs_bounds.height() as i32,
        }
    }

    fn _draw_cached_glyphs_at(&mut self, text: StrInLang<'_>, caret: IVec2, color: u32) {
        text::draw_glyphs(
            self.glyphs_cache
                .get(text.str)
                .expect("glyphs weren't cached before trying to draw them")
                .iter()
                .cloned(),
            caret,
            self.font_for(text),
            color,
            &mut self.buffer,
            self.buffer_width,
        );
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
trait StrInLangExt<'a> {
    fn of(self, lang: Lang) -> StrInLang<'a>;
}
impl<'a> StrInLangExt<'a> for &'a str {
    fn of(self, lang: Lang) -> StrInLang<'a> {
        StrInLang {
            str: self,
            lang: Some(lang),
        }
    }
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
