//! `org.quire.Casement.Debug1` in the event loop (feature `debug-inject`, the jail's private
//! bus only): the D-Bus object forwards every call down a calloop channel and this module
//! answers it with the `State` in hand. A call that needs time to pass (a wait, a sleep, a
//! clock step) becomes a job the loop works on each turn; one that does not is answered at
//! once.

mod dump;
mod inject;
mod png;
mod shot;

use crate::State;
use crate::time::ClockSource;
use casement_core::time::Mono;
use casement_ipc::dbus::{DebugCall, DebugReply, DebugRequest};
use casement_ipc::{InjectLine, WaitPredicate};
use inject::Injector;
use smithay::reexports::calloop::{Error as LoopError, LoopHandle};
use smithay::reexports::calloop::channel::{Channel, Event};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::time::{Duration, Instant};

/// How long the loop sleeps between turns while a job waits on real time.
pub const WAITING_TURN: Duration = Duration::from_millis(5);

/// Sends the answer back to the D-Bus call.
type Answer = Box<dyn FnOnce(DebugReply)>;

/// What the loop is working on for a call.
enum Job {
    /// Walk the stepped clock to this instant.
    Advance { until: Mono, answer: Answer },
    /// Walk the stepped clock until `output` (the first when none) has presented `goal` frames.
    Frames { goal: u64, answer: Answer },
    /// Wait for a predicate: virtual time moves it along for `frames`, real time bounds it.
    Wait {
        predicate: WaitPredicate,
        frames_goal: u64,
        deadline: Instant,
        answer: Answer,
    },
    /// Inject lines, pausing at each `sleep`.
    Script {
        lines: VecDeque<InjectLine>,
        answers: Vec<String>,
        resume_at: Option<Mono>,
        answer: Answer,
    },
    /// Panic once this many frames were presented (a crash for the supervisor's tests).
    PanicAt { goal: u64 },
}

/// The debug server's state: the jobs in order and the virtual devices it injects through.
pub struct Debug {
    jobs: VecDeque<Job>,
    injector: Injector,
}

/// The server, shared by the channel's callback and the loop's turn.
pub type Shared = Rc<RefCell<Debug>>;

/// Starts answering `calls` on `handle`'s loop.
pub fn attach(
    handle: &LoopHandle<'static, State>,
    calls: Channel<DebugCall>,
) -> Result<Shared, LoopError> {
    let shared: Shared = Rc::new(RefCell::new(Debug {
        jobs: VecDeque::new(),
        injector: Injector::default(),
    }));
    let owner = shared.clone();
    handle
        .insert_source(calls, move |event, (), state: &mut State| {
            if let Event::Msg(call) = event {
                owner.borrow_mut().receive(state, call);
            }
        })
        .map_err(LoopError::from)?;
    Ok(shared)
}

/// What a loop turn does for the server: work the job at the front.
pub fn turn(shared: &Shared, state: &mut State) {
    shared.borrow_mut().work(state);
}

impl Debug {
    /// Takes a call: answered now, or queued as a job.
    fn receive(&mut self, state: &mut State, call: DebugCall) {
        let DebugCall { request, reply } = call;
        let answer: Answer = Box::new(move |answer| {
            let _ = reply.send(answer);
        });
        match request {
            DebugRequest::Inject(lines) => self.jobs.push_back(Job::Script {
                lines: lines.into(),
                answers: Vec::new(),
                resume_at: None,
                answer,
            }),
            DebugRequest::Wait(predicate, timeout) => {
                let frames_goal = dump::frames_presented(state, frames_output(&predicate))
                    .saturating_add(frames_of(&predicate));
                self.jobs.push_back(Job::Wait {
                    predicate,
                    frames_goal,
                    deadline: state.clock.wall_instant() + timeout,
                    answer,
                });
            }
            DebugRequest::Step(n) => match virtual_clock(state) {
                Ok(()) => {
                    let goal = dump::frames_presented(state, None).saturating_add(u64::from(n));
                    self.jobs.push_back(Job::Frames { goal, answer });
                }
                Err(why) => answer(DebugReply::Refused(why)),
            },
            DebugRequest::Advance(by) => match virtual_clock(state) {
                Ok(()) => {
                    let until = state.clock.now().after(by);
                    self.jobs.push_back(Job::Advance { until, answer });
                }
                Err(why) => answer(DebugReply::Refused(why)),
            },
            DebugRequest::Output(verb, args) => answer(dump::output(state, &verb, &args)),
            DebugRequest::Windows => answer(DebugReply::Json(dump::windows(state))),
            DebugRequest::Scene => answer(DebugReply::Json(dump::scene(state))),
            DebugRequest::Shot(area, path) => answer(shot::shot(state, &area, &path)),
            DebugRequest::Fault(verb, amount) => match verb.as_str() {
                "drop-output" => answer(dump::drop_output(state, amount)),
                "panic-after-frames" => {
                    let goal =
                        dump::frames_presented(state, None).saturating_add(u64::from(amount));
                    self.jobs.push_back(Job::PanicAt { goal });
                    answer(DebugReply::Done);
                }
                other => answer(DebugReply::Refused(format!(
                    "fault {other} is not supported at P0"
                ))),
            },
        }
    }

    /// Works the front job; a finished one is answered and leaves.
    fn work(&mut self, state: &mut State) {
        let Some(job) = self.jobs.pop_front() else {
            return;
        };
        if let Some(unfinished) = self.step(state, job) {
            self.jobs.push_front(unfinished);
        }
    }

    /// One step of `job`; the job back if it is not done.
    fn step(&mut self, state: &mut State, job: Job) -> Option<Job> {
        match job {
            Job::Advance { until, answer } => {
                run_to(state, until);
                if state.clock.now() >= until {
                    answer(DebugReply::Done);
                    None
                } else {
                    Some(Job::Advance { until, answer })
                }
            }
            Job::Frames { goal, answer } => {
                if dump::frames_presented(state, None) >= goal || !walk_to_next_stop(state) {
                    answer(DebugReply::Done);
                    None
                } else {
                    Some(Job::Frames { goal, answer })
                }
            }
            Job::Wait {
                predicate,
                frames_goal,
                deadline,
                answer,
            } => self.wait(state, predicate, frames_goal, deadline, answer),
            Job::Script {
                lines,
                answers,
                resume_at,
                answer,
            } => self.script(state, lines, answers, resume_at, answer),
            Job::PanicAt { goal } => {
                assert!(
                    dump::frames_presented(state, None) < goal,
                    "casement: the injected fault panic-after-frames fired"
                );
                Some(Job::PanicAt { goal })
            }
        }
    }

    fn wait(
        &mut self,
        state: &mut State,
        predicate: WaitPredicate,
        frames_goal: u64,
        deadline: Instant,
        answer: Answer,
    ) -> Option<Job> {
        let met = dump::met(state, &predicate, frames_goal);
        let expired = state.clock.wall_instant() >= deadline;
        if met || expired {
            let outcome = if met { "met" } else { "timeout" };
            answer(DebugReply::Waited(outcome.to_owned(), dump::scene(state)));
            return None;
        }
        // Virtual time only moves when asked: a wait on frames or on motion settling walks it.
        let walks = matches!(
            predicate,
            WaitPredicate::Frames { .. } | WaitPredicate::Settled
        );
        if walks && virtual_clock(state).is_ok() {
            walk_to_next_stop(state);
        }
        Some(Job::Wait {
            predicate,
            frames_goal,
            deadline,
            answer,
        })
    }

    /// Injects lines until a sleep, then waits for time to pass.
    fn script(
        &mut self,
        state: &mut State,
        mut lines: VecDeque<InjectLine>,
        mut answers: Vec<String>,
        resume_at: Option<Mono>,
        answer: Answer,
    ) -> Option<Job> {
        if let Some(until) = resume_at {
            run_to(state, until);
            if state.clock.now() < until {
                return Some(Job::Script {
                    lines,
                    answers,
                    resume_at,
                    answer,
                });
            }
        }
        while let Some(line) = lines.pop_front() {
            answers.push(format!("ok {line:?}"));
            if let InjectLine::Sleep(by) = line {
                let until = state.clock.now().after(by);
                run_to(state, until);
                return Some(Job::Script {
                    lines,
                    answers,
                    resume_at: Some(until),
                    answer,
                });
            }
            self.injector.line(state, &line);
        }
        answer(DebugReply::Injected(answers));
        None
    }
}

/// `Ok` when the clock can be stepped (it is virtual).
fn virtual_clock(state: &State) -> Result<(), String> {
    match &state.clock {
        ClockSource::Monotonic(_) => Err("the wall clock cannot be stepped".to_owned()),
        ClockSource::Virtual(_) | ClockSource::Stepped(_) => Ok(()),
    }
}

/// Asks the virtual clock to walk to `until` (the wall clock moves by itself).
fn run_to(state: &State, until: Mono) {
    if let Some(clock) = state.clock.shared() {
        clock.run_to(until);
    }
}

/// Walks the virtual clock to its next stop; false when nothing is due for an hour (no
/// outputs, no timers) and the walk would never reach a frame.
fn walk_to_next_stop(state: &State) -> bool {
    let Some(clock) = state.clock.shared() else {
        return true;
    };
    let (at, stop) = clock.next_stop(clock.now().after(Duration::from_secs(3600)));
    clock.run_to(at);
    !matches!(stop, casement_core::time::Stop::End)
}

/// The frames a wait predicate counts.
fn frames_of(predicate: &WaitPredicate) -> u64 {
    match predicate {
        WaitPredicate::Frames { n, .. } => u64::from(*n),
        _ => 0,
    }
}

/// The output a frames predicate names.
fn frames_output(predicate: &WaitPredicate) -> Option<&casement_core::OutputName> {
    match predicate {
        WaitPredicate::Frames { output, .. } => output.as_ref(),
        _ => None,
    }
}
