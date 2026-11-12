//! Toast notifications: the queue held on `State` and the region that draws it.

use rune::{El, Tx, icon};
use rune_kit::{Kit, Toast, ToastRegion};

use crate::app::State;
use crate::icons;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToastKind {
    Success,
    Warning,
    Error,
}

#[derive(Clone, Debug)]
pub struct ToastItem {
    pub id: u64,
    pub kind: ToastKind,
    pub text: String,
    pub due: f64,
}

/// Notifications in the corner; each closes after a few seconds or on its close button.
#[derive(Default)]
pub struct Toasts {
    next: u64,
    pub items: Vec<ToastItem>,
}

impl Toasts {
    const SECS: f64 = 4.5;
    const MAX: usize = 3;

    pub fn push(&mut self, tx: &mut Tx, kind: ToastKind, text: impl Into<String>) {
        self.next += 1;
        let due = tx.clock() + Self::SECS;
        self.items.push(ToastItem {
            id: self.next,
            kind,
            text: text.into(),
            due,
        });
        if self.items.len() > Self::MAX {
            self.items.remove(0);
        }
        tx.wake_at(due);
    }

    pub fn success(&mut self, tx: &mut Tx, text: impl Into<String>) {
        self.push(tx, ToastKind::Success, text);
    }
    pub fn warning(&mut self, tx: &mut Tx, text: impl Into<String>) {
        self.push(tx, ToastKind::Warning, text);
    }
    pub fn error(&mut self, tx: &mut Tx, text: impl Into<String>) {
        self.push(tx, ToastKind::Error, text);
    }

    /// Drops expired toasts; true when any went.
    pub fn expire(&mut self, now: f64) -> bool {
        let before = self.items.len();
        self.items.retain(|t| t.due > now);
        self.items.len() != before
    }

    pub fn build(&self, k: &Kit) -> Option<El> {
        if self.items.is_empty() {
            return None;
        }
        let mut region = ToastRegion::new();
        for t in &self.items {
            let (title, glyph, ink) = match t.kind {
                ToastKind::Success => ("Done", &icons::CHECK, k.c.success.icon),
                ToastKind::Warning => ("Warning", &icons::INFO, k.c.warning.icon),
                ToastKind::Error => ("Error", &icons::X, k.c.danger.icon),
            };
            let id = t.id;
            region = region.toast(
                Toast::new(format!("toast-{id}"), title)
                    .description(t.text.clone())
                    .lead(icon(glyph, 16.0).color(ink))
                    .on_close(move |s: &mut State| s.toasts.items.retain(|t| t.id != id))
                    .build(k),
            );
        }
        Some(region.build(k))
    }
}
