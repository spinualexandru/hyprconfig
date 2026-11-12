//! Loading placeholders.

use rune::{El, col, el, row};
use rune_kit::Kit;

/// A shimmer-free placeholder block.
pub fn skeleton(k: &Kit, w: Option<f32>, h: f32) -> El {
    let e = el().h(h).radius(6.0).fill(k.c.layer_03).a11y_hidden();
    match w {
        Some(w) => e.w(w),
        None => e.w(0.0).grow(1.0),
    }
}

/// Rows of placeholder blocks: each row lists fixed widths, `None` grows.
pub fn skeleton_rows(k: &Kit, rows: usize, shape: &[Option<f32>]) -> El {
    col()
        .key("skeleton")
        .w_full()
        .gap(16.0)
        .label("Loading")
        .children((0..rows).map(|i| {
            row()
                .key(format!("s{i}"))
                .w_full()
                .gap(16.0)
                .children(shape.iter().map(|w| skeleton(k, *w, 22.0)))
        }))
}
