//! Hyprconfig: configure Hyprland with a GUI, drawn with Rune.

mod app;
mod backend;
mod dispatchers;
mod fixtures;
mod icons;
mod ipc;
mod jobs;
mod pages;
mod theme;
mod ui;

#[cfg(test)]
mod tests;

fn main() {
    // `hyprconfig notify <event>`: tell the running app something changed (Noctalia's hooks).
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("notify") {
        let event = args.get(1).map(String::as_str).unwrap_or_default();
        if let Err(e) = ipc::notify(event) {
            eprintln!("hyprconfig: {e}");
            std::process::exit(1);
        }
        return;
    }
    rune::prewarm_gpu();
    // Wayland app_id / X11 WM_CLASS, matching hyprconfig.desktop and users' window rules.
    rune::tooling::set_app_id(app::APP_ID);
    let app = rune::App::new(app::spec()).init(app::State::launch);
    let handle = app.handle();
    // `on_ready` runs again if the window is recreated; the system is only wired up once.
    let mut started = false;
    app.on_ready(move |_| {
        if !std::mem::replace(&mut started, true) {
            handle.update(app::ready);
        }
    })
    .run();
}
