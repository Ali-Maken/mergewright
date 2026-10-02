use super::*;

fn pair(key: &str, value: Option<&str>) -> ConfigPair {
    (
        key.as_bytes().to_vec(),
        value.map(|value| value.as_bytes().to_vec()),
    )
}

/// What `git -c` writes (git ≥ 2.31): `'key'='value'`, space-separated.
#[test]
fn new_style_pairs_parse() {
    assert_eq!(
        config_parameters(b"'core.autocrlf'='true' 'user.name'='A B'"),
        Ok(vec![
            pair("core.autocrlf", Some("true")),
            pair("user.name", Some("A B"))
        ])
    );
    assert_eq!(
        config_parameters(b"'core.autocrlf'="),
        Ok(vec![pair("core.autocrlf", None)]),
        "`'key'=` is an implicit true"
    );
}

/// The older forms git still reads: `'key=value'`, `'key'` (bare).
#[test]
fn old_style_pairs_parse() {
    assert_eq!(
        config_parameters(b"'core.autocrlf=input' 'core.bare'"),
        Ok(vec![
            pair("core.autocrlf", Some("input")),
            pair("core.bare", None)
        ])
    );
    assert_eq!(
        config_parameters(b"'core.autocrlf='"),
        Ok(vec![pair("core.autocrlf", Some(""))]),
        "an empty value is a value (git-false), not a bare key"
    );
}

/// `'\''` and `'\!'` inside a quoted word stand for `'` and `!`.
#[test]
fn escaped_quotes_and_bangs_unquote() {
    assert_eq!(
        config_parameters(br"'alias.x'='it'\''s'\!'!'"),
        Ok(vec![pair("alias.x", Some("it's!!"))])
    );
}

/// Shapes git refuses ("bogus format"): an unquoted key, an unclosed quote,
/// a bad escape, junk after a value.
#[test]
fn malformed_parameters_are_an_error() {
    for bad in [
        &b"core.autocrlf=true"[..],
        b"'core.autocrlf'='true",
        b"'core.autocrlf'\\x'",
        b"'core.autocrlf'='true'x",
        b"'core.autocrlf'=true",
    ] {
        assert!(
            config_parameters(bad).is_err(),
            "{}",
            String::from_utf8_lossy(bad)
        );
    }
}

/// git's `isspace` takes `\r` as a separator, as it does space, `\t` and
/// `\n` (git 2.54: `'core.autocrlf'='true'\r` reads `true`); `\v` stays bogus.
#[test]
fn carriage_return_separates_like_git() {
    for raw in [
        &b"'core.autocrlf'='true'\r"[..],
        b"'core.autocrlf'='true'\r'core.bare'",
        b"'core.autocrlf'=\r",
        b"'core.autocrlf=true'\r\n",
    ] {
        let pairs = config_parameters(raw).map(|pairs| pairs.into_iter().next());
        assert!(
            matches!(&pairs, Ok(Some((key, _))) if key == b"core.autocrlf"),
            "{:?} -> {pairs:?}",
            String::from_utf8_lossy(raw)
        );
    }
    assert!(config_parameters(b"'core.autocrlf'='true'\x0b").is_err());
}
