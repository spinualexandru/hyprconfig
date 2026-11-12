//! Shared building blocks: page scaffolding, cards, pills, keycaps, tables, skeletons, a slider
//! and toasts. Everything reads colours and type from the kit.
//!
//! Each concern lives in its own file; everything is re-exported here so callers keep using
//! `ui::pill`, `ui::Card` and friends.

mod buttons;
mod card;
mod display;
mod layout;
mod skeleton;
mod slider;
mod style;
mod table;
mod toast;

pub use buttons::*;
pub use card::*;
pub use display::*;
pub use layout::*;
pub use skeleton::*;
pub use slider::*;
pub use style::*;
pub use table::*;
pub use toast::*;

pub const GUTTER: f32 = 24.0;
