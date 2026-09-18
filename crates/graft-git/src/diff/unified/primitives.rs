//! The byte-level primitives [`super::unified`]'s header parsing is built on:
//! numbers, percentages, quoted paths, and the small "find a substring in a
//! byte slice" helper none of the standard library gives `&[u8]`. Split out
//! of `unified.rs` (m2 review M8) — nothing here knows what a hunk or a file
//! header is; [`super::unified`] is the layer that does.
//!
//! Pure byte parsing, no `git` spawn — same fuzz target as `unified.rs`, so
//! nothing here may panic on arbitrary bytes either.

pub(crate) fn parse_range(r: &[u8]) -> Option<(u32, u32)> {
    match r.iter().position(|&b| b == b',') {
        Some(comma) => Some((parse_u32(&r[..comma])?, parse_u32(&r[comma + 1..])?)),
        None => Some((parse_u32(r)?, 1)),
    }
}

pub(crate) fn parse_u32(s: &[u8]) -> Option<u32> {
    if s.is_empty() || !s.iter().all(u8::is_ascii_digit) {
        return None;
    }
    std::str::from_utf8(s).ok()?.parse().ok()
}

pub(crate) fn parse_mode(rest: &[u8]) -> Option<u32> {
    parse_u32(rest)
}

pub(crate) fn parse_percent(rest: &[u8]) -> Option<u8> {
    let digits = rest.strip_suffix(b"%")?;
    if digits.is_empty() || !digits.iter().all(u8::is_ascii_digit) {
        return None;
    }
    std::str::from_utf8(digits).ok()?.parse().ok()
}

/// The `diff --git a/x b/x` line's two paths, `a/`/`b/` prefixes stripped.
///
/// Git quotes both paths together, C-style, whenever either needs it; when
/// neither is quoted, a path containing a literal `" b/"` is ambiguous (a
/// long-known git wart) and is not something this parser resolves — the
/// `---`/`+++`/`rename to`/`copy to` lines are the unambiguous source of the
/// final path whenever they are present.
pub(crate) fn parse_diff_git_header(rest: &[u8]) -> Option<(Vec<u8>, Vec<u8>)> {
    if rest.first() == Some(&b'"') {
        let (a, after) = take_quoted(rest)?;
        let after = after.strip_prefix(b" ")?;
        let b = if after.first() == Some(&b'"') {
            take_quoted(after)?.0
        } else {
            after
        };
        Some((strip_ab(&unquote(a)), strip_ab(&unquote(b))))
    } else {
        let split = find(rest, b" b/")?;
        Some((strip_ab(&rest[..split]), strip_ab(&rest[split + 1..])))
    }
}

/// `--- `/`+++ ` line's path: `None` for `/dev/null`, else unquoted with its
/// `a/`/`b/` prefix stripped.
pub(crate) fn parse_side_path(rest: &[u8]) -> Option<Vec<u8>> {
    if rest == b"/dev/null" {
        None
    } else {
        Some(strip_ab(&unquote_maybe(rest)))
    }
}

/// `rename from`/`rename to`/`copy from`/`copy to` line's path: unquoted,
/// with no `a/`/`b/` prefix (git never adds one to these).
pub(crate) fn unquote_maybe(rest: &[u8]) -> Vec<u8> {
    if rest.first() == Some(&b'"') {
        unquote(rest)
    } else {
        rest.to_vec()
    }
}

pub(crate) fn strip_ab(path: &[u8]) -> Vec<u8> {
    if path.starts_with(b"a/") || path.starts_with(b"b/") {
        path[2..].to_vec()
    } else {
        path.to_vec()
    }
}

/// Finds the first index of `needle` in `hay`.
pub(crate) fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

/// Takes one C-quoted token (`"..."`, with `\\`-escapes) from the start of
/// `s`. Returns the token including its quotes, and the rest of `s`.
pub(crate) fn take_quoted(s: &[u8]) -> Option<(&[u8], &[u8])> {
    if s.first() != Some(&b'"') {
        return None;
    }
    let mut i = 1;
    while i < s.len() {
        match s[i] {
            b'\\' => i += 2,
            b'"' => return Some((&s[..=i], &s[i + 1..])),
            _ => i += 1,
        }
    }
    None
}

/// Undoes git's C-style path quoting: `\"`, `\\`, the usual C escapes, and
/// `\NNN` octal byte escapes (how a non-UTF-8 or non-ASCII byte is spelled).
/// `token` includes its surrounding quotes; a token that is not quoted is
/// returned as-is.
pub(crate) fn unquote(token: &[u8]) -> Vec<u8> {
    let Some(inner) = token
        .strip_prefix(b"\"")
        .and_then(|s| s.strip_suffix(b"\""))
    else {
        return token.to_vec();
    };
    let mut out = Vec::with_capacity(inner.len());
    let mut i = 0;
    while i < inner.len() {
        if inner[i] != b'\\' || i + 1 >= inner.len() {
            out.push(inner[i]);
            i += 1;
            continue;
        }
        let escape = inner[i + 1];
        if escape.is_ascii_digit() {
            let mut value: u32 = 0;
            let mut j = i + 1;
            let mut digits = 0;
            while digits < 3 && j < inner.len() && inner[j].is_ascii_digit() {
                value = value * 8 + u32::from(inner[j] - b'0');
                j += 1;
                digits += 1;
            }
            // A 3-digit octal escape can spell 0-511; a value past 255 is
            // not a byte git could have written, but the parser never
            // panics on it (this is a `cargo-fuzz` target) — it saturates.
            out.push(u8::try_from(value).unwrap_or(u8::MAX));
            i = j;
            continue;
        }
        let literal = match escape {
            b'"' => b'"',
            b'\\' => b'\\',
            b'a' => 0x07,
            b'b' => 0x08,
            b'f' => 0x0c,
            b'n' => b'\n',
            b'r' => b'\r',
            b't' => b'\t',
            b'v' => 0x0b,
            other => other,
        };
        out.push(literal);
        i += 2;
    }
    out
}
