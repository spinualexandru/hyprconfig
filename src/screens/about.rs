use gtk4::{prelude::*, Box as GtkBox, Label, Orientation};

pub fn about_page() -> GtkBox {
    let box_ = GtkBox::new(Orientation::Vertical, 12);
    box_.append(&Label::new(Some("HyprConfig Settings Manager")));
    box_.append(&Label::new(Some("Version 1.0.0")));
    box_.append(&Label::new(Some("For Hyprland window manager.")));
    box_
}
