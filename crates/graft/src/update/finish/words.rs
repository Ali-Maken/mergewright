//! The summary's words: the message split and joined, and what a
//! resolved-by-you row says was chosen.

use super::{Choices, ResultIs, Side, SideNames, Take};

/// A message split the way git splits one: the first line is the subject;
/// the rest, after one blank line if there is one, is the body.
#[must_use]
pub(crate) fn split_message(text: &str) -> (String, String) {
    let text = text.replace("\r\n", "\n");
    match text.split_once('\n') {
        Some((first, rest)) => {
            let rest = rest.strip_prefix('\n').unwrap_or(rest);
            (
                first.trim_end().to_owned(),
                rest.trim_end_matches('\n').to_owned(),
            )
        }
        None => (text.trim_end().to_owned(), String::new()),
    }
}

/// The message the fields hold: the subject, then a blank line and the body
/// when the body is not blank.
#[must_use]
pub(crate) fn compose(subject: &str, body: &str) -> String {
    if body.trim().is_empty() {
        subject.to_owned()
    } else {
        format!("{subject}\n\n{body}")
    }
}

/// What a resolved file's row says about it: what the user chose when that
/// was recorded, else what the result is. First match wins (ticket 19,
/// scope 3).
#[must_use]
pub(crate) fn by_you_words(
    choices: Option<&Choices>,
    result: ResultIs,
    names: &SideNames,
) -> String {
    let name = |side: Side| match side {
        Side::Ours => names.ours.as_str(),
        Side::Theirs => names.theirs.as_str(),
    };
    if let Some(choices) = choices {
        if let Some(side) = choices.whole {
            return if result == ResultIs::Deleted {
                format!("deleted · as {}", name(side))
            } else {
                format!("kept · {}", name(side))
            };
        }
        if choices.edited {
            return "edited in your editor".to_owned();
        }
        let count = |take: Take| {
            choices
                .takes
                .iter()
                .filter(|landed| **landed == Some(take))
                .count()
        };
        let kinds: Vec<(usize, &str)> = [
            (count(Take::Ours), names.ours.as_str()),
            (count(Take::Theirs), names.theirs.as_str()),
            (count(Take::Both), "both"),
        ]
        .into_iter()
        .filter(|(taken, _)| *taken > 0)
        .collect();
        match kinds.as_slice() {
            [] => {}
            [(taken, label)] => {
                return format!("{label} in {taken} of {} conflicts", choices.takes.len());
            }
            _ => {
                return kinds
                    .iter()
                    .map(|(taken, label)| format!("{label} in {taken}"))
                    .collect::<Vec<_>>()
                    .join(", ");
            }
        }
    }
    match result {
        ResultIs::Yours => format!("same as {}", names.ours.as_str()),
        ResultIs::Theirs => format!("same as {}", names.theirs.as_str()),
        ResultIs::Deleted => "deleted".to_owned(),
        ResultIs::Combined => "combined".to_owned(),
    }
}
