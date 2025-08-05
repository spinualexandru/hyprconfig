use gtk4::{prelude::*, Box as GtkBox, Entry, Label, Orientation, Switch};

pub fn keyboard_page() -> GtkBox {
    let box_ = GtkBox::new(Orientation::Vertical, 12);
    box_.set_margin_top(18);
    box_.set_margin_bottom(18);
    box_.set_margin_start(18);
    box_.set_margin_end(18);
    let key_repeat_box: GtkBox = GtkBox::new(Orientation::Horizontal, 12);
    key_repeat_box.append(&Label::new(Some("Key Repeat Rate:")));
    let repeat_entry = Entry::new();
    repeat_entry.set_placeholder_text(Some("e.g. 30 ms"));
    key_repeat_box.append(&repeat_entry);
    box_.append(&key_repeat_box);

    let switch_box: GtkBox = GtkBox::new(Orientation::Horizontal, 12);
    switch_box.append(&Label::new(Some("Enable Key Repeat:")));
    let repeat_switch = Switch::new();
    switch_box.append(&repeat_switch);
    box_.append(&switch_box);
    box_.append(&repeat_entry);

    box_
}
