//! What a running job hands over before it ends: [`Progress`].

/// What a running job hands over before it ends.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Progress {
    /// The history walk has started, or laid out its next rows.
    History(crate::history::HistoryUpdate),
    /// A search read another chunk of history and found these commits.
    Search(std::sync::Arc<[mergewright_core::oid::Oid]>),
    /// The Map view finished another row's walk (`crate::map`).
    Map {
        /// Rows finished so far.
        done: u32,
        /// Rows the view is building in total.
        total: u32,
    },
    /// A working-tree write started running, described in the words the
    /// journal and the status line both use (`Operation::describe`).
    Operating {
        /// What is running, e.g. `"stage src/a.rs"`.
        description: String,
    },
    /// A fetch, pull or push reported how far it has got: git's own
    /// progress line, shortened (`RemoteProgress::brief`, m4-remotes/05).
    RemoteProgress {
        /// e.g. `"receiving objects 45%"`.
        brief: String,
    },
    /// A fetch, pull or push is waiting on git's credential question
    /// (m4-remotes/06): open the prompt; the job blocks until
    /// [`super::Command::AnswerCredential`] or [`super::Command::CancelCredential`] under
    /// the same id, or until it is cancelled.
    CredentialNeeded {
        /// git's own words, raw — remote-reachable, sanitised before it is
        /// kept (`update::credential`).
        prompt: String,
        /// Where the masked field's pulse starts, drawn fresh at random for
        /// this prompt by `jobs::askpass::pulse_seed`. It is carried on the
        /// event rather than drawn in `update` so `update` stays pure and
        /// its tests can pin it (ADR-0038 §2; see
        /// `mergewright_ui::text_input::SecretField::pulse`).
        pulse_seed: bool,
    },
}
