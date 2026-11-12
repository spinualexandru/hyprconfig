//! Tables: a header row over bordered rows with fixed or shared column widths.

use rune::{El, col, el, row, text};
use rune_kit::Kit;

/// A table: a header row, then rows. `widths` are fixed points or `None` to share the rest.
pub fn table(k: &Kit, key: &str, headers: &[(&str, Option<f32>)], rows: Vec<El>) -> El {
    let head = row()
        .key("thead")
        .w_full()
        .h(36.0)
        .px(12.0)
        .gap(12.0)
        .center_y()
        .fill(k.c.layer_01)
        .children(headers.iter().map(|(h, w)| {
            cell(
                text((*h).to_owned())
                    .font(k.ty.text_small_medium)
                    .color(k.c.text_muted),
                *w,
            )
        }));
    col()
        .key(key)
        .role("table")
        .w_full()
        .radius(8.0)
        .border(1.0, k.c.border_weak)
        .clip_rounded(8.0)
        .child(head)
        .children(rows.into_iter().map(|r| {
            col()
                .w_full()
                .child(el().w_full().h(1.0).fill(k.c.border_weak).a11y_hidden())
                .child(r)
        }))
}

/// A table row's cells laid out like its header.
pub fn table_row(k: &Kit, key: impl Into<String>, widths: &[Option<f32>], cells: Vec<El>) -> El {
    row()
        .key(key.into())
        .role("row")
        .w_full()
        .min_h(48.0)
        .px(12.0)
        .py(6.0)
        .gap(12.0)
        .center_y()
        .hover(k.c.hover)
        .children(
            cells
                .into_iter()
                .zip(widths.iter())
                .map(|(c, w)| cell(c, *w)),
        )
}

fn cell(content: El, w: Option<f32>) -> El {
    let c = row().min_w(0.0).center_y().child(content);
    match w {
        Some(w) => c.w(w).shrink(),
        None => c.w(0.0).grow(1.0),
    }
}
