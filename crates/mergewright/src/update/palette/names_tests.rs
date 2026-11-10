//! No action `?` help or `⌃P` lists names an earlier product
//! (m8-polish-and-ship/50).

use mergewright_core::event::{Event, KeyCode, KeyPress};
use mergewright_git::state::InProgress;
use mergewright_ui::keymap::defaults;

use crate::update::banner::tests::stopped;
use crate::update::palette::rows;
use crate::update::{AppEvent, update};

/// Every earlier product's title and command (pre-publish-cleanup/11b2).
const EARLIER: [(&str, &str); 2] = [("Graft", "graft"), ("Tributary", "trib")];

/// `text` names `word` as a whole word (`_` and alphanumerics bound it).
fn names_word(text: &str, word: &str) -> bool {
    let bytes = text.as_bytes();
    let inside = |byte: u8| byte.is_ascii_alphanumeric() || byte == b'_';
    text.match_indices(word).any(|(at, _)| {
        let before = at.checked_sub(1).map(|i| bytes[i]);
        let after = bytes.get(at + word.len()).copied();
        !before.is_some_and(inside) && !after.is_some_and(inside)
    })
}

/// Every action `?` help and `⌃P` list — each binding's label and search
/// words, and every row `?` shows over a stopped merge — names no earlier
/// generation's title or command (m8-polish-and-ship/50).
#[test]
fn no_action_names_an_old_product() {
    let state = stopped(InProgress::Merge, &["a.rs"]);
    let key = KeyPress::plain(KeyCode::Char('?'));
    let (open, _) = update(&state, AppEvent::Core(Event::Key(key)));
    let palette = open.palette.as_ref().expect("`?` opens the palette");
    let mut words: Vec<String> = Vec::new();
    for (header, group) in rows(&open, palette) {
        words.push(header.as_str().to_owned());
        words.extend(
            group
                .into_iter()
                .map(|(row, _)| row.label.as_str().to_owned()),
        );
    }
    assert!(!words.is_empty(), "`?` lists rows");
    for binding in defaults() {
        words.push(binding.label.to_owned());
        words.extend(binding.search.iter().map(|word| (*word).to_owned()));
    }
    let named: Vec<&String> = words
        .iter()
        .filter(|text| {
            EARLIER
                .iter()
                .any(|(title, command)| names_word(text, title) || names_word(text, command))
        })
        .collect();
    assert!(
        named.is_empty(),
        "an action names an old product: {named:?}"
    );
}
