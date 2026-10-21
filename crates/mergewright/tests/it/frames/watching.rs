//! The job pool as the frames harness watches it: the id of every `ShowRows`
//! the loop sends, and every other read's id with the wake it was sent from
//! (ADR-0065 D1; [`super::unasked`]).

use std::sync::{Mutex, PoisonError};

use mergewright::jobs::Dispatch;
use mergewright::update::Command;

use super::unasked::{Ask, AskedFrom, command_id};

/// The job pool, noting what the loop asks of it.
pub(crate) struct Watching<'a, D> {
    pub(crate) inner: &'a D,
    pub(crate) asked: Mutex<Vec<u64>>,
    /// The wake being handled: what a command sent now was asked from.
    pub(crate) origin: Mutex<AskedFrom>,
    /// Every id-carrying read sent, with its origin.
    pub(crate) jobs: Mutex<Vec<Ask>>,
}

impl<'a, D> Watching<'a, D> {
    pub(crate) fn new(inner: &'a D, asked: Vec<u64>) -> Self {
        Watching {
            inner,
            asked: Mutex::new(asked),
            origin: Mutex::default(),
            jobs: Mutex::default(),
        }
    }
}

impl<D: Dispatch> Dispatch for Watching<'_, D> {
    fn dispatch(&self, command: Command) {
        if let Some(id) = command_id(&command) {
            let from = *self.origin.lock().unwrap_or_else(PoisonError::into_inner);
            self.jobs
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(Ask { id, from });
        }
        if let Command::ShowRows(_, request) = &command {
            self.asked
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(request.id);
        }
        self.inner.dispatch(command);
    }
}

impl<D> Watching<'_, D> {
    /// How many `ShowRows` the loop has sent, and the newest id.
    pub(super) fn asked_so_far(&self) -> (usize, Option<u64>) {
        let asked = self.asked.lock().unwrap_or_else(PoisonError::into_inner);
        (asked.len(), asked.last().copied())
    }

    pub(super) fn asked(&self) -> Vec<u64> {
        self.asked
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}
