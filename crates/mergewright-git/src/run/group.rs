//! Who is in a git child's process group (m8-polish-and-ship/13c).
//!
//! Every child [`super::GitInvocation`] spawns leads its own process group,
//! so everything git starts — hooks, their children, filters, gpg — is in
//! it unless it moved itself out. A hook-first stop
//! ([`super::GitInvocation::stop_hooks_first`]) needs the members by pid, to
//! end git's helpers while leaving git itself to pause the operation.
//!
//! There is no portable call for this. Linux has `/proc`; macOS has no
//! `/proc`, and `ps` is the only way to the table without `unsafe`
//! (`proc_listpids` is libc-only, and this crate forbids `unsafe`). Any other
//! target says it cannot, and the caller falls back to stopping the whole
//! group.

use std::io;

use nix::unistd::Pid;

/// Every live process whose process group is `pgid`, the leader included.
///
/// # Errors
/// The table could not be read or did not parse, or this target has no way
/// to read it. The caller then stops the group as a whole.
pub(crate) fn members(pgid: Pid) -> io::Result<Vec<Pid>> {
    imp::members(pgid)
}

#[cfg(target_os = "linux")]
mod imp {
    use std::io;

    use nix::unistd::Pid;

    /// `/proc/<pid>/stat` field 5 is the process group.
    pub(super) fn members(pgid: Pid) -> io::Result<Vec<Pid>> {
        let mut found = Vec::new();
        for entry in std::fs::read_dir("/proc")? {
            let entry = entry?;
            let Some(pid) = entry
                .file_name()
                .to_str()
                .and_then(|name| name.parse::<i32>().ok())
            else {
                continue; // not a process directory (`self`, `meminfo`, …)
            };
            let stat = match std::fs::read_to_string(entry.path().join("stat")) {
                Ok(stat) => stat,
                // Safe to skip: the process ended between the listing and
                // the read, so it is no member any more.
                Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
                Err(error) => return Err(error),
            };
            if group_of(&stat)? == pgid.as_raw() {
                found.push(Pid::from_raw(pid));
            }
        }
        Ok(found)
    }

    /// `pid (comm) state ppid pgrp …`: `comm` may hold spaces and
    /// parentheses, so the fields are counted from the last `)`.
    fn group_of(stat: &str) -> io::Result<i32> {
        stat.rfind(')')
            .and_then(|end| stat[end + 1..].split_whitespace().nth(2))
            .and_then(|field| field.parse().ok())
            .ok_or_else(|| io::Error::other(format!("unparsable /proc stat: {stat}")))
    }
}

#[cfg(target_os = "macos")]
mod imp {
    use std::io;
    use std::process::{Command, Stdio};

    use nix::unistd::Pid;

    /// `ps -A -o pid= -o pgid=`: every process, two numbers a line.
    pub(super) fn members(pgid: Pid) -> io::Result<Vec<Pid>> {
        let output = Command::new("/bin/ps")
            .args(["-A", "-o", "pid=", "-o", "pgid="])
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()?;
        if !output.status.success() {
            return Err(io::Error::other(format!("ps failed: {}", output.status)));
        }
        let table = String::from_utf8(output.stdout)
            .map_err(|_| io::Error::other("ps printed something other than UTF-8"))?;
        parse(&table, pgid)
    }

    pub(super) fn parse(table: &str, wanted: Pid) -> io::Result<Vec<Pid>> {
        let mut found = Vec::new();
        for line in table.lines().filter(|line| !line.trim().is_empty()) {
            let mut fields = line.split_whitespace().map(str::parse::<i32>);
            match (fields.next(), fields.next(), fields.next()) {
                (Some(Ok(pid)), Some(Ok(group)), None) => {
                    if group == wanted.as_raw() {
                        found.push(Pid::from_raw(pid));
                    }
                }
                _ => return Err(io::Error::other(format!("unparsable ps line: {line}"))),
            }
        }
        Ok(found)
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
mod imp {
    use std::io;

    use nix::unistd::Pid;

    pub(super) fn members(_pgid: Pid) -> io::Result<Vec<Pid>> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "no process table reader for this target",
        ))
    }
}

#[cfg(test)]
mod tests {
    use std::os::unix::process::CommandExt;
    use std::process::{Command, Stdio};

    use super::*;

    #[test]
    fn members_lists_a_child_in_the_group() {
        let mut child = Command::new("sleep")
            .arg("30")
            .process_group(0)
            .stdin(Stdio::null())
            .spawn()
            .expect("sleep starts");
        let pid = Pid::from_raw(i32::try_from(child.id()).expect("a pid fits i32"));

        let listed = members(pid);

        // Our own child, still unreaped: its pid is its own.
        child.kill().expect("our own child can be killed");
        child.wait().expect("and reaped");
        let listed = listed.expect("the table reads on this host");
        assert_eq!(listed, vec![pid], "the leader alone is in its new group");
        let ours = nix::unistd::getpgrp();
        assert!(
            !members(ours).expect("reads").is_empty(),
            "our own group lists at least this test process"
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn a_ps_line_that_does_not_parse_is_an_error_not_a_skip() {
        let group = Pid::from_raw(7);
        assert_eq!(
            imp::parse("  7     7\n  8     7\n  9     1\n", group).expect("parses"),
            vec![Pid::from_raw(7), Pid::from_raw(8)]
        );
        assert!(imp::parse("7 seven\n", group).is_err());
    }
}
