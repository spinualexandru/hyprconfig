use std::process::Command;

use gtk4::gdk::Display as GdkDisplay;
use gtk4::{
    Box as GtkBox, ComboBoxText, CssProvider, Entry, Frame, Label, Orientation,
    STYLE_PROVIDER_PRIORITY_APPLICATION, ScrolledWindow, Separator, Switch, prelude::*,
};
use hyprland::data::Monitors;
use hyprland::shared::HyprData;

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

pub fn display_page() -> GtkBox {
    // Set up CSS for styling
    let css = "
    .monitor-frame {
        background: #f6f6f7;
        border-radius: 12px;
        padding: 18px;
        margin-bottom: 18px;
        box-shadow: 0 2px 8px rgba(0,0,0,0.04);
    }
    .monitor-title {
        font-weight: bold;
        font-size: 1.2em;
        margin-bottom: 6px;
    }
    .monitor-label {
        margin-bottom: 2px;
    }
    ";
    let provider = CssProvider::new();
    provider.load_from_data(css);
    gtk4::style_context_add_provider_for_display(
        &GdkDisplay::default().unwrap(),
        &provider,
        STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    let outer_box = GtkBox::new(Orientation::Vertical, 0);
    let scrolled = ScrolledWindow::builder()
        .min_content_height(400)
        .min_content_width(500)
        .vexpand(true)
        .hexpand(true)
        .build();
    let box_ = GtkBox::new(Orientation::Vertical, 18);
    box_.set_margin_top(18);
    box_.set_margin_bottom(18);
    box_.set_margin_start(18);
    box_.set_margin_end(18);
    box_.append(&Label::new(Some("Monitors:")));

    match Monitors::get() {
        Ok(monitors) => {
            let mut first = true;
            for monitor in monitors {
                if !first {
                    box_.append(&Separator::new(Orientation::Horizontal));
                }
                first = false;
                let monitor_box = GtkBox::new(Orientation::Vertical, 6);
                monitor_box.set_widget_name("monitor-box");
                // Monitor icon and name
                let title = format!(
                    "🖥️  <span class='monitor-title'>{}</span> <span size='small' foreground='#888'>({}x{} @ {}Hz)</span>",
                    monitor.name, monitor.width, monitor.height, monitor.refresh_rate
                );
                let title_label = Label::new(None);
                title_label.set_markup(&title);
                title_label.set_widget_name("monitor-title");
                monitor_box.append(&title_label);
                // Details
                let desc = Label::new(Some(&format!("Description: {}", monitor.description)));
                desc.set_widget_name("monitor-label");
                monitor_box.append(&desc);
                let pos = Label::new(Some(&format!("Position: {}x{}", monitor.x, monitor.y)));
                pos.set_widget_name("monitor-label");
                monitor_box.append(&pos);
                let scale = Label::new(Some(&format!("Scale: {}", monitor.scale)));
                scale.set_widget_name("monitor-label");
                monitor_box.append(&scale);
                // Frame for visual grouping
                let frame = Frame::new(None);
                frame.set_child(Some(&monitor_box));
                frame.set_widget_name("monitor-frame");
                box_.append(&frame);
            }
        }
        Err(e) => {
            box_.append(&Label::new(Some(&format!(
                "Failed to get monitor info: {}",
                e
            ))));
        }
    }
    scrolled.set_child(Some(&box_));
    outer_box.append(&scrolled);
    outer_box
}

pub fn network_page() -> GtkBox {
    let box_ = GtkBox::new(Orientation::Vertical, 12);
    box_.append(&Label::new(Some("Wi-Fi Enabled:")));
    let wifi_switch = Switch::new();
    box_.append(&wifi_switch);
    box_.append(&Label::new(Some("Current Network:")));
    let network_entry = Entry::new();
    network_entry.set_placeholder_text(Some("SSID"));
    // get network name from shell iw dev | grep ssid | awk -F 'ssid ' '{print $2}'
    let get_network_name = Command::new("nmcli")
        .arg("-t")
        .arg("-f")
        .arg("CONNECTION,STATE,DEVICE")
        .arg("dev")
        .output()
        .unwrap();
    let network_name_output = String::from_utf8(get_network_name.stdout).unwrap();
    let network_name = network_name_output.split(':').nth(0).unwrap();
    network_entry.set_text(network_name);
    box_.append(&network_entry);
    box_
}

pub fn about_page() -> GtkBox {
    let box_ = GtkBox::new(Orientation::Vertical, 12);
    box_.append(&Label::new(Some("HyprConfig Settings Manager")));
    box_.append(&Label::new(Some("Version 1.0.0")));
    box_.append(&Label::new(Some("For Hyprland window manager.")));
    box_
}
