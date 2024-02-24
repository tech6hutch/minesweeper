use ab_glyph::{point, Font, Glyph, Point, Rect, ScaleFont};

// Based off an example in the ab_glyph documentation.
pub fn layout_paragraph<F, FS>(
    font: &FS,
    position: Point,
    max_width: f32,
    text: &str,
    target: &mut Vec<Glyph>,
) -> Rect
where
    F: Font,
    FS: ScaleFont<F>,
{
    let mut glyphs_bounds = Rect {
        min: position,
        max: position,
    };
    let v_advance = font.height() + font.line_gap();
    let mut caret = position + point(0.0, font.ascent());
    let mut prev_glyph: Option<Glyph> = None;
    for c in text.chars() {
        if c.is_control() {
            if c == '\n' {
                caret = point(position.x, caret.y + v_advance);
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
            if caret.x > position.x + max_width {
                caret = point(position.x, caret.y + v_advance);
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

    glyphs_bounds
}
