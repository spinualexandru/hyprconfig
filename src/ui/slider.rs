//! A horizontal value slider with drag, keyboard steps and Escape-to-cancel.

use rune::{Color, Drag, El, Phase, Tx, el, row};
use rune_kit::Kit;

use crate::app::State;

/// A horizontal slider over `0..=max` that fills its row. `on_change` runs as it drags (the
/// value lands at once). Escape during a drag puts the old value back.
#[allow(clippy::too_many_arguments)]
pub fn slider(
    k: &Kit,
    key: &str,
    label: &str,
    value: f32,
    max: f32,
    step: f32,
    disabled: bool,
    on_change: impl Fn(&mut State, f32, &mut Tx) + Clone + 'static,
) -> El {
    const KNOB: f32 = 18.0;
    const BAR: f32 = 4.0;
    let v = (value / max).clamp(0.0, 1.0);
    let to_value =
        move |x: f32, w: f32| ((x - KNOB / 2.0) / (w - KNOB).max(1.0)).clamp(0.0, 1.0) * max;
    let accent = if disabled {
        k.c.border_strong
    } else {
        k.c.bg_accent
    };
    let mut s = row()
        .key(key)
        .role("slider")
        .label(label.to_owned())
        .range_value(f64::from(value), 0.0, f64::from(max))
        .range_step(f64::from(step))
        .h(22.0)
        .w(0.0)
        .grow(1.0)
        .center_y()
        .opacity(if disabled { 0.5 } else { 1.0 })
        .child(
            el().key("filled")
                .w(0.0)
                .grow(v)
                .h(BAR)
                .radius(BAR / 2.0)
                .fill(accent)
                .a11y_hidden(),
        )
        .child(
            el().key("knob")
                .size(KNOB, KNOB)
                .radius(KNOB / 2.0)
                .fill(k.c.knob)
                .border(1.0, k.c.border_base)
                .shadow(1.0, 3.0, Color::BLACK.alpha(0.18))
                .a11y_hidden(),
        )
        .child(
            el().key("rest")
                .w(0.0)
                .grow(1.0 - v)
                .h(BAR)
                .radius(BAR / 2.0)
                .fill(k.c.layer_03)
                .a11y_hidden(),
        );
    if !disabled {
        let step_change = on_change.clone();
        s = s
            .focusable(true)
            .on_step(move |st: &mut State, steps, tx| {
                let next = (value + steps as f32 * step).clamp(0.0, max);
                step_change(st, next, tx);
            })
            .on_drag(move |st: &mut State, d: &mut Drag, tx| match d.phase {
                Phase::Down => {
                    d.data = value;
                    on_change(st, to_value(d.x, d.w), tx);
                    tx.instant();
                }
                Phase::Move => {
                    on_change(st, to_value(d.x, d.w), tx);
                    tx.instant();
                }
                Phase::Up => {
                    if d.cancelled() {
                        on_change(st, d.data, tx);
                    }
                }
            });
    }
    s
}
