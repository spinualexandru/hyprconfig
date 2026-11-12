//! Cards and their surfaces: the `Card` builder, plain surfaces, icon tiles, note and error cards.

use rune::{Color, El, Icon, col, el, icon, row, text};
use rune_kit::Kit;

use super::card_title_style;

/// A bordered surface with an optional header (icon tile, title, description, actions).
pub struct Card {
    icon: Option<&'static Icon>,
    title: Option<String>,
    description: Option<String>,
    actions: Vec<El>,
    body: Vec<El>,
    border: Option<Color>,
    gap: f32,
}

impl Card {
    pub fn new() -> Card {
        Card {
            icon: None,
            title: None,
            description: None,
            actions: Vec::new(),
            body: Vec::new(),
            border: None,
            gap: 16.0,
        }
    }
    pub fn icon(mut self, i: &'static Icon) -> Self {
        self.icon = Some(i);
        self
    }
    pub fn title(mut self, t: impl Into<String>) -> Self {
        self.title = Some(t.into());
        self
    }
    pub fn description(mut self, d: impl Into<String>) -> Self {
        self.description = Some(d.into());
        self
    }
    pub fn action(mut self, a: El) -> Self {
        self.actions.push(a);
        self
    }
    pub fn child(mut self, c: El) -> Self {
        self.body.push(c);
        self
    }
    pub fn children(mut self, c: impl IntoIterator<Item = El>) -> Self {
        self.body.extend(c);
        self
    }
    pub fn border(mut self, c: Color) -> Self {
        self.border = Some(c);
        self
    }
    pub fn gap(mut self, g: f32) -> Self {
        self.gap = g;
        self
    }
    pub fn build(self, k: &Kit) -> El {
        let mut root = surface(k).gap(self.gap);
        if let Some(b) = self.border {
            root = root.border(1.0, b);
        }
        if self.title.is_some() || self.description.is_some() || !self.actions.is_empty() {
            let mut head = row().key("card-head").w_full().gap(12.0).center_y();
            if let Some(i) = self.icon {
                head = head.child(icon_tile(k, i));
            }
            let mut titles = col().key("titles").grow(1.0).shrink().min_w(0.0).gap(2.0);
            if let Some(t) = self.title {
                titles = titles.child(
                    text(t)
                        .key("title")
                        .role("heading")
                        .font(card_title_style())
                        .color(k.c.text_strong),
                );
            }
            if let Some(d) = self.description {
                titles = titles.child(
                    text(d)
                        .key("description")
                        .font(k.ty.body)
                        .color(k.c.text_muted)
                        .wrap()
                        .w_full(),
                );
            }
            head = head.child(titles);
            if !self.actions.is_empty() {
                head = head.child(
                    row()
                        .key("card-actions")
                        .gap(8.0)
                        .center_y()
                        .children(self.actions),
                );
            }
            root = root.child(head);
        }
        root.children(self.body)
    }
}

/// A card's surface without the header logic, for custom layouts.
pub fn surface(k: &Kit) -> El {
    col()
        .w_full()
        .padding(20.0)
        .radius(12.0)
        .fill(k.c.surface_raised_base)
        .border(1.0, k.c.border_weak)
}

/// The rounded square behind a card's or row's icon.
pub fn icon_tile(k: &Kit, i: &'static Icon) -> El {
    icon_tile_colored(k, i, k.c.text_accent)
}

pub fn icon_tile_colored(_k: &Kit, i: &'static Icon, ink: Color) -> El {
    el().key("tile")
        .size(40.0, 40.0)
        .radius(10.0)
        .fill(ink.alpha(0.12))
        .center()
        .child(icon(i, 20.0).color(ink))
        .a11y_hidden()
}

/// A muted card holding one line of text (empty states).
pub fn note_card(k: &Kit, key: &str, lines: &[&str]) -> El {
    surface(k)
        .key(key)
        .gap(8.0)
        .children(lines.iter().enumerate().map(|(i, l)| {
            text((*l).to_owned())
                .key(format!("line{i}"))
                .font(k.ty.body)
                .color(k.c.text_muted)
                .wrap()
                .w_full()
        }))
}

/// A danger-bordered card with an error message.
pub fn error_card(k: &Kit, key: &str, title: &str, message: &str) -> El {
    Card::new()
        .title(title)
        .border(k.c.danger.icon)
        .gap(8.0)
        .child(
            text(message.to_owned())
                .key("message")
                .font(k.ty.body)
                .color(k.c.text_muted)
                .wrap()
                .w_full()
                .selectable(true),
        )
        .build(k)
        .key(key)
}
