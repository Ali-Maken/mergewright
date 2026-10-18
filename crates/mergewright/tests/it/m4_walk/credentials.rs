//! The credential scenarios: git's password question answered, refused
//! and cancelled inside Mergewright, over loopback HTTP with Basic auth.

use mergewright_core::event::KeyCode;
use mergewright_core::journal::OperationKind;
use mergewright_fixtures::Shape;

use super::fixture::{PASSWORD, RemoteKind, USER, WRONG};
use super::steps::{Capture, Scenario, Step, Wait, When};
use super::vocabulary::{
    aborted_cancelled, bare_ref, ch, head, key, last_entry, no_prompt, ours_locally, plain,
    prompt_is, same_bare, sticky_containing,
};
use crate::repo_truth::check;

pub(super) fn credentials() -> Vec<Scenario> {
    vec![
        wrong_password_then_right(),
        password_prompt_esc_cancels(),
        username_then_password(),
    ]
}

fn wrong_password_then_right() -> Scenario {
    Scenario {
        name: "wrong-password-then-right",
        shape: Shape::Linear,
        remote: RemoteKind::Http { anonymous: false },
        setup: ours_locally,
        steps: vec![
            Step::WaitFor(Wait::Ready),
            key(ch('P')),
            Step::Confirm,
            Step::WaitFor(Wait::CredentialPrompt),
            Step::See("the prompt is masked", |state| {
                prompt_is(state, true, "assword")
            }),
            Step::TypeSecret(WRONG),
            key(KeyCode::Enter),
            Step::Settle("the refused push"),
            // The target git names is the whole URL
            // (`http://127.0.0.1:<port>/repo.git/`), not the `host:port` the
            // design note pictured — recorded in the ticket, asserted as
            // built.
            Step::See("authentication failed, with the hint", |state| {
                sticky_containing(
                    state,
                    &["authentication failed for ", "127.0.0.1:", "P to try again"],
                )
            }),
            Step::Mark("refused"),
            Step::WaitFor(Wait::Ready),
            key(ch('P')),
            Step::Confirm,
            Step::WaitFor(Wait::CredentialPrompt),
            Step::TypeSecret(PASSWORD),
            key(KeyCode::Enter),
            Step::Write("the push"),
            Step::Mark("pushed"),
        ],
        expect: |fixture, marks| {
            let refused = marks.get("refused")?;
            same_bare("a wrong password", &marks.before, refused)?;
            let (ok, rejected) = refused.http.ok_or("no HTTP counters")?;
            check(ok == 0 && rejected >= 1, || {
                format!("with the wrong password: {ok} authorised, {rejected} rejected")
            })?;
            let pushed = marks.get("pushed")?;
            let tip = head(&marks.before)?;
            check(bare_ref(pushed, &fixture.main_ref())? == tip, || {
                "the right password did not push".to_owned()
            })?;
            let (ok, rejected) = pushed.http.ok_or("no HTTP counters")?;
            check(ok >= 1 && rejected >= 1, || {
                format!("in all: {ok} authorised, {rejected} rejected")
            })
        },
        capture: Some(Capture {
            stem: "m4-03-credential-masked",
            headless: Vec::new(),
            keys: plain(&[ch('P'), KeyCode::Right, KeyCode::Enter]),
            secret: Some(PASSWORD),
            on_screen: "Password for",
            when: When::Settled,
        }),
    }
}

fn password_prompt_esc_cancels() -> Scenario {
    Scenario {
        name: "password-prompt-esc-cancels",
        shape: Shape::Linear,
        remote: RemoteKind::Http { anonymous: false },
        setup: ours_locally,
        steps: vec![
            Step::WaitFor(Wait::Ready),
            key(ch('P')),
            Step::Confirm,
            Step::WaitFor(Wait::CredentialPrompt),
            key(KeyCode::Escape),
            Step::See("Esc closed the prompt", no_prompt),
            Step::Settle("the cancelled push"),
            Step::Mark("cancelled"),
        ],
        expect: |_, marks| {
            let cancelled = marks.get("cancelled")?;
            same_bare("a cancelled push", &marks.before, cancelled)?;
            let entry = last_entry(cancelled)?;
            check(entry.kind == OperationKind::Push, || {
                format!("the last entry is {:?}", entry.kind)
            })?;
            aborted_cancelled(entry)?;
            let (ok, _) = cancelled.http.ok_or("no HTTP counters")?;
            check(ok == 0, || format!("{ok} requests were authorised"))
        },
        capture: None,
    }
}

fn username_then_password() -> Scenario {
    Scenario {
        name: "username-then-password",
        shape: Shape::Linear,
        remote: RemoteKind::Http { anonymous: true },
        setup: ours_locally,
        steps: vec![
            Step::WaitFor(Wait::Ready),
            key(ch('P')),
            Step::Confirm,
            Step::WaitFor(Wait::CredentialPrompt),
            Step::See("the username is asked in the open", |state| {
                prompt_is(state, false, "sername")
            }),
            Step::Type(USER),
            key(KeyCode::Enter),
            Step::WaitFor(Wait::CredentialPrompt),
            Step::See("the password is asked masked", |state| {
                prompt_is(state, true, "assword")
            }),
            Step::TypeSecret(PASSWORD),
            key(KeyCode::Enter),
            Step::Write("the push"),
            Step::Mark("pushed"),
        ],
        expect: |fixture, marks| {
            let pushed = marks.get("pushed")?;
            let tip = head(&marks.before)?;
            check(bare_ref(pushed, &fixture.main_ref())? == tip, || {
                "the push did not land".to_owned()
            })
        },
        capture: Some(Capture {
            stem: "m4-04-credential-username",
            headless: Vec::new(),
            keys: plain(&[
                ch('P'),
                KeyCode::Right,
                KeyCode::Enter,
                ch('t'),
                ch('e'),
                ch('s'),
                ch('t'),
                ch('e'),
                ch('r'),
            ]),
            secret: None,
            on_screen: "Username for",
            when: When::Settled,
        }),
    }
}
