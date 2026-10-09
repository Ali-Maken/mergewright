//! Who would author a ghost: the repository's own identity, as git would stamp it now.

#[allow(
    clippy::wildcard_imports,
    reason = "one module split in files: its parent is its prelude"
)]
use super::*;

/// The repository's own user, as git would stamp a commit it makes now:
/// one `git var GIT_AUTHOR_IDENT` spawn, which is exactly the value the
/// merge or revert commit would carry.
///
/// A repository where this fails is one where the commit would fail too
/// ("Please tell me who you are"), so the preview fails the same way rather
/// than drawing a row with a made-up name on it.
pub(super) fn identity_now(repo: &RepoHandle, stop: &dyn Stop) -> Result<Identity, PreviewError> {
    let output = GitInvocation::new(repo)
        .args([
            GitArg::Flag("var"),
            GitArg::Word("GIT_AUTHOR_IDENT".to_owned()),
        ])
        .run(stop)?;
    parse_ident(&output.stdout).ok_or_else(|| {
        PreviewError::Read(RunError::Unparsable(
            "git var GIT_AUTHOR_IDENT did not give a name, an email and a time".to_owned(),
        ))
    })
}

/// `A U Thor <a@example.com> 1758377000 +0500` — name, email, then two
/// fields from the right. Bytes, never decoded: a name git stores in
/// Latin-1 is still that person's name.
pub(super) fn parse_ident(line: &[u8]) -> Option<Identity> {
    let line = line.split(|&byte| byte == b'\n').next()?;
    let mut fields = line.rsplitn(3, |&byte| byte == b' ');
    let offset = std::str::from_utf8(fields.next()?).ok()?;
    let seconds: i64 = std::str::from_utf8(fields.next()?).ok()?.parse().ok()?;
    let rest = fields.next()?;
    let open = rest.iter().rposition(|&byte| byte == b'<')?;
    let close = rest.iter().rposition(|&byte| byte == b'>')?;
    if close < open {
        return None;
    }
    let name = rest[..open].strip_suffix(b" ").unwrap_or(&rest[..open]);
    Some(Identity {
        name: name.to_vec(),
        email: rest[open + 1..close].to_vec(),
        when: Timestamp {
            seconds,
            offset_seconds: parse_offset(offset)?,
        },
    })
}

/// `+0530` → 19800 seconds east.
pub(super) fn parse_offset(text: &str) -> Option<i32> {
    let (sign, digits) = text.split_at_checked(1)?;
    let sign = match sign {
        "+" => 1,
        "-" => -1,
        _ => return None,
    };
    if digits.len() != 4 {
        return None;
    }
    let hours: i32 = digits[..2].parse().ok()?;
    let minutes: i32 = digits[2..].parse().ok()?;
    Some(sign * (hours * 3600 + minutes * 60))
}
