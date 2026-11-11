//! Window rules (v3) and layer rules (v2): named rules listed by name, each expanding to its
//! match conditions and effects, which load when first opened.

use rune::{El, col, icon, row, text};
use rune_kit::{IconButton, IconButtonVariant, Kit};

use crate::app::{self, State};
use crate::backend::hyprland as hypr;
use crate::icons;
use crate::jobs::Remote;
use crate::ui::{self, Card, Tone};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Window,
    Layer,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RuleDetail {
    pub enabled: bool,
    pub matches: Vec<(String, String)>,
    pub effects: Vec<(String, String)>,
}

#[derive(Default)]
pub struct RulesState {
    pub names: Remote<Vec<String>>,
    /// Expanded rules, by name.
    pub open: Vec<String>,
    pub details: Vec<(String, Remote<RuleDetail>)>,
}

impl RulesState {
    pub fn detail(&self, name: &str) -> Option<&Remote<RuleDetail>> {
        self.details.iter().find(|(n, _)| n == name).map(|(_, d)| d)
    }
}

fn of(s: &mut State, kind: Kind) -> &mut RulesState {
    match kind {
        Kind::Window => &mut s.windowrules,
        Kind::Layer => &mut s.layerrules,
    }
}

fn of_ref(s: &State, kind: Kind) -> &RulesState {
    match kind {
        Kind::Window => &s.windowrules,
        Kind::Layer => &s.layerrules,
    }
}

fn fetch_names(kind: Kind) -> Result<Vec<String>, String> {
    match kind {
        Kind::Window => hypr::get_windowrule_names(),
        Kind::Layer => hypr::get_layerrule_names(),
    }
}

fn fetch_detail(kind: Kind, name: String) -> Result<RuleDetail, String> {
    match kind {
        Kind::Window => hypr::get_windowrule(name).map(|r| RuleDetail {
            enabled: r.enabled,
            matches: r
                .match_properties
                .into_iter()
                .map(|p| (p.key, p.value))
                .collect(),
            effects: r
                .effect_properties
                .into_iter()
                .map(|p| (p.key, p.value))
                .collect(),
        }),
        Kind::Layer => hypr::get_layerrule(name).map(|r| RuleDetail {
            enabled: r.enabled,
            matches: r
                .match_properties
                .into_iter()
                .map(|p| (p.key, p.value))
                .collect(),
            effects: r
                .effect_properties
                .into_iter()
                .map(|p| (p.key, p.value))
                .collect(),
        }),
    }
}

pub fn load(s: &mut State, kind: Kind) {
    let attempt = of(s, kind).names.begin();
    s.jobs.run(
        "rule-names",
        move || fetch_names(kind),
        move |s, r, _| {
            let rules = of(s, kind);
            if rules.names.land(attempt, r) {
                // Details may have changed with the file; reopened rules load again.
                let names = rules.names.data.clone().unwrap_or_default();
                rules.open.retain(|n| names.contains(n));
                rules.details.clear();
                for name in rules.open.clone() {
                    load_detail(s, kind, name);
                }
            }
        },
    );
}

fn load_detail(s: &mut State, kind: Kind, name: String) {
    let rules = of(s, kind);
    let attempt = match rules.details.iter_mut().find(|(n, _)| *n == name) {
        Some((_, d)) => d.begin(),
        None => {
            let mut d = Remote::default();
            let a = d.begin();
            rules.details.push((name.clone(), d));
            a
        }
    };
    let n = name.clone();
    s.jobs.run(
        "rule-detail",
        move || fetch_detail(kind, n),
        move |s, r, _| {
            if let Some((_, d)) = of(s, kind).details.iter_mut().find(|(n, _)| *n == name) {
                d.land(attempt, r);
            }
        },
    );
}

pub fn toggle(s: &mut State, kind: Kind, name: &str) {
    let rules = of(s, kind);
    if let Some(i) = rules.open.iter().position(|n| n == name) {
        rules.open.remove(i);
        return;
    }
    rules.open.push(name.to_owned());
    if rules
        .detail(name)
        .is_none_or(|d| d.data.is_none() && !d.loading)
    {
        load_detail(s, kind, name.to_owned());
    }
}

pub fn delete(s: &mut State, kind: Kind, name: String) {
    let names = &mut of(s, kind).names;
    // One change at a time: unnamed Lua rules go by position, which shifts until the list
    // reloads.
    if names.settling() {
        return;
    }
    names.hold();
    s.jobs.write(
        "delete-rule",
        move || match kind {
            Kind::Window => hypr::delete_windowrule(name),
            Kind::Layer => hypr::delete_layerrule(name),
        },
        move |s, r, tx| {
            of(s, kind).names.after_write(r.is_ok());
            match r {
                Ok(()) => load(s, kind),
                Err(e) => app::fail(s, tx, "Failed to delete the rule", e),
            }
        },
    );
}

fn props(k: &Kit, key: &str, title: &str, list: &[(String, String)], tone: Tone) -> El {
    col()
        .key(key)
        .w_full()
        .gap(8.0)
        .child(
            text(title.to_owned())
                .font(k.ty.body_medium)
                .color(k.c.text_muted),
        )
        .child(
            row().key("pills").w_full().gap(8.0).flex_wrap().children(
                list.iter()
                    .map(|(key, v)| ui::pill(k, format!("{key}: {v}"), tone)),
            ),
        )
}

fn detail_error(k: &Kit, e: &str) -> El {
    text(e.to_owned())
        .key("error")
        .font(k.ty.body)
        .color(k.c.danger.icon)
        .wrap()
        .w_full()
}

fn rule_card(s: &State, k: &Kit, kind: Kind, name: &str) -> El {
    let rules = of_ref(s, kind);
    let open = rules.open.iter().any(|n| n == name);
    let detail = rules.detail(name);
    let n = name.to_owned();
    let mut head = row()
        .key("head")
        .role("button")
        .label(name.to_owned())
        .expanded(open)
        .grow(1.0)
        .shrink()
        .min_w(0.0)
        .gap(8.0)
        .py(4.0)
        .center_y()
        .focusable(true)
        .on_press(move |s: &mut State| toggle(s, kind, &n))
        .child(
            icon(
                if open {
                    &icons::CHEVRON_DOWN
                } else {
                    &icons::CHEVRON_RIGHT
                },
                16.0,
            )
            .color(k.c.icon_muted),
        )
        .child(
            text(name.to_owned())
                .font(ui::mono(k))
                .color(k.c.text_base)
                .truncate()
                .shrink(),
        );
    if detail
        .and_then(|d| d.data.as_ref())
        .is_some_and(|d| !d.enabled)
    {
        head = head.child(ui::pill(k, "disabled", Tone::Neutral));
    }
    let n = name.to_owned();
    let bar = row()
        .key("bar")
        .w_full()
        .px(12.0)
        .py(6.0)
        .gap(8.0)
        .center_y()
        .hover(k.c.hover)
        .child(head)
        .child(
            IconButton::new(&icons::TRASH_2)
                .variant(IconButtonVariant::Ghost)
                .label(format!("Delete {name}"))
                .help("Delete")
                .disabled(rules.names.settling())
                .on_press(move |s: &mut State| delete(s, kind, n.clone()))
                .build(k)
                .key("delete"),
        );
    let mut card = col()
        .key(format!("rule-{name}"))
        .w_full()
        .radius(8.0)
        .border(1.0, k.c.border_weak)
        .clip_rounded(8.0)
        .child(bar);
    if open {
        let body = match detail {
            Some(d) if d.data.is_none() && d.loading => col()
                .w_full()
                .gap(8.0)
                .child(ui::skeleton(k, None, 14.0))
                .child(ui::skeleton(k, Some(240.0), 14.0)),
            Some(Remote {
                data: None,
                error: Some(e),
                ..
            }) => detail_error(k, e),
            Some(Remote {
                data: Some(d),
                error,
                ..
            }) => {
                let mut b = col().w_full().gap(16.0);
                // A failed reload shows above what loaded before.
                if let Some(e) = error {
                    b = b.child(detail_error(k, e));
                }
                if !d.matches.is_empty() {
                    b = b.child(props(
                        k,
                        "matches",
                        "Match Conditions",
                        &d.matches,
                        Tone::Neutral,
                    ));
                }
                if !d.effects.is_empty() {
                    b = b.child(props(k, "effects", "Effects", &d.effects, Tone::Outline));
                }
                if d.matches.is_empty() && d.effects.is_empty() {
                    b = b.child(
                        text("No properties defined")
                            .font(k.ty.body)
                            .color(k.c.text_muted),
                    );
                }
                b
            }
            _ => col(),
        };
        card = card.child(
            col()
                .key("detail")
                .w_full()
                .px(16.0)
                .pt(8.0)
                .pb(16.0)
                .border_sides([1.0, 0.0, 0.0, 0.0], k.c.border_weak)
                .child(body),
        );
    }
    card
}

pub fn view(s: &State, k: &Kit, kind: Kind) -> El {
    let rules = of_ref(s, kind);
    let (key, title, subtitle, icon, singular) = match kind {
        Kind::Window => (
            "windowrules",
            "Windowrules",
            "View and manage your Hyprland window rules",
            &icons::APP_WINDOW,
            "windowrule",
        ),
        Kind::Layer => (
            "layerrules",
            "Layerrules",
            "View and manage your Hyprland layer rules",
            &icons::LAYERS,
            "layerrule",
        ),
    };
    let description = format!("Your Hyprland {singular} configurations");
    let refresh = ui::refresh_button(rules.names.loading, move |s, _| load(s, kind))
        .build(k)
        .key("refresh");
    let mut body = Vec::new();
    if let Some(e) = &rules.names.error {
        body.push(ui::error_card(k, "error", "Error", e));
    }
    match rules.names.data.as_deref() {
        None if rules.names.first_load() => body.push(
            Card::new()
                .icon(icon)
                .title(title)
                .description(description)
                .child(ui::skeleton_rows(k, 4, &[Some(160.0), None]))
                .build(k)
                .key("loading"),
        ),
        None => {}
        Some([]) => {
            let hint = match kind {
                Kind::Window => {
                    "Rules are declared with hl.window_rule({ match = { class = \"...\" }, float = true }). Rules without a name are listed by position."
                }
                Kind::Layer => {
                    "Rules are declared with hl.layer_rule({ match = { namespace = \"...\" }, blur = true }). Rules without a name are listed by position."
                }
            };
            let first = format!("No {singular}s found in your Hyprland configuration.");
            body.push(ui::note_card(k, "empty", &[&first, hint]));
        }
        Some(names) => body.push(
            Card::new()
                .icon(icon)
                .title(format!("{title} ({})", names.len()))
                .description(description)
                .child(
                    col()
                        .key("rules")
                        .w_full()
                        .gap(8.0)
                        .children(names.iter().map(|n| rule_card(s, k, kind, n))),
                )
                .build(k)
                .key("list"),
        ),
    }
    ui::page(k, key, title, subtitle, vec![refresh], body)
}
