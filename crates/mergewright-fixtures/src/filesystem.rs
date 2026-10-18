//! Can this filesystem hold a name that is not valid UTF-8?
//!
//! One fixture needs a branch whose name is Latin-1 (`refs/heads/caf\xe9`),
//! and a loose ref is a file. APFS refuses such a name outright, so on a Mac
//! that fixture cannot exist. The ticket's rule is that it must then *skip
//! with a stated reason*, never silently pass — so this module's job is not
//! only to answer yes or no, but to be able to name the filesystem that said
//! no.

use std::fs;
use std::io;
use std::path::Path;
use std::process::Command;

/// Whether non-UTF-8 names work here, and if not, why not.
#[derive(Clone, Debug)]
pub enum Support {
    /// The filesystem accepted a non-UTF-8 name.
    Available,
    /// It did not. The string names the filesystem and what it said.
    Unavailable(String),
}

/// Tries to create, then delete, a file whose name is not valid UTF-8.
///
/// `dir` must already exist and be on the filesystem that will hold the
/// corpus — asking the question anywhere else answers a different question.
#[cfg(unix)]
pub fn non_utf8_names(dir: &Path) -> Result<Support, io::Error> {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    // `\xff` is not a valid UTF-8 byte in any position.
    let path = dir.join(OsStr::from_bytes(b"graft-probe-\xff"));
    match fs::File::create(&path) {
        Ok(_) => {
            fs::remove_file(&path)?;
            Ok(Support::Available)
        }
        Err(why) if why.kind() == io::ErrorKind::InvalidInput || is_illegal_byte_sequence(&why) => {
            Ok(Support::Unavailable(format!(
                "{} rejects file names that are not valid UTF-8 ({why})",
                describe(dir)
            )))
        }
        Err(why) => Err(why),
    }
}

/// On Windows a path is UTF-16; there is no byte sequence to reject.
#[cfg(not(unix))]
pub fn non_utf8_names(_dir: &Path) -> Result<Support, io::Error> {
    Ok(Support::Unavailable(
        "this platform's file names are UTF-16, so a non-UTF-8 name cannot be created".to_owned(),
    ))
}

/// `EILSEQ` is what APFS returns, and it has no `ErrorKind` of its own.
#[cfg(unix)]
fn is_illegal_byte_sequence(why: &io::Error) -> bool {
    /// `EILSEQ` on macOS; Linux filesystems that refuse the name use
    /// `EINVAL`, which `ErrorKind::InvalidInput` already covers.
    const EILSEQ: i32 = 92;
    why.raw_os_error() == Some(EILSEQ)
}

/// The filesystem holding `dir`, as "<type> at <mount point>".
///
/// Best effort by design: if the platform's tools cannot be read, the reason
/// string says so rather than pretending to know. A skip that cannot name the
/// filesystem is still an honest skip; a skip that guesses is not.
fn describe(dir: &Path) -> String {
    match probe_mount(dir) {
        Ok(description) => description,
        Err(why) => format!("this filesystem (its type could not be determined: {why})"),
    }
}

fn probe_mount(dir: &Path) -> Result<String, String> {
    let df = text(Command::new("df").arg("-P").arg(dir))?;
    let row = df
        .lines()
        .nth(1)
        .ok_or_else(|| "`df -P` printed no row".to_owned())?;
    let fields: Vec<&str> = row.split_whitespace().collect();
    let device = *fields
        .first()
        .ok_or_else(|| "`df -P` printed an empty row".to_owned())?;
    let point = *fields
        .last()
        .ok_or_else(|| "`df -P` printed an empty row".to_owned())?;

    // `mount` prints "<device> on <point> (<type>, <options>)" on both macOS
    // and Linux, which is the only part of its output relied on here.
    let mount = text(&mut Command::new("mount"))?;
    let prefix = format!("{device} on {point} (");
    let kind = mount
        .lines()
        .find_map(|line| line.strip_prefix(&prefix)?.split([',', ')']).next())
        .unwrap_or("an unidentified filesystem");
    Ok(format!("{kind} at {point}"))
}

fn text(command: &mut Command) -> Result<String, String> {
    let output = command.output().map_err(|why| {
        format!(
            "`{}` could not be run: {why}",
            command.get_program().display()
        )
    })?;
    if !output.status.success() {
        return Err(format!(
            "`{}` exited {}",
            command.get_program().display(),
            output.status
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}
