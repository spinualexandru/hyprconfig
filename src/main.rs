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
    // Commands that run and exit before any window:
    // `hyprconfig notify <event>` tells the running app something changed (Noctalia's hooks);
    // `hyprconfig noctalia off` takes the Noctalia integration out (`just uninstall`).
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let command = match args.as_slice() {
        ["notify", rest @ ..] => Some(ipc::notify(rest.first().copied().unwrap_or_default())),
        ["noctalia", "off"] => Some(backend::palette::remove_noctalia_integration()),
        _ => None,
    };
    if let Some(result) = command {
        if let Err(e) = result {
            eprintln!("hyprconfig: {e}");
            std::process::exit(1);
        }
        return;
    }
    rune::prewarm_gpu();
    // Wayland app_id / X11 WM_CLASS, matching hyprconfig.desktop and users' window rules.
    rune::tooling::set_app_id(app::APP_ID);
    // A headless run (`--scene`, `--snap`) draws made-up data, not the user's prefs and colours.
    let init = if rune::is_headless() {
        app::State::new
    } else {
        app::State::launch
    };
    let app = rune::App::new(app::spec()).init(init);
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
