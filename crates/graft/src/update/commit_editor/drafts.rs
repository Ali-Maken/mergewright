//! The words kept between screens (C15, 08s M1): one slot per way in, so a
//! peek at the last commit with `a` never costs the words typed for `c`,
//! and an empty `c` never costs an amend's.

use graft_core::oid::Oid;

use super::LastCommit;

/// How the screen was opened. Fixed for the screen's life: turning the
/// amend option on or off later changes what `⏎` writes, never which slot
/// `esc` keeps the words in nor whether the index is taken as it is
/// ([`super::include_of`]).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum WayIn {
    /// `c`: a new commit.
    #[default]
    Commit,
    /// `a`: an amend of `HEAD`, prefilled with its message.
    Amend,
}

/// The words `esc` keeps from the message line (C15), and the message `u`
/// gives back after a commit (Q9 a).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Draft {
    /// The subject line.
    pub subject: String,
    /// The body.
    pub body: String,
    /// `Some(head)` when kept from an amend of `head`: `a` shows them on
    /// any `HEAD`, saying which commit they were typed for when it is not
    /// this one.
    pub amending: Option<Oid>,
}

impl Draft {
    /// The words as one message: subject, then the body after a blank line.
    fn text(&self) -> String {
        if self.body.is_empty() {
            self.subject.clone()
        } else {
            format!("{}\n\n{}", self.subject, self.body)
        }
    }
}

/// The two slots, each taken by its own way in on open and written by its
/// own way in on close (08s M1). Both boxed: `AppState` carries two words
/// apiece while nothing is kept.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Drafts {
    /// `c`'s words, and where `u` gives a commit's message back.
    pub commit: Option<Box<Draft>>,
    /// `a`'s words, kept only when they differ from `HEAD`'s message.
    pub amend: Option<Box<Draft>>,
    /// The commit whose message `u` last gave back, until a redo or the
    /// next give: a redo re-creating it takes the words back only while
    /// they are still exactly what was given (08s m1).
    pub given_back: Option<Box<LastCommit>>,
}

/// `u` took back the commit `last` made: its words go to the `c` slot.
/// Words already there stay first; the commit's subject and body follow in
/// the body after a blank line, so nothing typed is replaced (08s Q9).
pub fn give_back(drafts: &mut Drafts, words: Draft) {
    let returned = Draft {
        amending: None,
        ..words
    };
    let merged = match drafts.commit.take() {
        Some(kept) if !(kept.subject.is_empty() && kept.body.is_empty()) => {
            let body = if kept.body.is_empty() {
                returned.text()
            } else {
                format!("{}\n\n{}", kept.body, returned.text())
            };
            Draft {
                subject: kept.subject,
                body,
                amending: None,
            }
        }
        _ => returned,
    };
    drafts.commit = Some(Box::new(merged));
}

/// A redo re-created `commit`, whose message `u` had given back: the `c`
/// slot lets go of the words only when they are still exactly the commit's
/// (08s m1); edited, they are his.
pub fn redo_took_back(drafts: &mut Drafts, commit: Oid) {
    let Some(given) = drafts.given_back.take_if(|given| given.commit == commit) else {
        return;
    };
    if drafts.commit.as_deref() == Some(&given.message) {
        drafts.commit = None;
    }
}
