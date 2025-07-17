use hyprland::shared::HyprData;
mod dashboard;
mod filesystem;
mod navigation;

use gtk::prelude::*;
use gtk::{glib, Application, ApplicationWindow};
use gtk4 as gtk;

fn main() -> glib::ExitCode {
    let app = Application::builder().application_id("hyprconfig").build();

    app.connect_activate(|app| {
        // We create the main window.
        let window = ApplicationWindow::builder()
            .application(app)
            .default_width(900)
            .default_height(650)
            .title("HyprConfig Settings")
            .build();

        // Use the navigation module for the main layout
        let navigation = navigation::Navigation::new();
        window.set_child(Some(navigation.widget()));
        window.present();
    });

    app.run()
}
