//! Comment-aware scanning and evaluation of the Lua subset hyprconfig reads.

use std::collections::HashMap;

pub(super) fn long_bracket_level(bytes: &[u8], i: usize) -> Option<usize> {
    if bytes.get(i) != Some(&b'[') {
        return None;
    }
    let mut j = i + 1;
    while bytes.get(j) == Some(&b'=') {
        j += 1;
    }
    (bytes.get(j) == Some(&b'[')).then_some(j - i - 1)
}

pub(super) fn long_bracket_end(bytes: &[u8], from: usize, level: usize) -> usize {
    let mut i = from;
    while i < bytes.len() {
        if bytes[i] == b']' {
            let mut j = i + 1;
            while bytes.get(j) == Some(&b'=') {
                j += 1;
            }
            if j - i - 1 == level && bytes.get(j) == Some(&b']') {
                return j + 1;
            }
        }
        i += 1;
    }
    bytes.len()
}

/// When a string literal starts at `i`, the offset just past its end.
pub(super) fn string_literal_end(bytes: &[u8], i: usize) -> Option<usize> {
    match *bytes.get(i)? {
        quote @ (b'"' | b'\'') => {
            let mut j = i + 1;
            while j < bytes.len() {
                match bytes[j] {
                    b'\\' => j += 2,
                    b'\n' => return Some(j),
                    b if b == quote => return Some(j + 1),
                    _ => j += 1,
                }
            }
            Some(bytes.len())
        }
        b'[' => {
            long_bracket_level(bytes, i).map(|level| long_bracket_end(bytes, i + level + 2, level))
        }
        _ => None,
    }
}

/// Blanks out `--` line comments and `--[[ ]]` block comments, keeping newlines so every
/// byte offset still lines up with the original text.
pub(super) fn mask_lua_comments(contents: &str) -> String {
    let bytes = contents.as_bytes();
    let mut out = bytes.to_vec();
    let mut i = 0usize;

    while i < bytes.len() {
        if let Some(end) = string_literal_end(bytes, i) {
            i = end.max(i + 1);
            continue;
        }
        if bytes[i] == b'-' && bytes.get(i + 1) == Some(&b'-') {
            let end = match long_bracket_level(bytes, i + 2) {
                Some(level) => long_bracket_end(bytes, i + 2 + level + 2, level),
                None => bytes[i..]
                    .iter()
                    .position(|b| *b == b'\n')
                    .map(|pos| i + pos)
                    .unwrap_or(bytes.len()),
            };
            for byte in &mut out[i..end] {
                if *byte != b'\n' {
                    *byte = b' ';
                }
            }
            i = end;
            continue;
        }
        i += 1;
    }

    String::from_utf8(out).unwrap_or_else(|_| contents.to_string())
}

pub(super) fn is_ident_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// Offsets of `word` used as a standalone identifier outside string literals.
pub(super) fn find_keyword_positions(masked: &str, word: &str) -> Vec<usize> {
    let bytes = masked.as_bytes();
    let mut positions = Vec::new();
    let mut i = 0usize;

    while i < bytes.len() {
        if let Some(end) = string_literal_end(bytes, i) {
            i = end.max(i + 1);
            continue;
        }
        if is_ident_byte(bytes[i]) {
            let start = i;
            while i < bytes.len() && is_ident_byte(bytes[i]) {
                i += 1;
            }
            // `x.word` is a field access, `x ..word` a concatenation.
            let member_access =
                start > 0 && bytes[start - 1] == b'.' && (start < 2 || bytes[start - 2] != b'.');
            if &masked[start..i] == word && !member_access {
                positions.push(start);
            }
            continue;
        }
        i += 1;
    }

    positions
}

pub(super) fn find_matching_delimiter(
    contents: &str,
    open: usize,
    left: u8,
    right: u8,
) -> Option<usize> {
    let bytes = contents.as_bytes();
    let mut depth = 0usize;
    let mut i = open;

    while i < bytes.len() {
        if let Some(end) = string_literal_end(bytes, i) {
            i = end.max(i + 1);
            continue;
        }
        let byte = bytes[i];
        if byte == b'-' && bytes.get(i + 1) == Some(&b'-') {
            // Unmasked input: skip line comments.
            i = bytes[i..]
                .iter()
                .position(|b| *b == b'\n')
                .map(|pos| i + pos)
                .unwrap_or(bytes.len());
            continue;
        }
        if byte == left {
            depth += 1;
        } else if byte == right {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return Some(i);
            }
        }
        i += 1;
    }

    None
}

pub(super) fn block_keyword_delta(word: &str) -> i32 {
    match word {
        "function" | "do" | "if" | "repeat" => 1,
        "end" | "until" => -1,
        _ => 0,
    }
}

/// Trimmed spans of the items separated by `separator` at nesting depth zero. Nesting
/// counts brackets as well as `function`/`if`/`do` ... `end` blocks.
pub(super) fn split_top_level_spans(source: &str, separator: u8) -> Vec<(usize, usize)> {
    let bytes = source.as_bytes();
    let mut spans = Vec::new();
    let mut depth = 0i32;
    let mut start = 0usize;
    let mut i = 0usize;

    let push = |spans: &mut Vec<(usize, usize)>, from: usize, to: usize| {
        let raw = &source[from..to];
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            let lead = raw.len() - raw.trim_start().len();
            spans.push((from + lead, from + lead + trimmed.len()));
        }
    };

    while i < bytes.len() {
        if let Some(end) = string_literal_end(bytes, i) {
            i = end.max(i + 1);
            continue;
        }
        let byte = bytes[i];
        if is_ident_byte(byte) && !byte.is_ascii_digit() {
            let word_start = i;
            while i < bytes.len() && is_ident_byte(bytes[i]) {
                i += 1;
            }
            let member = word_start > 0 && bytes[word_start - 1] == b'.';
            if !member {
                depth += block_keyword_delta(&source[word_start..i]);
            }
            continue;
        }
        match byte {
            b'(' | b'{' | b'[' => depth += 1,
            b')' | b'}' | b']' => depth -= 1,
            _ if depth == 0 && (byte == separator || (separator == b',' && byte == b';')) => {
                push(&mut spans, start, i);
                start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    push(&mut spans, start, bytes.len());

    spans
}

/// Bracket depth at the start of every line.
pub(super) fn line_start_depths(masked: &str) -> Vec<(usize, i32)> {
    let bytes = masked.as_bytes();
    let mut depths = vec![(0usize, 0i32)];
    let mut depth = 0i32;
    let mut i = 0usize;

    while i < bytes.len() {
        if let Some(end) = string_literal_end(bytes, i) {
            // Long strings can span lines; those lines are not statements.
            for (offset, byte) in bytes[i..end.min(bytes.len())].iter().enumerate() {
                if *byte == b'\n' {
                    depths.push((i + offset + 1, depth + 1));
                }
            }
            i = end.max(i + 1);
            continue;
        }
        match bytes[i] {
            b'(' | b'{' | b'[' => depth += 1,
            b')' | b'}' | b']' => depth -= 1,
            b'\n' => depths.push((i + 1, depth)),
            _ => {}
        }
        i += 1;
    }

    depths
}

#[derive(Debug)]
pub(super) struct TableEntry {
    pub(super) key: Option<String>,
    /// Entry span, relative to the table source.
    pub(super) start: usize,
    pub(super) end: usize,
    pub(super) value_start: usize,
    pub(super) value_end: usize,
    /// End of the entry including its trailing separator, if any.
    pub(super) sep_end: usize,
}

/// Entries of a table constructor `{ ... }` with spans relative to `source`.
pub(super) fn table_entries(source: &str) -> Vec<TableEntry> {
    let lead = source.len() - source.trim_start().len();
    let trimmed = source.trim();
    let (inner_start, inner_end) = if trimmed.starts_with('{') && trimmed.ends_with('}') {
        (lead + 1, lead + trimmed.len() - 1)
    } else {
        (lead, lead + trimmed.len())
    };
    let inner = &source[inner_start..inner_end];
    let bytes = source.as_bytes();

    split_top_level_spans(inner, b',')
        .into_iter()
        .map(|(start, end)| {
            let (start, end) = (inner_start + start, inner_start + end);
            let text = &source[start..end];
            let (key, value_start) = match find_assignment_eq(text) {
                Some(eq) => {
                    let key = text[..eq]
                        .trim()
                        .trim_matches(['[', ']'])
                        .trim()
                        .trim_matches(['"', '\''])
                        .to_string();
                    let after = &text[eq + 1..];
                    let value_start = start + eq + 1 + (after.len() - after.trim_start().len());
                    (Some(key), value_start)
                }
                None => (None, start),
            };
            let mut sep_end = end;
            while sep_end < inner_end && matches!(bytes[sep_end], b' ' | b'\t') {
                sep_end += 1;
            }
            if sep_end < inner_end && matches!(bytes[sep_end], b',' | b';') {
                sep_end += 1;
            } else {
                sep_end = end;
            }
            TableEntry {
                key,
                start,
                end,
                value_start,
                value_end: end,
                sep_end,
            }
        })
        .collect()
}

/// Offset of the assignment `=` in `key = value`, ignoring `==`, `~=`, `<=` and `>=`.
pub(super) fn find_assignment_eq(text: &str) -> Option<usize> {
    let bytes = text.as_bytes();
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
            b'=' if depth == 0 => {
                let prev = i.checked_sub(1).map(|p| bytes[p]);
                let next = bytes.get(i + 1).copied();
                if next == Some(b'=') {
                    i += 2;
                    continue;
                }
                if !matches!(prev, Some(b'=' | b'~' | b'<' | b'>')) {
                    return Some(i);
                }
            }
            _ => {}
        }
        i += 1;
    }

    None
}

pub(super) fn parse_lua_table_fields(source: &str) -> Vec<(String, String)> {
    table_entries(source)
        .into_iter()
        .filter_map(|entry| {
            Some((
                entry.key?,
                source[entry.value_start..entry.value_end].to_string(),
            ))
        })
        .collect()
}

pub(super) fn table_value<'a>(fields: &'a [(String, String)], key: &str) -> Option<&'a str> {
    fields
        .iter()
        .find(|(name, _)| name == key)
        .map(|(_, value)| value.as_str())
}

pub(super) fn table_field(table: &str, key: &str) -> Option<String> {
    table_value(&parse_lua_table_fields(table), key).map(str::to_string)
}

// Expression evaluation

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
