//! Text styles shared across pages.

use rune::TextStyle;
use rune_kit::Kit;

pub fn title_style() -> TextStyle {
    TextStyle::new(26.0, 700.0).lh(32.0)
}

pub fn section_style() -> TextStyle {
    TextStyle::new(17.0, 600.0).lh(24.0)
}

pub fn card_title_style() -> TextStyle {
    TextStyle::new(15.0, 600.0).lh(20.0)
}

pub fn mono(k: &Kit) -> TextStyle {
    TextStyle {
        size: 12.5,
        line_height: 18.0,
        ..k.ty.mono
    }
}
