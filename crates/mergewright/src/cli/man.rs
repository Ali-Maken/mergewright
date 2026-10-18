//! `mwr --generate-man`: the man page, in roff, written from the same
//! tables the program runs on (m8-polish-and-ship/23) — the flags from
//! [`FLAGS`](super::FLAGS), the keys from `mergewright_ui::keymap::defaults()`,
//! the config from [`CONFIG_KEYS`](super::CONFIG_KEYS). The committed copy,
//! `docs/man/mwr.1`, is compared with this output by a test.
//!
//! The output is plain ASCII: every other character is written as a roff
//! `\[uXXXX]` escape, which mandoc and groff both read.

use mergewright_ui::keymap::{Binding, ContextId, KeyChord, KeyCode};
use mergewright_ui::theme::Token;

use mergewright_core::on_disk::Generation;

use super::COMMAND;
use super::config_keys::{ConfigKeyDoc, is_pattern};
use super::flags::{Flag, spelled};

/// The whole man page.
#[must_use]
pub fn man_page(flags: &[Flag], keys: &[Binding], config: &[ConfigKeyDoc]) -> String {
    let mut out = String::new();
    let upper = COMMAND.to_ascii_uppercase();
    out.push_str(&format!(
        ".TH {upper} 1 \"\" \"{COMMAND}\" \"User Commands\"\n"
    ));
    section(&mut out, "NAME");
    // The product's name too: `man -k` and `apropos` read this line
    // (usability walk m8, m7).
    out.push_str(&format!(
        "{COMMAND} \\- {}, a terminal Git client for the daily work\n",
        Generation::CURRENT.title()
    ));
    synopsis(&mut out, flags);
    section(&mut out, "DESCRIPTION");
    for (at, paragraph) in DESCRIPTION.iter().enumerate() {
        // `.SH` already starts a paragraph.
        if at > 0 {
            out.push_str(".PP\n");
        }
        text(&mut out, &fill(paragraph));
    }
    options(&mut out, flags);
    key_tables(&mut out, keys);
    configuration(&mut out, config);
    section(&mut out, "ENVIRONMENT");
    tagged(&mut out, ENVIRONMENT);
    section(&mut out, "FILES");
    tagged(&mut out, FILES);
    section(&mut out, "SEE ALSO");
    out.push_str(".BR git (1)\n");
    out
}

const DESCRIPTION: &[&str] = &[
    "{cmd} opens the Git repository at path, or the one the current directory is in, \
     and draws its history as a graph. Staging, committing, branching, merging, \
     rebasing, stashing, fetching, pulling and pushing are done from the keyboard. \
     What it changes in the repository is written to an undo journal, and u undoes \
     the last change.",
    "A bar at the bottom of the screen names the keys that work where you are. \
     Press ? for help on the place you are in, and ctrl+p for the command palette, \
     which finds every action by name.",
    "With no repository, {cmd} shows where you can go instead: a recent repository, \
     a clone, or a practice repository with a short tour.",
];

const ENVIRONMENT: &[(&str, &str)] = &[
    (
        "XDG_CONFIG_HOME",
        "Where the config folder is: $XDG_CONFIG_HOME/{dir}. Ignored unless absolute; \
         unset, $HOME/.config is used.",
    ),
    (
        "XDG_STATE_HOME",
        "Where crash reports go: $XDG_STATE_HOME/{dir}. Unset or empty, \
         $HOME/.local/state is used.",
    ),
    (
        "HOME",
        "The fallback for both of the above. With neither it nor XDG_CONFIG_HOME set, \
         no config file is read, and {cmd} says so.",
    ),
    (
        "NO_COLOR",
        "Set to anything non-empty: no colour is drawn. Shape, weight and glyphs still \
         tell things apart.",
    ),
    (
        "TERM, COLORTERM, TERM_PROGRAM",
        "Read, with TERM_PROGRAM_VERSION, KITTY_WINDOW_ID, TMUX, SSH_TTY, SSH_CONNECTION \
         and the terminal's own answers, to choose the rendering tier and colour depth. \
         graph.renderer overrides the tier.",
    ),
    (
        "MWR_PERF_TRACE",
        "A file name: every drawn frame appends one line to it, for a performance report.",
    ),
    (
        crate::cli::ASKPASS_SOCKET_VAR,
        "Set by {cmd} for the git it runs, so that git can ask {cmd} for a password. \
         Not for setting by hand.",
    ),
];

const FILES: &[(&str, &str)] = &[
    (
        "$XDG_CONFIG_HOME/{dir}/config.toml",
        "The config file; see CONFIGURATION. No file is the normal case.",
    ),
    (
        "$XDG_CONFIG_HOME/{dir}/recent",
        "The repositories opened recently, offered when {cmd} starts outside one.",
    ),
    (
        "$XDG_CONFIG_HOME/{dir}/launches",
        "Launches counted, so the line saying where help is shows on the first five, \
         or until you first open the command palette.",
    ),
    ("$XDG_STATE_HOME/{dir}/", "Crash reports."),
    (
        "<git directory>/{dir}/",
        "Per repository: the undo journal (undo-journal.jsonl), the pane layout, a \
         paused rewrite's plan, the message file ctrl+e opens in your editor, and \
         whether you declined the commit-graph offer.",
    ),
];

/// The tables' `{cmd}` and `{dir}` filled in: the command's name and the
/// config/state folder in force (one source with the code that uses them).
fn fill(words: &str) -> String {
    words
        .replace("{cmd}", COMMAND)
        .replace("{dir}", Generation::CURRENT.dir())
}

fn section(out: &mut String, name: &str) {
    out.push_str(".SH ");
    out.push_str(name);
    out.push('\n');
}

fn synopsis(out: &mut String, flags: &[Flag]) {
    section(out, "SYNOPSIS");
    out.push_str(&format!(".B {COMMAND}\n"));
    for flag in flags {
        // Quoted, or `.RB` would close the gap after the comma.
        out.push_str(&format!(".RB [ \"{}\" ]\n", option_words(flag)));
    }
    out.push_str(".RI [ path ]\n");
}

fn options(out: &mut String, flags: &[Flag]) {
    section(out, "OPTIONS");
    out.push_str(".TP\n.I path\n");
    text(
        out,
        "The repository to open. Default: the current directory. After --, every \
         argument is a path, even one that starts with a dash.",
    );
    for flag in flags {
        out.push_str(".TP\n.B ");
        out.push_str(&option_words(flag));
        out.push('\n');
        text(out, &capitalised(flag.help));
    }
}

/// `-h, --help` with its dashes escaped, so roff draws them as ASCII minus.
fn option_words(flag: &Flag) -> String {
    escape(&spelled(flag)).replace('-', "\\-")
}

fn capitalised(words: &str) -> String {
    let mut chars = words.chars();
    match chars.next() {
        Some(first) => format!("{}{}.", first.to_ascii_uppercase(), chars.as_str()),
        None => String::new(),
    }
}

fn key_tables(out: &mut String, keys: &[Binding]) {
    section(out, "KEYS");
    text(
        out,
        "Every action, by the place it works in. Keys are written the way the [keys] \
         tables of the config file take them; the name after each label is the \
         action's name there. An action listed with no key has none until you give it one.",
    );
    for &context in ContextId::ALL {
        let rows: Vec<&Binding> = keys.iter().filter(|b| b.context == context).collect();
        if rows.is_empty() {
            continue;
        }
        out.push_str(&format!(
            ".SS {} ([keys.{}])\n",
            escape(place_name(context)),
            context.config_name()
        ));
        for binding in rows {
            out.push_str(".TP\n.B ");
            let chords: Vec<String> = binding.default.iter().map(chord_text).collect();
            if chords.is_empty() {
                out.push_str("(none)");
            } else {
                out.push_str(&escape(&chords.join(", ")));
            }
            out.push('\n');
            text(out, &format!("{} ({})", binding.label, binding.action.0));
        }
    }
}

fn configuration(out: &mut String, config: &[ConfigKeyDoc]) {
    section(out, "CONFIGURATION");
    text(
        out,
        &fill(
            "$XDG_CONFIG_HOME/{dir}/config.toml, in TOML. Every key is optional. An unknown \
             key is a warning, not an error; a value a key does not take stops {cmd} with \
             the line and column.",
        ),
    );
    let mut current = "";
    for doc in config {
        if doc.section != current {
            current = doc.section;
            out.push_str(&format!(".SS [{}]\n", escape(current)));
        }
        out.push_str(".TP\n.B ");
        out.push_str(&escape(doc.key));
        out.push('\n');
        text(out, doc.help);
        out.push_str(".br\n");
        text(
            out,
            &format!("Values: {}. Default: {}.", doc.values, doc.default),
        );
        if is_pattern(doc) && doc.section.starts_with("theme") {
            out.push_str(".br\n");
            let names: Vec<&str> = Token::ALL.iter().map(|token| token.name()).collect();
            text(
                out,
                &format!(
                    "Kinds: dark, light, high-contrast. Tokens: {}.",
                    names.join(", ")
                ),
            );
        }
    }
}

fn tagged(out: &mut String, rows: &[(&str, &str)]) {
    for (tag, words) in rows {
        out.push_str(".TP\n.B ");
        out.push_str(&escape(&fill(tag)));
        out.push('\n');
        text(out, &fill(words));
    }
}

/// Input lines roff source keeps under (mandoc's lint asks for 80).
const WIDTH: usize = 80;

/// A paragraph of running text, escaped, wrapped at spaces to [`WIDTH`].
fn text(out: &mut String, words: &str) {
    let mut line = String::new();
    for word in escape(words).split(' ').filter(|word| !word.is_empty()) {
        if !line.is_empty() && line.len() + 1 + word.len() > WIDTH {
            source_line(out, &line);
            line.clear();
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() {
        source_line(out, &line);
    }
}

fn source_line(out: &mut String, line: &str) {
    // A line starting with `.` or `'` would be read as a request.
    if line.starts_with('.') || line.starts_with('\'') {
        out.push_str("\\&");
    }
    out.push_str(line);
    out.push('\n');
}

/// Roff's escapes: a backslash as `\e`, anything past ASCII as `\[uXXXX]`.
fn escape(words: &str) -> String {
    let mut out = String::with_capacity(words.len());
    for c in words.chars() {
        match c {
            '\\' => out.push_str("\\e"),
            c if c.is_ascii() => out.push(c),
            c => out.push_str(&format!("\\[u{:04X}]", u32::from(c))),
        }
    }
    out
}

/// A chord the way `[keys]` takes it: `ctrl+p`, `enter`, `shift+up`, `M`.
pub(crate) fn chord_text(chord: &KeyChord) -> String {
    let mut out = String::new();
    if chord.ctrl {
        out.push_str("ctrl+");
    }
    if chord.alt {
        out.push_str("alt+");
    }
    if chord.shift {
        out.push_str("shift+");
    }
    let key = match chord.code {
        KeyCode::Char(' ') => "space".to_owned(),
        KeyCode::Char(c) => c.to_string(),
        KeyCode::Enter => "enter".to_owned(),
        KeyCode::Escape => "esc".to_owned(),
        KeyCode::Backspace => "backspace".to_owned(),
        KeyCode::Tab => "tab".to_owned(),
        KeyCode::BackTab => "backtab".to_owned(),
        KeyCode::Delete => "delete".to_owned(),
        KeyCode::Insert => "insert".to_owned(),
        KeyCode::Left => "left".to_owned(),
        KeyCode::Right => "right".to_owned(),
        KeyCode::Up => "up".to_owned(),
        KeyCode::Down => "down".to_owned(),
        KeyCode::Home => "home".to_owned(),
        KeyCode::End => "end".to_owned(),
        KeyCode::PageUp => "pgup".to_owned(),
        KeyCode::PageDown => "pgdn".to_owned(),
        KeyCode::Function(n) => format!("f{n}"),
    };
    out.push_str(&key);
    out
}

/// The place's name as a heading, in the keymap doc's words.
fn place_name(context: ContextId) -> &'static str {
    match context {
        ContextId::Global => "Everywhere",
        ContextId::Graph => "Graph",
        ContextId::Map => "Map view",
        // Never "ghost" on anything he reads (`preview::no_user_facing_string_says_ghost`).
        ContextId::Preview => "Preview box",
        ContextId::Confirm => "Confirm box",
        ContextId::Changes => "Working-tree changes",
        ContextId::Diff => "Diff",
        ContextId::Resolver => "Conflict resolver",
        ContextId::Banner => "Operation banner",
        ContextId::Finish => "Finish summary",
        ContextId::Rewrite => "Rewrite editor",
        ContextId::Picker => "Branch picker",
        ContextId::Blame => "Blame",
        ContextId::Stash => "Stash list",
        ContextId::CommitEditor => "Commit message editor",
        ContextId::Journal => "Undo journal",
        ContextId::History => "File history",
        ContextId::Palette => "Command palette",
        ContextId::EmptyState => "Outside a repository",
        ContextId::Tour => "Practice tour",
        ContextId::Credential => "Credential prompt",
        ContextId::Menu => "Context menu",
    }
}
