use glib::Propagation;
use gtk4::{prelude::*, Box as GtkBox, Button, Entry, Label, Orientation, Switch};

pub fn plugins_page() -> GtkBox {
    let box_ = GtkBox::new(Orientation::Vertical, 12);
    box_.set_margin_top(18);
    box_.set_margin_bottom(18);
    box_.set_margin_start(18);
    box_.set_margin_end(18);

    box_.append(&Label::new(Some("Plugin Management:")));

    let plugins = hyprland::ctl::plugin::list();
    if let Ok(plugins) = plugins {
        for plugin in plugins.split("\n\n").filter(|p| !p.trim().is_empty()) {
            let row = GtkBox::new(Orientation::Horizontal, 8);
            let label = Label::new(Some(plugin.trim()));
            label.set_xalign(0.0);
            let unload_switch = Switch::new();
            unload_switch.set_active(true);
            unload_switch.set_vexpand(false);
            unload_switch.set_valign(gtk4::Align::Center);
            unload_switch.connect_state_set(move |switch, state| {
                println!("Toggling plugin");
                if state {
                    println!("Plugin enabled");
                } else {
                    println!("Plugin disabled");
                }
                // Return handler click id
                Propagation::Proceed
            });
            // Connect to unload logic here
            row.append(&label);
            row.append(&unload_switch);
            box_.append(&row);
        }
    } else if let Err(e) = plugins {
        box_.append(&Label::new(Some(&format!("Error fetching plugins: {}", e))));
    }

    let plugin_entry = Entry::new();
    plugin_entry.set_placeholder_text(Some("Enter plugin name to add..."));
    box_.append(&plugin_entry);

    let add_button = Button::with_label("Add Plugin");
    box_.append(&add_button);

    add_button.connect_clicked(move |_| {
        println!("Plugin added: {}", plugin_entry.text());
        plugin_entry.set_text("");
    });

    box_
}
