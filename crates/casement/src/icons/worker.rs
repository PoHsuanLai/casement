//! Icon lookups on a thread of their own.
//!
//! A lookup reads and decodes files (an icon theme can sit on a slow disk or a network folder),
//! so the loop never does it: it asks here, draws a stand-in, and takes the answer the next
//! frame after it comes. One thread serves every question, in order; one stuck lookup leaves the
//! later icons as their stand-ins, and the loop never waits for it.

use casement_icons::{Finder, IconKey, Pixels, SystemTheme, Want};
use std::sync::mpsc::{self, Receiver, Sender};

/// One question for the thread.
#[derive(Debug)]
pub(super) struct Job {
    /// Which question this is, for the answer to be filed under.
    pub key: IconKey,
    /// What to find.
    pub want: Want,
    /// The edge in device pixels.
    pub edge: u32,
    /// The lookup to use: a copy, so a theme changed since does not alter a question asked.
    pub finder: Finder<SystemTheme>,
}

/// The thread's answer to a [`Job`].
#[derive(Debug)]
pub(super) struct Found {
    /// The question's key.
    pub key: IconKey,
    /// The question's edge.
    pub edge: u32,
    /// The icon, or nothing when the app has none the lookup could find.
    pub pixels: Option<Pixels>,
}

/// A question could not be put to the thread (it never started or it is gone).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Refused;

/// The thread and the two ends that talk to it.
#[derive(Debug)]
pub(super) struct Running {
    jobs: Sender<Job>,
    found: Receiver<Found>,
}

impl Running {
    /// Starts the thread. It ends when the questions' end is dropped.
    fn start() -> Option<Running> {
        let (jobs, asked) = mpsc::channel::<Job>();
        let (answer, found) = mpsc::channel::<Found>();
        std::thread::Builder::new()
            .name("casement-icons".to_owned())
            .spawn(move || {
                for job in asked {
                    let pixels = job.finder.find(&job.want, job.edge);
                    let found = Found {
                        key: job.key,
                        edge: job.edge,
                        pixels,
                    };
                    if answer.send(found).is_err() {
                        return;
                    }
                }
            })
            .map_err(|error| tracing::warn!(%error, "icons are not looked for: no thread"))
            .ok()?;
        Some(Running { jobs, found })
    }
}

/// The icon thread: not started until the first question, and not tried again once it failed to
/// start.
#[derive(Debug, Default)]
pub(super) enum Worker {
    /// No question has been asked yet.
    #[default]
    Idle,
    /// Answering.
    Running(Running),
    /// The thread could not be started: every question is refused.
    Broken,
}

impl Worker {
    /// Puts `job` to the thread, starting it first when it is not running yet.
    pub fn submit(&mut self, job: Job) -> Result<(), Refused> {
        if matches!(self, Worker::Idle) {
            *self = Running::start().map_or(Worker::Broken, Worker::Running);
        }
        match self {
            Worker::Running(running) => running.jobs.send(job).map_err(|_| Refused),
            Worker::Idle | Worker::Broken => Err(Refused),
        }
    }

    /// The answers that have come since the last call; never waits.
    pub fn answers(&self) -> Vec<Found> {
        match self {
            Worker::Running(running) => running.found.try_iter().collect(),
            Worker::Idle | Worker::Broken => Vec::new(),
        }
    }
}
