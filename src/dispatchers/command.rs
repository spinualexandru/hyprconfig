//! Parsing a bind's command into plain text and `$variable` references.

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Part {
    pub text: String,
    pub variable: bool,
}

/// Whether `c` can be part of a variable name.
pub fn is_name_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Splits a command into plain text and `$name` / `${name}` variable references.
pub fn parse_command(command: &str) -> Vec<Part> {
    let word = is_name_char;
    let mut parts = Vec::new();
    let mut plain = String::new();
    let mut rest = command;
    while let Some(i) = rest.find('$') {
        plain.push_str(&rest[..i]);
        let after = &rest[i + 1..];
        let braced = after.strip_prefix('{').and_then(|b| {
            let end = b.find('}')?;
            (end > 0 && b[..end].chars().all(word)).then_some(end + 3)
        });
        let len =
            braced.unwrap_or_else(|| 1 + after.find(|c: char| !word(c)).unwrap_or(after.len()));
        if len > 1 {
            if !plain.is_empty() {
                parts.push(Part {
                    text: std::mem::take(&mut plain),
                    variable: false,
                });
            }
            parts.push(Part {
                text: rest[i..i + len].to_owned(),
                variable: true,
            });
        } else {
            plain.push('$');
        }
        rest = &rest[i + len..];
    }
    plain.push_str(rest);
    if !plain.is_empty() {
        parts.push(Part {
            text: plain,
            variable: false,
        });
    }
    parts
}

/// Splits a command like `parse_command`, keeping only references to names `known` answers
/// for as variables. `${name}` must name one whole; an unbraced `$name` takes the longest known
/// prefix, so with `terminal` known, `$terminalx` is `$terminal` followed by `x`.
pub fn resolve_command(command: &str, known: impl Fn(&str) -> bool) -> Vec<Part> {
    let mut out: Vec<Part> = Vec::new();
    let mut push = |text: &str, variable: bool| match out.last_mut() {
        Some(last) if !variable && !last.variable => last.text.push_str(text),
        _ if !text.is_empty() => out.push(Part {
            text: text.to_owned(),
            variable,
        }),
        _ => {}
    };
    for part in parse_command(command) {
        if !part.variable {
            push(&part.text, false);
            continue;
        }
        let name = variable_name(&part.text);
        let braced = part.text.starts_with("${");
        let prefix = if braced {
            known(name).then_some(name.len())
        } else {
            (1..=name.len()).rev().find(|&n| known(&name[..n]))
        };
        match prefix {
            Some(_) if braced => push(&part.text, true),
            Some(n) => {
                push(&part.text[..1 + n], true);
                push(&name[n..], false);
            }
            None => push(&part.text, false),
        }
    }
    out
}

/// `$name` or `${name}` to `name`.
pub fn variable_name(reference: &str) -> &str {
    reference
        .trim_start_matches('$')
        .trim_start_matches('{')
        .trim_end_matches('}')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(text: &str, variable: bool) -> Part {
        Part {
            text: text.into(),
            variable,
        }
    }

    #[test]
    fn splits_variables_from_text() {
        assert_eq!(
            parse_command("$terminal -e htop"),
            vec![p("$terminal", true), p(" -e htop", false)]
        );
        assert_eq!(
            parse_command("${menu}run --x"),
            vec![p("${menu}", true), p("run --x", false)]
        );
        assert_eq!(
            parse_command("echo $ $5"),
            vec![p("echo $ ", false), p("$5", true)]
        );
        assert_eq!(parse_command("plain"), vec![p("plain", false)]);
        assert!(parse_command("").is_empty());
    }

    #[test]
    fn names_from_references() {
        assert_eq!(variable_name("$terminal"), "terminal");
        assert_eq!(variable_name("${menu}"), "menu");
    }

    #[test]
    fn resolves_only_known_variables() {
        let known = |n: &str| matches!(n, "terminal" | "menu");
        assert_eq!(
            resolve_command("$terminalx --hold $HOME ${menu}!", known),
            vec![
                p("$terminal", true),
                p("x --hold $HOME ", false),
                p("${menu}", true),
                p("!", false),
            ]
        );
        assert_eq!(
            resolve_command("${terminalx}", known),
            vec![p("${terminalx}", false)]
        );
    }
}
