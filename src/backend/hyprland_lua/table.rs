//! Lua table constructors: splitting `{ key = value, ... }` into entries, reading fields
//! and building table bodies.

use super::lexer::{split_top_level_spans, string_literal_end};

/// The body of a table constructor: `key = value` pairs joined with commas.
pub(super) fn lua_table_fields(fields: &[(impl AsRef<str>, impl AsRef<str>)]) -> String {
    fields
        .iter()
        .map(|(key, value)| format!("{} = {}", key.as_ref(), value.as_ref()))
        .collect::<Vec<_>>()
        .join(", ")
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
