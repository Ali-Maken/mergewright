//! Sanitising against hostile repository content.
//!
//! Every case here is something a repository can actually contain: a commit
//! message, a branch name, an author name or a path, crafted to make the
//! terminal do something the text doesn't say. The assertions are about what
//! survives into [`SanitizedText`], never about how pretty the replacement
//! looks.

use super::*;

/// The escape byte, spelled out once so the test bodies read as the attack
/// rather than as byte soup.
const ESC: char = '\u{1B}';

/// Everything [`sanitize`] must never emit, written from the documented rule
/// rather than from [`classify`], so the two can disagree and the test notices.
///
/// The first group is the classes DESIGN.md §8.2 names; the second is this
/// module's documented widening to the whole `Cf` category and the two line
/// separators.
// Grouped by where the rule comes from — DESIGN.md's list, then this module's
// widening — which matters more here than collapsing arms that share `true`.
#[allow(clippy::match_same_arms, reason = "the grouping is the documentation")]
fn is_forbidden(c: char) -> bool {
    match c {
        '\n' | '\t' => false,

        // DESIGN.md §8.2: C0, DEL and C1; the bidi overrides and isolates;
        // the zero-width and invisible formatting characters.
        '\u{0}'..='\u{1F}' | '\u{7F}' | '\u{80}'..='\u{9F}' => true,
        '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}' => true,
        '\u{200B}'..='\u{200F}' | '\u{2060}' | '\u{FEFF}' => true,

        // This module's widening: the rest of Unicode's `Cf` category, and
        // U+2028/U+2029.
        '\u{AD}' | '\u{600}'..='\u{605}' | '\u{61C}' | '\u{6DD}' | '\u{70F}' => true,
        '\u{890}'..='\u{891}' | '\u{8E2}' | '\u{180E}' => true,
        '\u{2061}'..='\u{206F}' | '\u{2028}' | '\u{2029}' => true,
        '\u{FFF9}'..='\u{FFFB}' | '\u{110BD}' | '\u{110CD}' => true,
        '\u{13430}'..='\u{1343F}' | '\u{1BCA0}'..='\u{1BCA3}' => true,
        '\u{1D173}'..='\u{1D17A}' | '\u{E0001}' | '\u{E0020}'..='\u{E007F}' => true,

        _ => false,
    }
}

/// The same rule for single-line text, where `\n` and `\t` are two more
/// controls.
fn is_forbidden_on_one_line(c: char) -> bool {
    c == '\n' || c == '\t' || is_forbidden(c)
}

#[test]
fn ansi_colour_injection_in_a_commit_message_comes_out_inert() {
    // "fix: broken build" that repaints the row red and clears to end of line.
    let hostile = "fix: build\u{1B}[31m\u{1B}[2K fine";
    let clean = sanitize(hostile);

    assert!(!clean.contains(ESC), "ESC survived: {clean:?}");
    assert_eq!(clean.as_str(), "fix: build␛[31m␛[2K fine");
}

#[test]
fn osc_8_hyperlink_comes_out_inert() {
    // OSC 8 wraps visible text in a link to somewhere else entirely. Both the
    // ESC-introduced and the C1-introduced (0x9D) spellings must die, and so
    // must the BEL and ST terminators.
    let esc_form = "\u{1B}]8;;https://evil.example\u{7}docs\u{1B}]8;;\u{7}";
    let c1_form = "\u{9D}8;;https://evil.example\u{7}docs\u{9D}8;;\u{7}";

    for hostile in [esc_form, c1_form] {
        let clean = sanitize(hostile);
        assert!(
            !clean.chars().any(is_forbidden),
            "a control survived: {clean:?}"
        );
        assert!(
            clean.contains("docs"),
            "visible text was dropped: {clean:?}"
        );
        assert!(
            clean.contains("https://evil.example"),
            "the URL must stay visible, not silently vanish: {clean:?}"
        );
    }
}

#[test]
fn a_bidi_spoofed_branch_name_comes_out_inert() {
    // RLO makes "fix-txt.exe" render as "fix-exe.txt" while git still checks
    // out the executable.
    let hostile = "fix-\u{202E}exe.txt";
    let clean = sanitize(hostile);

    assert!(
        !clean.chars().any(is_forbidden),
        "a bidi control survived: {clean:?}"
    );
    assert_eq!(clean.as_str(), "fix-\u{FFFD}exe.txt");
}

#[test]
fn bidi_isolates_come_out_inert() {
    for c in ['\u{2066}', '\u{2067}', '\u{2068}', '\u{2069}', '\u{202A}'] {
        let clean = sanitize(&format!("a{c}b"));
        assert_eq!(clean.as_str(), "a\u{FFFD}b", "isolate {c:?} survived");
    }
}

#[test]
fn a_zero_width_payload_comes_out_inert() {
    // Two author names that look identical in every terminal.
    let hostile = "Ali\u{200B}ce\u{200D} Smith\u{FEFF}";
    let clean = sanitize(hostile);

    assert!(
        !clean.chars().any(is_forbidden),
        "a zero-width char survived: {clean:?}"
    );
    assert_eq!(clean.as_str(), "Ali\u{FFFD}ce\u{FFFD} Smith\u{FFFD}");
}

#[test]
fn invisible_characters_outside_design_md_s_list_are_neutralised_too() {
    // Not named in §8.2, all invisible, all usable to hide text in a ref name:
    // soft hyphen, Arabic letter mark, Mongolian vowel separator, a tag
    // character, and the line/paragraph separators.
    for c in [
        '\u{AD}',
        '\u{61C}',
        '\u{180E}',
        '\u{E0041}',
        '\u{2028}',
        '\u{2029}',
    ] {
        let clean = sanitize(&format!("a{c}b"));
        assert_eq!(clean.as_str(), "a\u{FFFD}b", "{c:?} survived");
    }
}

#[test]
fn newline_and_tab_pass_through_untouched() {
    // Widgets lay these out structurally (DESIGN.md §8.2); a commit body is
    // unreadable without them.
    let clean = sanitize("subject\n\nbody\twith a tab\n");
    assert_eq!(clean.as_str(), "subject\n\nbody\twith a tab\n");
}

#[test]
fn every_other_c0_control_is_replaced_visibly() {
    for code in 0x00u32..=0x1F {
        let c = char::from_u32(code).expect("C0 code points are all valid chars");
        if c == '\n' || c == '\t' {
            continue;
        }
        let clean = sanitize(&format!("a{c}b"));
        assert_eq!(clean.as_str(), "a␛b", "U+{code:04X} survived");
    }
}

#[test]
fn del_and_the_c1_block_are_replaced_visibly() {
    for code in std::iter::once(0x7Fu32).chain(0x80..=0x9F) {
        let c = char::from_u32(code).expect("DEL and C1 are all valid chars");
        let clean = sanitize(&format!("a{c}b"));
        assert_eq!(clean.as_str(), "a␛b", "U+{code:04X} survived");
    }
}

#[test]
fn ordinary_text_is_left_exactly_alone() {
    for text in [
        "",
        "Merge pull request #12 from origin/main",
        "Björk Guðmundsdóttir <bjork@example.com>",
        "修复了一个错误",
        "🎉 ship it",
        "src/graft-term/détecté.rs",
    ] {
        assert_eq!(sanitize(text).as_str(), text, "text was altered: {text:?}");
    }
}

#[test]
fn sanitising_is_idempotent() {
    // The placeholders are themselves clean, so a second pass changes nothing.
    // The app layer relies on this: re-sanitising an already-clean string on
    // its way through a view model must not corrupt it.
    let hostile = "\u{1B}[31m\u{202E}\u{200B}\u{9B}0m ok\n";
    let once = sanitize(hostile);
    let twice = sanitize(once.as_str());
    assert_eq!(once.as_str(), twice.as_str());
}

#[test]
fn one_character_in_means_one_character_out() {
    // Nothing is ever silently deleted (DESIGN.md §8.2): a hidden payload
    // shows up as a run of placeholders the user can see and count.
    let hostile = "a\u{1B}\u{202E}\u{200B}b";
    assert_eq!(sanitize(hostile).chars().count(), hostile.chars().count());
}

#[test]
fn invalid_utf8_bytes_become_visible_replacements() {
    // A path git handed us that is not valid UTF-8, with an escape hidden in
    // the valid part.
    let clean = sanitize_path_bytes(b"src/caf\xC3\x28/\x1b[31mx.rs");

    assert!(
        !clean.chars().any(is_forbidden),
        "a control survived: {clean:?}"
    );
    assert!(
        clean.contains('\u{FFFD}'),
        "lossy marker missing: {clean:?}"
    );
    assert!(clean.contains("␛[31m"), "ESC not replaced: {clean:?}");
}

#[test]
fn valid_utf8_bytes_survive_the_byte_entry_point() {
    assert_eq!(
        sanitize_path_bytes("naïve.rs".as_bytes()).as_str(),
        "naïve.rs"
    );
}

#[cfg(unix)]
#[test]
fn a_non_utf8_os_str_path_is_sanitised_lossily() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    use std::path::Path;

    let hostile = Path::new(OsStr::from_bytes(b"\xffhidden\x1b]0;pwned\x07.txt"));
    let clean = sanitize_path(hostile);

    assert!(
        !clean.chars().any(is_forbidden),
        "a control survived: {clean:?}"
    );
    assert!(clean.starts_with('\u{FFFD}'), "{clean:?}");
    assert!(clean.ends_with(".txt"), "{clean:?}");
}

#[test]
fn a_newline_or_tab_in_a_file_name_cannot_reach_a_single_line_row() {
    // Both are legal in a Unix path. Drawn raw in a status row, the newline
    // scrolls the screen and the tab jumps the cursor — repository content
    // moving the cursor, which is exactly what this module exists to stop.
    let hostile = "src/evil\nname\there.rs";

    let line = sanitize_line(hostile);
    assert_eq!(line.as_str(), "src/evil␛name␛here.rs");
    assert!(!line.chars().any(is_forbidden_on_one_line));

    // The multi-line entry point deliberately keeps them; the doc says which
    // to use where.
    assert_eq!(sanitize(hostile).as_str(), hostile);
}

#[cfg(unix)]
#[test]
fn a_path_with_a_newline_in_it_is_single_line_by_construction() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    use std::path::Path;

    let hostile = Path::new(OsStr::from_bytes(b"a\nb\tc.rs"));
    assert_eq!(sanitize_path(hostile).as_str(), "a␛b␛c.rs");
    assert_eq!(sanitize_path_bytes(b"a\nb\tc.rs").as_str(), "a␛b␛c.rs");
}

#[test]
fn sanitized_text_reads_as_a_string_slice() {
    let clean = sanitize("branch/name");

    // Deref, AsRef and Display: the three ways a widget will reach for it.
    assert_eq!(clean.len(), 11);
    assert_eq!(clean.as_str(), "branch/name");
    let as_ref: &str = clean.as_ref();
    assert_eq!(as_ref, "branch/name");
    assert_eq!(format!("{clean}"), "branch/name");
}

// The newtype's field is private and `sanitize` (with its byte and `OsStr`
// siblings) is its only constructor, so no crate outside `graft-term` can make
// a `SanitizedText` that skipped the cleaning — that is the whole point of the
// type (ARCHITECTURE.md, DESIGN.md §8.2). Confirmed by hand: adding
// `let t = graft_term::SanitizedText("raw".to_owned());` to `graft`'s
// integration test fails to compile with E0423, "cannot initialize a tuple
// struct which contains private fields". Left out of the suite because a `trybuild`
// compile-fail harness is a second test binary to build and link, which
// PROMPT.md §7.1 warns off; the guarantee is a visibility rule, not logic that
// can drift.

mod properties {
    use proptest::prelude::*;

    use super::*;

    /// The case count for whichever tier this was compiled for: 64 in the
    /// fast tier, 1000 with `--features heavy-tests` (PROMPT.md §7.1).
    /// `graft-fixtures` owns the two numbers so that turning the tier on
    /// changes every proptest in the workspace at once. The environment
    /// cannot change the count (ADR-0017).
    fn tier() -> ProptestConfig {
        ProptestConfig::with_cases(graft_fixtures::proptest_cases())
    }

    proptest! {
        #![proptest_config(tier())]

        /// The whole contract, over arbitrary text: nothing forbidden gets
        /// out, nothing is silently dropped, and a second pass is a no-op.
        #[test]
        fn output_is_inert_length_preserving_and_idempotent(text in "(?s).{0,80}") {
            let clean = sanitize(&text);

            prop_assert!(!clean.chars().any(is_forbidden));
            prop_assert_eq!(clean.chars().count(), text.chars().count());
            let again = sanitize(clean.as_str());
            prop_assert_eq!(again.as_str(), clean.as_str());
        }

        /// Same contract for arbitrary bytes, which is what a path from git
        /// actually is.
        #[test]
        fn arbitrary_path_bytes_are_inert_too(bytes in proptest::collection::vec(any::<u8>(), 0..80)) {
            let clean = sanitize_path_bytes(&bytes);

            prop_assert!(!clean.chars().any(is_forbidden_on_one_line));
            let again = sanitize_path_bytes(clean.as_bytes());
            prop_assert_eq!(again.as_str(), clean.as_str());
        }

        /// Single-line text never keeps anything that can move the cursor,
        /// `\n` and `\t` included.
        #[test]
        fn single_line_output_cannot_move_the_cursor(text in "(?s).{0,80}") {
            let clean = sanitize_line(&text);

            prop_assert!(!clean.chars().any(is_forbidden_on_one_line));
            prop_assert_eq!(clean.chars().count(), text.chars().count());
            let again = sanitize_line(clean.as_str());
            prop_assert_eq!(again.as_str(), clean.as_str());
        }

        /// Text drawn only from the hostile classes: every character must be
        /// replaced, so the output is placeholders and nothing else.
        #[test]
        fn a_pure_payload_becomes_pure_placeholder(
            text in proptest::collection::vec(
                prop_oneof![
                    (0x00u32..=0x1F).prop_filter("newline and tab are kept", |c| *c != 0x0A && *c != 0x09),
                    Just(0x7Fu32),
                    0x80u32..=0x9F,
                    0x200Bu32..=0x200F,
                    0x202Au32..=0x202E,
                    0x2066u32..=0x2069,
                    Just(0x2060u32),
                    Just(0xFEFFu32),
                    Just(0xADu32),
                    Just(0x61Cu32),
                    Just(0x2028u32),
                    Just(0x2029u32),
                    0xE0020u32..=0xE007F,
                    0x13430u32..=0x1343F,
                ].prop_map(|c| char::from_u32(c).expect("all ranges are valid chars")),
                1..20,
            ).prop_map(|chars| chars.into_iter().collect::<String>())
        ) {
            let clean = sanitize(&text);

            prop_assert!(clean.chars().all(|c| c == CONTROL_PLACEHOLDER || c == INVISIBLE_PLACEHOLDER));
            prop_assert_eq!(clean.chars().count(), text.chars().count());
        }
    }
}
