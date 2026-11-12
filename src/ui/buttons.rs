//! Preconfigured buttons pages share.

use rune::{Icon, Tx};
use rune_kit::{Button, ButtonVariant, IconButton, IconButtonVariant};

use crate::app::State;
use crate::icons;

/// The refresh button pages share; it spins nothing, it disables while loading.
pub fn refresh_button(loading: bool, f: impl Fn(&mut State, &mut Tx) + 'static) -> Button {
    Button::new(if loading { "Refreshing…" } else { "Refresh" })
        .icon(&icons::REFRESH_CW)
        .variant(ButtonVariant::Outline)
        .disabled(loading)
        .on_press_tx(f)
}

pub fn icon_button(i: &'static Icon, label: &str) -> IconButton {
    IconButton::new(i)
        .variant(IconButtonVariant::Ghost)
        .label(label)
        .help(label)
}
