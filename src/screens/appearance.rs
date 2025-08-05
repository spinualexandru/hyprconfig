use gtk4::{prelude::*, Box as GtkBox, ComboBoxText, Entry, Label, Orientation};

pub fn appearance_page() -> GtkBox {
    let box_ = GtkBox::new(Orientation::Vertical, 12);
    box_.set_margin_top(18);
    box_.set_margin_bottom(18);
    box_.set_margin_start(18);
    box_.set_margin_end(18);
    box_.append(&Label::new(Some("Theme:")));
    let theme_combo = ComboBoxText::new();
    theme_combo.append_text("Light");
    theme_combo.append_text("Dark");
    theme_combo.append_text("System");
    box_.append(&theme_combo);
    box_.append(&Label::new(Some("Accent Color:")));
    let accent_entry = Entry::new();
    accent_entry.set_placeholder_text(Some("#RRGGBB"));
    box_.append(&accent_entry);
    box_
}
