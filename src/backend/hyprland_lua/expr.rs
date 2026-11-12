//! Evaluation of Lua string expressions and literals, plus quoting values back into Lua.

use std::collections::HashMap;

use super::lexer::{
    find_matching_delimiter, long_bracket_end, long_bracket_level, string_literal_end,
};

pub(super) fn eval_lua_string_expr(
    expr: &str,
    variables: &HashMap<String, String>,
) -> Option<String> {
    let expr = strip_wrapping_parens(strip_lua_line_comment(expr).trim());
    if expr.is_empty() {
        return Some(String::new());
    }

    if let Some((left, right)) = split_lua_or(expr) {
        if let Some(value) = eval_lua_string_expr(left, variables)
            && !value.is_empty()
        {
            return Some(value);
        }
        return eval_lua_string_expr(right, variables);
    }

    if let Some(env_name) = expr
        .strip_prefix("os.getenv(")
        .and_then(|rest| rest.strip_suffix(')'))
        .and_then(parse_lua_string_literal)
    {
        return std::env::var(env_name).ok();
    }

    let parts = split_lua_concat(expr);
    let mut output = String::new();
    for part in parts {
        let part = strip_wrapping_parens(part.trim());
        if let Some(value) = parse_lua_string_literal(part) {
            output.push_str(&value);
        } else if let Some(value) = variables.get(part) {
            output.push_str(value);
        } else if let Some(key) = part
            .strip_prefix("vars.")
            .or_else(|| part.strip_prefix("context.vars."))
        {
            output.push_str(variables.get(key)?);
        } else if is_lua_number(part) {
            output.push_str(part);
        } else if let Some((_prefix, key)) = part.rsplit_once('.') {
            output.push_str(variables.get(key)?);
        } else {
            return None;
        }
    }
    Some(output)
}

pub(super) fn is_lua_number(value: &str) -> bool {
    !value.is_empty()
        && value.parse::<f64>().is_ok()
        && value
            .chars()
            .all(|c| c.is_ascii_digit() || matches!(c, '.' | '-' | '+' | 'e' | 'E'))
}

pub(super) fn strip_wrapping_parens(value: &str) -> &str {
    let mut value = value.trim();
    loop {
        if value.starts_with('(')
            && value.ends_with(')')
            && find_matching_delimiter(value, 0, b'(', b')') == Some(value.len() - 1)
        {
            value = value[1..value.len() - 1].trim();
        } else {
            return value;
        }
    }
}

pub(super) fn split_lua_or(expr: &str) -> Option<(&str, &str)> {
    let bytes = expr.as_bytes();
    let mut depth = 0i32;
    let mut i = 0usize;

    while i < bytes.len() {
        if let Some(end) = string_literal_end(bytes, i) {
            i = end.max(i + 1);
            continue;
        }
        match bytes[i] {
            b'(' | b'{' | b'[' => depth += 1,
            b')' | b'}' | b']' => depth -= 1,
            b' ' if depth == 0 && i > 0 && bytes.get(i..i + 4) == Some(b" or ") => {
                return Some((&expr[..i], &expr[i + 4..]));
            }
            _ => {}
        }
        i += 1;
    }

    None
}

pub(super) fn split_lua_concat(expr: &str) -> Vec<String> {
    let bytes = expr.as_bytes();
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut depth = 0i32;
    let mut i = 0usize;

    while i < bytes.len() {
        if let Some(end) = string_literal_end(bytes, i) {
            i = end.max(i + 1);
            continue;
        }
        match bytes[i] {
            b'(' | b'{' | b'[' => depth += 1,
            b')' | b'}' | b']' => depth -= 1,
            b'.' if depth == 0 && bytes.get(i + 1) == Some(&b'.') => {
                parts.push(expr[start..i].trim().to_string());
                start = i + 2;
                i += 2;
                continue;
            }
            _ => {}
        }
        i += 1;
    }
    parts.push(expr[start..].trim().to_string());
    parts
}

pub(super) fn parse_lua_string_literal(value: &str) -> Option<String> {
    let value = value.trim();
    let bytes = value.as_bytes();

    if let Some(level) = long_bracket_level(bytes, 0) {
        if long_bracket_end(bytes, level + 2, level) != bytes.len() {
            return None;
        }
        let inner = &value[level + 2..value.len() - level - 2];
        // Lua drops a newline directly after the opening bracket.
        return Some(inner.strip_prefix('\n').unwrap_or(inner).to_string());
    }

    let quote = bytes.first().copied()?;
    if quote != b'\'' && quote != b'"' {
        return None;
    }
    if string_literal_end(bytes, 0) != Some(bytes.len()) || bytes.len() < 2 {
        return None;
    }

    let inner = &value[1..value.len() - 1];
    let mut out = String::new();
    let mut chars = inner.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('r') => out.push('\r'),
                Some(other) => out.push(other),
                None => out.push('\\'),
            }
        } else {
            out.push(ch);
        }
    }
    Some(out)
}

pub(super) fn strip_lua_line_comment(value: &str) -> &str {
    let bytes = value.as_bytes();
    let mut i = 0usize;

    while i < bytes.len() {
        if let Some(end) = string_literal_end(bytes, i) {
            i = end.max(i + 1);
            continue;
        }
        if bytes[i] == b'-' && bytes.get(i + 1) == Some(&b'-') {
            return &value[..i];
        }
        i += 1;
    }

    value
}

/// Collapses runs of whitespace outside string literals into single spaces.
pub(super) fn collapse_whitespace(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = String::with_capacity(value.len());
    let mut i = 0usize;
    let mut pending_space = false;

    while i < bytes.len() {
        if let Some(end) = string_literal_end(bytes, i) {
            if pending_space {
                out.push(' ');
                pending_space = false;
            }
            out.push_str(&value[i..end]);
            i = end.max(i + 1);
            continue;
        }
        let ch = value[i..].chars().next().unwrap_or(' ');
        if ch.is_whitespace() {
            pending_space = !out.is_empty();
        } else {
            if pending_space {
                out.push(' ');
                pending_space = false;
            }
            out.push(ch);
        }
        i += ch.len_utf8();
    }

    out
}

pub(super) fn lua_value_to_display(value: &str) -> String {
    parse_lua_string_literal(value.trim()).unwrap_or_else(|| collapse_whitespace(value.trim()))
}

pub(super) fn lua_value_expr(value: &str) -> String {
    if value.trim().parse::<i64>().is_ok() {
        value.trim().to_string()
    } else {
        lua_quote(value.trim())
    }
}

pub(super) fn is_lua_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

pub(super) fn is_lua_keyword(value: &str) -> bool {
    matches!(
        value,
        "and"
            | "break"
            | "do"
            | "else"
            | "elseif"
            | "end"
            | "false"
            | "for"
            | "function"
            | "goto"
            | "if"
            | "in"
            | "local"
            | "nil"
            | "not"
            | "or"
            | "repeat"
            | "return"
            | "then"
            | "true"
            | "until"
            | "while"
    )
}

pub(super) fn lua_quote(value: &str) -> String {
    let mut out = String::from("\"");
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(ch),
        }
    }
    out.push('"');
    out
}
