use gtk4::gdk::Display as GdkDisplay;
use gtk4::{prelude::*, Box as GtkBox, CssProvider, Frame, Label, Orientation, ScrolledWindow, Separator, STYLE_PROVIDER_PRIORITY_APPLICATION};
use hyprland::data::Monitors;
use hyprland::shared::HyprData;

pub fn display_page() -> GtkBox {
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
                let title = format!(
                    "🖥️  <span class='monitor-title'>{}</span> <span size='small' foreground='#888'>({}x{} @ {}Hz)</span>",
                    monitor.name, monitor.width, monitor.height, monitor.refresh_rate
                );
                let title_label = Label::new(None);
                title_label.set_markup(&title);
                title_label.set_widget_name("monitor-title");
                monitor_box.append(&title_label);
                let desc = Label::new(Some(&format!("Description: {}", monitor.description)));
                desc.set_widget_name("monitor-label");
                monitor_box.append(&desc);
                let resolution = Label::new(Some(&format!(
                    "{}x{}@{}Hz",
                    monitor.width, monitor.height, monitor.refresh_rate
                )));
                resolution.set_widget_name("monitor-label");
                monitor_box.append(&resolution);
                let pos = Label::new(Some(&format!("Position: {}x{}", monitor.x, monitor.y)));
                pos.set_widget_name("monitor-label");
                monitor_box.append(&pos);
                let scale = Label::new(Some(&format!("Scale: {}", monitor.scale)));
                scale.set_widget_name("monitor-label");
                monitor_box.append(&scale);
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

