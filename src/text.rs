use ab_glyph::{point, Font, Glyph, Rect, ScaleFont};
use glam::IVec2;

use crate::shared::lerp_colors;

pub fn draw_glyphs<F, FS, G>(
    glyphs: G,
    IVec2 {
        x: left_margin,
        y: top_margin,
    }: IVec2,
    font: &FS,
    color: u32,
    buffer: &mut [u32],
    buffer_width: usize,
) where
    F: Font,
    FS: ScaleFont<F>,
    G: Iterator<Item = Glyph>,
{
    for glyph in glyphs {
        if let Some(outlined) = font.outline_glyph(glyph) {
            let bounds = outlined.px_bounds();
            let offset_x = left_margin + bounds.min.x as i32;
            let offset_y = top_margin + bounds.min.y as i32;
            outlined.draw(|x, y, c| {
                let x = x as i32 + offset_x;
                let y = y as i32 + offset_y;
                let i = y as usize * buffer_width + x as usize;
                buffer[i] = lerp_colors(buffer[i], color, c);
            });
        }
    }
}

/// Returns the size of the resulting paragraph, were it to be drawn.
///
/// Use `f32::INFINITY` for `max_width` for none.
// Based off an example in the ab_glyph documentation.
pub fn layout_paragraph<F, FS>(
    font: &FS,
    max_width: f32,
    text: &str,
    target: &mut Vec<Glyph>,
) -> IVec2
where
    F: Font,
    FS: ScaleFont<F>,
{
    let mut glyphs_bounds = Rect::default();
    let v_advance = font.height() + font.line_gap();
    let mut caret = point(0.0, font.ascent());
    let mut prev_glyph: Option<Glyph> = None;
    for c in text.chars() {
        if c.is_control() {
            if c == '\n' {
                caret = point(0.0, caret.y + v_advance);
                prev_glyph = None;
            }
            continue;
        }

        let mut glyph = font.scaled_glyph(c);
        if let Some(prev_glyph) = prev_glyph {
            caret.x += font.kern(prev_glyph.id, glyph.id);
        }
        glyph.position = caret;

        prev_glyph = Some(glyph.clone());
        caret.x += font.h_advance(glyph.id);

        if !c.is_whitespace() {
            // Whitespace is allowed to overflow max_width since it's not visible
            // anyway *and* we don't want to start the next line with it.
            if caret.x > max_width {
                caret = point(0.0, caret.y + v_advance);
                glyph.position = caret;
                prev_glyph = None;
            }
            // Don't count trailing whitespace in width.
            if caret.x > glyphs_bounds.max.x {
                glyphs_bounds.max.x = caret.x;
            }
        }
        target.push(glyph);
    }

    glyphs_bounds.max.y = caret.y - font.descent();

    IVec2 {
        x: glyphs_bounds.width() as i32,
        y: glyphs_bounds.height() as i32,
    }
}
