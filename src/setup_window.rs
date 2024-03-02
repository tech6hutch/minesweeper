use ab_glyph::{Font, FontRef, Glyph, PxScaleFont};
use glam::IVec2;
use minifb::{Key, Window};
use std::collections::HashMap;

use crate::{game_window, shared, text};
use shared::{Config, Lang};

const SAFE_CELLS_FOR_FIRST_CLICK: i32 = game_window::SAFE_CELLS_FOR_FIRST_CLICK as i32;

// 4 is the minimum that doesn't crash :)
const MIN_COLS: i32 = 4;
const MIN_ROWS: i32 = 4;
const MAX_COLS: i32 = 99;
const MAX_ROWS: i32 = 99;

const WINDOW_WIDTH: usize = 300;
const WINDOW_HEIGHT: usize = 300;
const WINDOW_PADDING: i32 = 5;

pub fn run(old_cfg: Config) -> Option<Config> {
    let font_en = FontRef::try_from_slice(shared::FIRA_CODE_BYTES).unwrap();
    let font_en = font_en.as_scaled(20.0);
    let font_jp = FontRef::try_from_slice(shared::NOTO_SANS_JP_BYTES).unwrap();
    let font_jp = font_jp.as_scaled(20.0);

    let mut lang = old_cfg.lang;
    let mut rows: i32 = old_cfg.cell_rows as i32;
    let mut cols: i32 = old_cfg.cell_cols as i32;
    let mut mine_count: i32 = old_cfg.mine_count as i32;

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

        font: lang.en_jp(&font_en, &font_jp),
        font_en: &font_en,
        font_jp: &font_jp,
        caret: IVec2::splat(WINDOW_PADDING),
        caret_start: IVec2::splat(WINDOW_PADDING),
        line_height: 0,
        padding: IVec2::splat(WINDOW_PADDING),
    };
    let mut prev_buffer = gui.buffer.clone();

    let mut start_game = false;
    let mut needs_update = true;
    'window_loop: while gui.window.is_open() && !gui.window.is_key_down(Key::Escape) {
        let was_input = gui.update_input();
        needs_update |= was_input;

        // Break from this block to cancel the draw update, but get new input
        // info (redrawing the previous, complete buffer).
        'update_buffer: {
            if !needs_update {
                break 'update_buffer;
            }

            gui.buffer.fill(shared::COLOR_MESSAGE_BOX);
            gui.caret = gui.caret_start;

            gui.label(lang.en_jp("Language:", "言語："));
            let mut lang_btn = 0;
            if gui.button_set(
                ["English".of(Lang::En), "日本語".of(Lang::Jp)],
                &mut lang_btn,
            ) {
                lang = [Lang::En, Lang::Jp][usize::from(lang_btn)];
                gui.font = [gui.font_en, gui.font_jp][usize::from(lang_btn)];
                break 'update_buffer;
            }
            gui.new_line();

            gui.label(lang.en_jp("Columns:", "筋："));
            if gui.number_input(&mut cols) {
                cols = cols.clamp(MIN_COLS, MAX_COLS);
                mine_count = mine_count.clamp(0, rows * cols - SAFE_CELLS_FOR_FIRST_CLICK);
                break 'update_buffer;
            }
            gui.new_line();

            gui.label(lang.en_jp("Rows:", "段："));
            if gui.number_input(&mut rows) {
                rows = rows.clamp(MIN_ROWS, MAX_ROWS);
                mine_count = mine_count.clamp(0, rows * cols - SAFE_CELLS_FOR_FIRST_CLICK);
                break 'update_buffer;
            }
            gui.new_line();

            gui.label(lang.en_jp("Mines:", "地雷："));
            if gui.number_input(&mut mine_count) {
                mine_count = mine_count.clamp(0, rows * cols - SAFE_CELLS_FOR_FIRST_CLICK);
                break 'update_buffer;
            }
            gui.new_line();

            if gui.button(lang.en_jp("Start Game", "プレイ")) {
                start_game = true;
                break 'window_loop;
            }

            prev_buffer.copy_from_slice(&gui.buffer);
            needs_update = false;
        }

        gui.window
            .update_with_buffer(&prev_buffer, WINDOW_WIDTH, WINDOW_HEIGHT)
            .unwrap();
    }

    if !start_game {
        return None;
    }

    // The window implicitly closes.

    Some(Config {
        lang,
        cell_cols: cols.try_into().unwrap(),
        cell_rows: rows.try_into().unwrap(),
        mine_count: mine_count.try_into().unwrap(),
        ..Config::default()
    })
}

const BORDER_SIZE: i32 = 2;
const BUTTON_PADDING_HORIZONTAL: i32 = 8;
const BUTTON_PADDING_VERTICAL: i32 = 5;

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

    /// Draws a label.
    fn label<'a>(&mut self, text: impl Into<StrInLang<'a>>) {
        let text = text.into();
        let font = self.font_for(text);
        let mut glyphs = Vec::new();
        let glyphs_size = text::layout_paragraph(font, f32::INFINITY, text.str, &mut glyphs);

        let inner_size = glyphs_size
            + IVec2 {
                x: 0,
                y: BUTTON_PADDING_VERTICAL * 2,
            };
        let outer_size = inner_size + IVec2::splat(BORDER_SIZE * 2);
        self.wrap_if_needed(outer_size);

        self._draw_glyphs_at(
            glyphs,
            font,
            self.caret
                + IVec2 {
                    x: 0,
                    y: BORDER_SIZE + BUTTON_PADDING_VERTICAL,
                },
            shared::COLOR_BUTTON_TEXT,
        );

        self.caret.x += outer_size.x + self.padding.x;
    }

    /// Draws/handles a number input. Returns true if the value was (attempted to be) changed.
    fn number_input(&mut self, num: &mut i32) -> bool {
        if self.button("-") {
            *num = num.saturating_sub(1);
            return true;
        }

        {
            let string = num.to_string();
            let text = string.as_str().into();
            let font = self.font_for(text);
            let mut glyphs = Vec::new();
            let glyphs_size = text::layout_paragraph(font, f32::INFINITY, text.str, &mut glyphs);

            let inner_size = glyphs_size
                + IVec2 {
                    x: BUTTON_PADDING_HORIZONTAL * 2,
                    y: BUTTON_PADDING_VERTICAL * 2,
                };
            let outer_size = inner_size + IVec2::splat(BORDER_SIZE * 2);
            self.wrap_if_needed(outer_size);
            let mut caret = self.caret;

            // Draw the outline
            let bg_color =
                self.buffer[self.caret.y as usize * self.buffer_width + self.caret.x as usize];
            shared::draw_rectangle(
                caret,
                outer_size,
                shared::COLOR_BUTTON_TEXT,
                &mut self.buffer,
                self.buffer_width,
            );
            caret += IVec2::splat(BORDER_SIZE);
            shared::draw_rectangle(
                caret,
                inner_size,
                bg_color,
                &mut self.buffer,
                self.buffer_width,
            );
            caret += IVec2::new(BUTTON_PADDING_HORIZONTAL, BUTTON_PADDING_VERTICAL);

            self._draw_glyphs_at(glyphs, font, caret, shared::COLOR_BUTTON_TEXT);

            self.caret.x += outer_size.x + self.padding.x;
        }

        if self.button("+") {
            *num = num.saturating_add(1);
            return true;
        }

        false
    }

    /// Draws/handles a button. Returns true if it was clicked.
    fn button<'a>(&mut self, text: impl Into<StrInLang<'a>>) -> bool {
        let text = text.into();
        let font = self.font_for(text);
        let mut glyphs = Vec::new();
        let glyphs_size = text::layout_paragraph(font, f32::INFINITY, text.str, &mut glyphs);

        let inner_size = glyphs_size
            + IVec2 {
                x: BUTTON_PADDING_HORIZONTAL * 2,
                y: BUTTON_PADDING_VERTICAL * 2,
            };
        let outer_size = inner_size + IVec2::splat(BORDER_SIZE * 2);
        self.wrap_if_needed(outer_size);

        // Draw the outline
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

        // Draw the inside of the button
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

        self._draw_glyphs_at(glyphs, font, caret, shared::COLOR_BUTTON_TEXT);

        self.caret.x += outer_size.x + self.padding.x;

        clicked
    }

    /// Draws/handles a set of related buttons where one is always active. Returns true if any were clicked.
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

    fn _draw_glyphs_at(&mut self, glyphs: Vec<Glyph>, font: GuiFontRef, caret: IVec2, color: u32) {
        text::draw_glyphs(
            glyphs.into_iter(),
            caret,
            font,
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
