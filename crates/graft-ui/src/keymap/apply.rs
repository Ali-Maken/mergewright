//! One override line from his config, checked and applied.

use super::resolve::{FOLLOWS, QUIT_NOW_ID, SUSPEND_ID, index_of};
use super::{KeyChord, KeySwitches, KeyWarning, Keymap, defaults};

impl Keymap {
    /// One override line, applied, or the reason it isn't.
    pub(super) fn apply(
        &mut self,
        at: usize,
        name: &str,
        texts: &[String],
        switches: KeySwitches,
    ) -> Result<(), KeyWarning> {
        let Some(row) = index_of(name) else {
            return Err(KeyWarning::UnknownAction(name.to_string()));
        };
        let binding = &defaults()[row];
        if binding.context.is_text_field() {
            return Err(KeyWarning::Fixed(binding.action));
        }
        if let Some((_, leader)) = FOLLOWS.iter().find(|(f, _)| *f == binding.action) {
            return Err(KeyWarning::Follows {
                action: binding.action,
                leader: *leader,
            });
        }
        if binding.action == SUSPEND_ID && !switches.suspend_on_ctrl_z {
            return Err(KeyWarning::SwitchOff {
                action: binding.action,
                switch: "suspend_on_ctrl_z",
            });
        }
        let chords = parse_list(name, texts)?;
        if binding.action == QUIT_NOW_ID {
            return Err(KeyWarning::Reserved {
                action: name.to_string(),
                key: chords.first().copied().unwrap_or(KeyChord::QUIT_NOW),
            });
        }
        if chords.contains(&KeyChord::QUIT_NOW) {
            return Err(KeyWarning::Reserved {
                action: name.to_string(),
                key: KeyChord::QUIT_NOW,
            });
        }
        self.chords[row] = chords;
        self.line[row] = Some(at);
        Ok(())
    }
}

/// His chord list: `["none"]` alone is no chord; `none` inside a longer
/// list, or text naming no key, is a [`KeyWarning::BadKey`].
fn parse_list(action: &str, texts: &[String]) -> Result<Vec<KeyChord>, KeyWarning> {
    let bad = |text: &str| KeyWarning::BadKey {
        action: action.to_string(),
        text: text.to_string(),
    };
    match texts {
        [] => Err(bad("")),
        [only] if only.trim().eq_ignore_ascii_case("none") => Ok(Vec::new()),
        _ => {
            let mut out = Vec::new();
            for text in texts {
                if text.trim().eq_ignore_ascii_case("none") {
                    return Err(bad(text));
                }
                let chord = KeyChord::parse(text).map_err(|e| bad(&e.text))?;
                if !out.contains(&chord) {
                    out.push(chord);
                }
            }
            Ok(out)
        }
    }
}
