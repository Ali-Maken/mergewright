//! The one table of flags (m8-polish-and-ship/23): `--help`, the man page and
//! the README's flag list are all read off [`FLAGS`], so none of them can
//! name a flag `parse` does not take, or miss one it does.

/// One flag the binary takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Flag {
    /// The long spelling, without its dashes (`help`).
    pub long: &'static str,
    /// The one-letter spelling, if there is one (`h`).
    pub short: Option<char>,
    /// The value it takes, as `--help` names it (`<file>`); `None` for a
    /// switch.
    pub value: Option<&'static str>,
    /// What it does, in the words `--help` prints.
    pub help: &'static str,
    /// Left out of `--help` (still in the man page): a flag for packagers,
    /// not for daily use.
    pub hidden: bool,
}

/// Every flag `parse` accepts, in the order `--help` lists them.
pub const FLAGS: &[Flag] = &[
    Flag {
        long: "help",
        short: Some('h'),
        value: None,
        help: "print this and exit",
        hidden: false,
    },
    Flag {
        long: "generate-man",
        short: None,
        value: None,
        help: "print the man page (roff) to stdout and exit",
        hidden: true,
    },
];

/// What a flag asks for, once `parse` has found its row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FlagAction {
    /// `--help`.
    Usage,
    /// `--generate-man`.
    GenerateMan,
}

impl FlagAction {
    /// Every action `parse` can take on a flag; the test that pairs these
    /// with [`FLAGS`] rows both ways reads this.
    #[cfg(test)]
    pub(crate) const ALL: &'static [FlagAction] = &[FlagAction::Usage, FlagAction::GenerateMan];

    /// The action a row's long name selects; `None` for a row `parse` has no
    /// action for, which the both-ways test refuses.
    pub(crate) fn of(long: &str) -> Option<FlagAction> {
        match long {
            "help" => Some(FlagAction::Usage),
            "generate-man" => Some(FlagAction::GenerateMan),
            _ => None,
        }
    }
}

/// The row an argument spells — `--<long>` or `-<short>` — if any.
pub(crate) fn find<'a>(flags: &'a [Flag], argument: &str) -> Option<&'a Flag> {
    if let Some(long) = argument.strip_prefix("--") {
        return flags.iter().find(|flag| flag.long == long);
    }
    let short = argument.strip_prefix('-')?;
    let mut letters = short.chars();
    let (Some(letter), None) = (letters.next(), letters.next()) else {
        return None;
    };
    flags.iter().find(|flag| flag.short == Some(letter))
}

/// A flag as `--help` and the man page's synopsis write it: `-h, --help`,
/// `--generate-man`, `--config <file>`.
pub(crate) fn spelled(flag: &Flag) -> String {
    let mut out = String::new();
    if let Some(short) = flag.short {
        out.push('-');
        out.push(short);
        out.push_str(", ");
    }
    out.push_str("--");
    out.push_str(flag.long);
    if let Some(value) = flag.value {
        out.push(' ');
        out.push_str(value);
    }
    out
}

/// What `--help` prints: the synopsis, the path, then every flag not
/// [`Flag::hidden`], its help aligned four spaces past the widest.
#[must_use]
pub fn usage(flags: &[Flag]) -> String {
    let shown: Vec<(String, &str)> = flags
        .iter()
        .filter(|flag| !flag.hidden)
        .map(|flag| (spelled(flag), flag.help))
        .collect();
    let width = shown.iter().map(|(left, _)| left.len()).max().unwrap_or(0);

    let mut out = String::from(
        "usage: graft [<path>]\n\n  <path>    the repository to open (default: the current directory)",
    );
    if !shown.is_empty() {
        out.push('\n');
    }
    for (left, help) in shown {
        out.push_str(&format!("\n  {left:<width$}    {help}"));
    }
    out
}
