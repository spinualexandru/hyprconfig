use rune::{El, col, text};
use rune_kit::{IconButton, IconButtonVariant, Kit, Progress};

use crate::app::State;
use crate::backend::hyprland::{self as hypr, SystemInfo};
use crate::icons;
use crate::jobs::{self, Remote};
use crate::ui::{self, Card};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Default)]
pub struct AboutState {
    pub info: Remote<SystemInfo>,
}

pub fn load(s: &mut State) {
    jobs::load(
        s,
        "system-info",
        |s| &mut s.about.info,
        hypr::get_system_info,
    );
}

/// "45.2 G", "1.5T", "500 M" (df) or "8.5 GB" (free) in gigabytes; 0 when unreadable.
pub fn parse_size(value: &str) -> f64 {
    let v = value.trim();
    let split = v
        .find(|c: char| !(c.is_ascii_digit() || c == '.'))
        .unwrap_or(v.len());
    let Ok(n) = v[..split].parse::<f64>() else {
        return 0.0;
    };
    let unit = v[split..].trim().to_ascii_uppercase();
    match unit.trim_end_matches('B').trim_end_matches('I') {
        "T" => n * 1024.0,
        "G" => n,
        "M" => n / 1024.0,
        "K" => n / (1024.0 * 1024.0),
        _ => 0.0,
    }
}

fn usage(k: &Kit, key: &str, label: &str, used: &str, total: &str) -> El {
    let total_gb = parse_size(total);
    let frac = if total_gb > 0.0 {
        (parse_size(used) / total_gb) as f32
    } else {
        0.0
    };
    ui::field(
        k,
        key,
        label,
        col()
            .key("usage")
            .w_full()
            .max_w(288.0)
            .gap(8.0)
            .child(ui::value_text(k, format!("{used} / {total}")))
            .child(Progress::new(Some(frac)).build(k).a11y_hidden()),
    )
}

fn info_grid(k: &Kit, i: &SystemInfo) -> El {
    let plain = |key: &str, label: &str, v: &str| ui::field(k, key, label, ui::value_text(k, v));
    let gpus = col().key("gpus").gap(4.0).children(
        i.gpus
            .iter()
            .enumerate()
            .map(|(n, g)| ui::value_text(k, g.clone()).key(format!("gpu{n}"))),
    );
    col()
        .key("info")
        .w_full()
        .gap(16.0)
        .child(ui::grid2(
            "pairs",
            vec![
                plain("os", "OS", &i.os),
                plain("hostname", "Hostname", &i.hostname),
                plain("kernel", "Kernel", &i.kernel),
                plain("uptime", "Uptime", &i.uptime),
                plain("shell", "Shell", &i.shell),
                plain("cpu", "CPU", &i.cpu),
                usage(k, "ram", "RAM", &i.ram_used, &i.ram_total),
                usage(k, "disk", "Disk Space (Root)", &i.disk_used, &i.disk_total),
            ],
        ))
        .child(ui::field(
            k,
            "gpu",
            if i.gpus.len() > 1 { "GPUs" } else { "GPU" },
            gpus,
        ))
        .child(plain("hyprland", "Hyprland Version", &i.hyprland_version))
}

pub fn view(s: &State, k: &Kit) -> El {
    let r = &s.about.info;
    let refresh = IconButton::new(&icons::REFRESH_CW)
        .variant(IconButtonVariant::Neutral)
        .label("Refresh system information")
        .help("Refresh")
        .disabled(r.loading)
        .on_press(load)
        .build(k)
        .key("refresh");
    let error = r.error.as_ref().filter(|_| !r.loading).map(|e| {
        text(format!("Failed to load system information: {e}"))
            .key("error")
            .font(k.ty.body)
            .color(k.c.danger.icon)
            .wrap()
            .w_full()
    });
    let body = match (&r.data, error) {
        // A failed refresh shows above what loaded before.
        (Some(i), Some(error)) => col()
            .key("stale")
            .w_full()
            .gap(16.0)
            .child(error)
            .child(info_grid(k, i)),
        (Some(i), None) => info_grid(k, i),
        (None, Some(error)) => error,
        (None, None) => ui::skeleton_rows(k, 5, &[Some(140.0), None]),
    };
    ui::page(
        k,
        "about",
        "Hyprconfig",
        &format!(
            "Version {VERSION}. Hyprconfig is not an official Hyprland project and is not affiliated with Hyprland in any way."
        ),
        Vec::new(),
        vec![
            Card::new()
                .title("System Information")
                .description("Details about your system")
                .action(refresh)
                .child(body)
                .build(k)
                .key("system"),
            Card::new()
                .title("Hyprconfig")
                .child(ui::grid2(
                    "app",
                    vec![
                        ui::field(k, "version", "Version", ui::value_text(k, VERSION)),
                        ui::field(k, "license", "License", ui::value_text(k, "MIT License")),
                    ],
                ))
                .build(k)
                .key("app-card"),
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::parse_size;

    #[test]
    fn sizes_read_as_gigabytes() {
        assert_eq!(parse_size("45.5 G"), 45.5);
        assert_eq!(parse_size("1.5T"), 1536.0);
        assert_eq!(parse_size("512 M"), 0.5);
        assert_eq!(parse_size("16 GB"), 16.0);
        assert_eq!(parse_size("8.0 GiB"), 8.0);
        assert_eq!(parse_size("n/a"), 0.0);
    }
}
