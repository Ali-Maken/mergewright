//! [`MergeState`]: the merge git has in progress, as the journal records it
//! (m6-conflicts/20), and the byte-exact JSON form its files take.

use serde::{Deserialize, Serialize};

use crate::oid::Oid;

/// The merge git has in progress, as the files that make it one: what
/// `$GIT_DIR/MERGE_HEAD`, `MERGE_MSG` and `MERGE_MODE` held, byte for byte
/// (m6-conflicts/20). Writing these three back is what puts a finished merge
/// back in progress: `git status` says merging again and `git commit`
/// finishes it again.
///
/// **`AUTO_MERGE` is left out, deliberately.** It names a *tree* — git's
/// own merge result with the conflict markers in it — and only `git diff
/// AUTO_MERGE` reads it; `git status`, `git commit`, `git merge --continue`
/// and `git merge --abort` never do. Keeping it would mean keeping a tree
/// alive past `git gc`, and a tree cannot be a backup commit's parent: it
/// would take a new subtree in the backup commit's layout, which every
/// reader since M3 relies on. The cost of leaving it out: after `u`, `git
/// diff AUTO_MERGE` says the name is unknown.
///
/// On disk each byte field is a JSON string when it is UTF-8 and `{"hex":
/// "…"}` when it is not, so a message in any encoding round-trips exactly.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MergeState {
    /// `MERGE_HEAD`'s first line: the commit being merged in — the first of
    /// them for an octopus merge.
    pub merge_head: Oid,
    /// `MERGE_HEAD`'s further lines, in order: the other commits of an
    /// octopus merge (`git merge a b`), empty for an ordinary one
    /// (m6-conflicts/38). Left out of the journal line when empty, so every
    /// entry written before it reads back unchanged.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub other_heads: Vec<Oid>,
    /// `MERGE_MSG`'s exact bytes. Empty when git's file was missing, which
    /// happens only when it was removed by hand.
    #[serde(with = "text_bytes")]
    pub merge_msg: Vec<u8>,
    /// `MERGE_MODE`'s bytes when the file exists (`None` = no file,
    /// `Some(vec![])` = an empty one; git writes `no-ff` for `--no-ff`).
    #[serde(with = "text_bytes::option")]
    pub merge_mode: Option<Vec<u8>>,
}

impl MergeState {
    /// Every commit `MERGE_HEAD` names, in its order: `merge_head` first.
    pub fn heads(&self) -> impl Iterator<Item = Oid> + '_ {
        std::iter::once(self.merge_head).chain(self.other_heads.iter().copied())
    }

    /// `MERGE_HEAD` as git writes it: each commit's full hex id and a
    /// newline, one line per head. Byte-exact for any file git wrote; a
    /// hand-edited one (blank lines, trailing spaces) comes back in git's
    /// form.
    pub fn merge_head_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        for head in self.heads() {
            bytes.extend_from_slice(head.to_string().as_bytes());
            bytes.push(b'\n');
        }
        bytes
    }
}

/// Bytes as JSON: a plain string when they are UTF-8 (the everyday case,
/// readable in the journal file), `{"hex":"…"}` when they are not.
mod text_bytes {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    #[derive(Serialize, Deserialize)]
    #[serde(untagged)]
    enum Repr {
        Text(String),
        Hex { hex: String },
    }

    impl Repr {
        fn of(bytes: &[u8]) -> Self {
            match std::str::from_utf8(bytes) {
                Ok(text) => Repr::Text(text.to_owned()),
                Err(_) => Repr::Hex {
                    hex: bytes
                        .iter()
                        .flat_map(|byte| [byte >> 4, byte & 0x0f])
                        // A nibble is below 16, so `from_digit` always
                        // answers: nothing is dropped here.
                        .filter_map(|nibble| char::from_digit(u32::from(nibble), 16))
                        .collect(),
                },
            }
        }

        fn into_bytes<E: serde::de::Error>(self) -> Result<Vec<u8>, E> {
            match self {
                Repr::Text(text) => Ok(text.into_bytes()),
                Repr::Hex { hex } => {
                    if hex.len() % 2 != 0 {
                        return Err(E::custom("hex bytes of odd length"));
                    }
                    (0..hex.len())
                        .step_by(2)
                        .map(|at| {
                            hex.get(at..at + 2)
                                .and_then(|pair| u8::from_str_radix(pair, 16).ok())
                                .ok_or_else(|| E::custom("hex bytes that are not hex"))
                        })
                        .collect()
                }
            }
        }
    }

    pub(super) fn serialize<S: Serializer>(bytes: &[u8], to: S) -> Result<S::Ok, S::Error> {
        Repr::of(bytes).serialize(to)
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(from: D) -> Result<Vec<u8>, D::Error> {
        Repr::deserialize(from)?.into_bytes()
    }

    pub(super) mod option {
        use serde::{Deserialize, Deserializer, Serialize, Serializer};

        use super::Repr;

        // `serde(with)` hands over `&Option<Vec<u8>>`; the signature is
        // serde's, not a choice.
        #[allow(clippy::ref_option)]
        pub(in super::super) fn serialize<S: Serializer>(
            bytes: &Option<Vec<u8>>,
            to: S,
        ) -> Result<S::Ok, S::Error> {
            bytes.as_deref().map(Repr::of).serialize(to)
        }

        pub(in super::super) fn deserialize<'de, D: Deserializer<'de>>(
            from: D,
        ) -> Result<Option<Vec<u8>>, D::Error> {
            Option::<Repr>::deserialize(from)?
                .map(Repr::into_bytes)
                .transpose()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A UTF-8 message stays readable text in the file; a message in
    /// another encoding goes as hex and comes back byte for byte.
    #[test]
    fn merge_state_bytes_are_text_when_they_can_be_and_exact_always() {
        let head = Oid::from_hex("acf15d58a3b48e5878fe281a624fb5c688dcaee1").expect("a hex oid");
        let text = MergeState {
            merge_head: head,
            other_heads: Vec::new(),
            merge_msg: b"Merge branch 'x'\n".to_vec(),
            merge_mode: Some(Vec::new()),
        };
        let json = serde_json::to_string(&text).expect("it renders");
        assert_eq!(
            json,
            r#"{"merge_head":"acf15d58a3b48e5878fe281a624fb5c688dcaee1","merge_msg":"Merge branch 'x'\n","merge_mode":""}"#
        );
        assert_eq!(
            serde_json::from_str::<MergeState>(&json).expect("it parses"),
            text
        );

        let latin1 = MergeState {
            merge_head: head,
            other_heads: Vec::new(),
            merge_msg: b"caf\xe9\n".to_vec(),
            merge_mode: None,
        };
        let json = serde_json::to_string(&latin1).expect("it renders");
        assert!(
            json.contains(r#""merge_msg":{"hex":"636166e90a"}"#),
            "{json}"
        );
        assert!(json.contains(r#""merge_mode":null"#), "{json}");
        assert_eq!(
            serde_json::from_str::<MergeState>(&json).expect("it parses"),
            latin1
        );
        assert!(serde_json::from_str::<MergeState>(&json.replace("636166e90a", "6361f")).is_err());
    }

    /// An octopus merge's further heads go in `other_heads`, in order, and
    /// `MERGE_HEAD` comes back as git wrote it; a line without the field
    /// (every entry before m6-conflicts/38) reads as one head.
    #[test]
    fn an_octopus_merge_state_keeps_every_head_in_order() {
        let first = Oid::from_hex("acf15d58a3b48e5878fe281a624fb5c688dcaee1").expect("a hex oid");
        let second = Oid::from_hex("0123456789abcdef0123456789abcdef01234567").expect("a hex oid");
        let octopus = MergeState {
            merge_head: first,
            other_heads: vec![second],
            merge_msg: b"Merge branches 'a' and 'b'\n".to_vec(),
            merge_mode: Some(b"no-ff".to_vec()),
        };
        assert_eq!(octopus.heads().collect::<Vec<_>>(), vec![first, second]);
        assert_eq!(
            octopus.merge_head_bytes(),
            format!("{first}\n{second}\n").into_bytes()
        );
        let json = serde_json::to_string(&octopus).expect("it renders");
        assert!(
            json.contains(r#""other_heads":["0123456789abcdef0123456789abcdef01234567"]"#),
            "{json}"
        );
        assert_eq!(
            serde_json::from_str::<MergeState>(&json).expect("it parses"),
            octopus
        );

        let old = r#"{"merge_head":"acf15d58a3b48e5878fe281a624fb5c688dcaee1","merge_msg":"m\n","merge_mode":null}"#;
        let read = serde_json::from_str::<MergeState>(old).expect("an old line parses");
        assert!(read.other_heads.is_empty());
        assert_eq!(serde_json::to_string(&read).expect("it renders"), old);
    }
}
