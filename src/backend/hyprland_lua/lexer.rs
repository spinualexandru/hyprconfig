//! Comment-aware scanning of the Lua subset hyprconfig reads: comments, string literals,
//! brackets, delimiters and top-level splitting.

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

/// The items separated by `separator` at nesting depth zero, trimmed.
pub(super) fn split_top_level(source: &str, separator: u8) -> Vec<&str> {
    split_top_level_spans(source, separator)
        .into_iter()
        .map(|(start, end)| &source[start..end])
        .collect()
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
