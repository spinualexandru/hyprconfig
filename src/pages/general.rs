use rune::{El, text};
use rune_kit::Kit;

use crate::ui::{self, Card};

pub fn view(k: &Kit) -> El {
    ui::page(
        k,
        "general",
        "General",
        "General settings and preferences for Hyprland configuration.",
        Vec::new(),
        vec![
            Card::new()
                .title("System Preferences")
                .description("Configure your system settings")
                .child(
                    text("General configuration options will be added here.")
                        .key("placeholder")
                        .font(k.ty.body)
                        .color(k.c.text_muted)
                        .wrap()
                        .w_full(),
                )
                .build(k)
                .key("preferences"),
        ],
    )
}
