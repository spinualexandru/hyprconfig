//! The keybind form's modifier checkboxes, and how modifiers as written in a config map to them.

use super::command::variable_name;

/// The modifiers the keybind form has a checkbox for.
pub const MODIFIERS: [&str; 4] = ["SUPER", "SHIFT", "ALT", "CTRL"];

/// The checkbox a modifier as written in a config ticks: Hyprland's aliases (`CONTROL`,
/// `MOD4`, `WIN`, ...) included, any case. `None` for modifiers without a checkbox (`CAPS`,
/// `MOD5`, a variable, `CTRL+ALT` written as one word).
pub fn known_modifier(written: &str) -> Option<&'static str> {
    match written.to_uppercase().as_str() {
        "SUPER" | "WIN" | "LOGO" | "MOD4" | "META" => Some("SUPER"),
        "SHIFT" => Some("SHIFT"),
        "ALT" | "MOD1" => Some("ALT"),
        "CTRL" | "CONTROL" => Some("CTRL"),
        _ => None,
    }
}

/// What a bind's modifier stands for in the form: a checkbox name (`SUPER`), also through a
/// `$variable` naming one modifier, or else the modifier exactly as written.
pub fn modifier_id(written: &str, variables: &[(&str, &str)]) -> String {
    if let Some(m) = known_modifier(written) {
        return m.to_owned();
    }
    if written.starts_with('$') {
        let name = variable_name(written);
        if let Some(m) = variables
            .iter()
            .find(|(n, _)| *n == name)
            .and_then(|(_, v)| known_modifier(v.trim()))
        {
            return m.to_owned();
        }
    }
    written.to_owned()
}

/// The modifiers to write for a bind that had `original` (each as written, with its
/// `modifier_id`) once the form's ticked ids are `ticked`. Kept modifiers stay as written and
/// in their order, so an untouched set saves unchanged; newly ticked ones follow in the form's
/// order.
pub fn modifiers_to_save(original: &[(String, String)], ticked: &[String]) -> Vec<String> {
    let mut out: Vec<String> = original
        .iter()
        .filter(|(_, id)| ticked.contains(id))
        .map(|(written, _)| written.clone())
        .collect();
    for m in MODIFIERS {
        let wanted = ticked.iter().any(|t| t == m);
        let kept = original.iter().any(|(_, id)| id == m);
        if wanted && !kept {
            out.push(m.to_owned());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn written(mods: &[&str], vars: &[(&str, &str)]) -> Vec<(String, String)> {
        mods.iter()
            .map(|m| ((*m).to_owned(), modifier_id(m, vars)))
            .collect()
    }

    fn ids(original: &[(String, String)]) -> Vec<String> {
        original.iter().map(|(_, id)| id.clone()).collect()
    }

    #[test]
    fn modifier_aliases_tick_their_checkbox() {
        assert_eq!(known_modifier("control"), Some("CTRL"));
        assert_eq!(known_modifier("MOD4"), Some("SUPER"));
        assert_eq!(known_modifier("Win"), Some("SUPER"));
        assert_eq!(known_modifier("MOD5"), None);
        assert_eq!(known_modifier("CONTROL+MOD4"), None);
        let vars = [("mainMod", "SUPER")];
        assert_eq!(modifier_id("$mainMod", &vars), "SUPER");
        assert_eq!(modifier_id("${mainMod}", &vars), "SUPER");
        assert_eq!(modifier_id("$other", &vars), "$other");
        assert_eq!(modifier_id("CAPS", &vars), "CAPS");
    }

    #[test]
    fn untouched_modifiers_save_as_written() {
        let vars = [("mainMod", "SUPER")];
        for mods in [
            &["$mainMod"][..],
            &["CONTROL", "MOD4"],
            &["CTRL", "SUPER"],
            &["CONTROL+MOD4"],
            &["$unknown", "SHIFT"],
            &["super", "shift"],
            &[],
        ] {
            let original = written(mods, &vars);
            assert_eq!(modifiers_to_save(&original, &ids(&original)), mods);
        }
    }

    #[test]
    fn changed_modifiers_keep_the_rest_as_written() {
        let vars = [("mainMod", "SUPER")];
        let original = written(&["$mainMod", "CONTROL", "MOD5"], &vars);
        // Untick CTRL, tick SHIFT.
        let ticked = vec!["SUPER".to_owned(), "MOD5".to_owned(), "SHIFT".to_owned()];
        assert_eq!(
            modifiers_to_save(&original, &ticked),
            vec!["$mainMod", "MOD5", "SHIFT"]
        );
        // Untick SUPER: the variable standing for it goes.
        let ticked = vec!["CTRL".to_owned(), "MOD5".to_owned()];
        assert_eq!(
            modifiers_to_save(&original, &ticked),
            vec!["CONTROL", "MOD5"]
        );
        // A new bind gets the form's order.
        let ticked = vec!["CTRL".to_owned(), "SUPER".to_owned()];
        assert_eq!(modifiers_to_save(&[], &ticked), vec!["SUPER", "CTRL"]);
    }
}
