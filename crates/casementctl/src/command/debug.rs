//! Running a `debug` call: read stdin for `inject`, make the call, print the answer.

use super::bus::{self, Answer, BusError};
use super::plan::Call;
use std::io::BufRead;

/// The exit code of success.
const SUCCESS: u8 = 0;
/// The exit code of a wait that timed out (its scene dump goes to stderr).
const TIMED_OUT: u8 = 1;
/// The exit code of a call the bus or casement refused.
const REFUSED: u8 = 1;

/// Makes `call` and prints what it answered: exit 0 on success, 1 on a timed-out wait or a
/// refused call. An `inject` that names no lines streams stdin: each line is sent and answered
/// before the next is read, so a script can feed it through a fifo that never closes, as
/// shell-host's fake-input.sh is fed.
pub fn run_call(call: Call) -> u8 {
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => return fail(&format!("tokio: {error}")),
    };
    runtime.block_on(async {
        let connection = match bus::connect().await {
            Ok(connection) => connection,
            Err(error) => return fail_with(&error),
        };
        match call {
            Call::Inject(lines) if lines.is_empty() => stream_stdin(&connection).await,
            call => answer_one(&connection, call).await,
        }
    })
}

async fn answer_one(connection: &zbus::Connection, call: Call) -> u8 {
    match bus::make(connection, call).await {
        Ok(answer) => print_answer(answer),
        Err(error) => fail_with(&error),
    }
}

/// Each command line on stdin (blank lines and `#` comments skipped) as its own `Inject`. A line
/// casement refuses is answered `err <line>: <why>` and the stream goes on, as fake-input.sh does;
/// the exit status is 1 if any was refused. A lost bus ends the stream at once.
async fn stream_stdin(connection: &zbus::Connection) -> u8 {
    let mut refused = 0_u32;
    for line in std::io::stdin().lock().lines() {
        let line = match line {
            Ok(line) => line,
            Err(error) => return fail(&format!("reading stdin: {error}")),
        };
        if !is_command(&line) {
            continue;
        }
        let command = line.trim().to_owned();
        match bus::make(connection, Call::Inject(vec![command.clone()])).await {
            Ok(answer) => {
                print_answer(answer);
            }
            Err(error) => match refusal(&error) {
                Some(why) => {
                    println!("err {command}: {why}");
                    refused += 1;
                }
                None => return fail_with(&error),
            },
        }
    }
    if refused == 0 { SUCCESS } else { REFUSED }
}

/// Why casement refused a call, when the bus carried the refusal back (a lost bus is not one).
fn refusal(error: &BusError) -> Option<String> {
    match error {
        BusError::Zbus(zbus::Error::MethodError(_, why, _)) => {
            Some(why.clone().unwrap_or_else(|| "refused".to_owned()))
        }
        BusError::Zbus(zbus::Error::FDO(fault)) => Some(fault.to_string()),
        _ => None,
    }
}

/// Whether a stdin line is a command: blank lines and `#` comments are not.
fn is_command(line: &str) -> bool {
    let line = line.trim();
    !line.is_empty() && !line.starts_with('#')
}

fn print_answer(answer: Answer) -> u8 {
    match answer {
        Answer::Lines(lines) => lines.iter().for_each(|line| println!("{line}")),
        Answer::Text(text) if text.is_empty() => {}
        Answer::Text(text) => println!("{text}"),
        Answer::Waited { outcome, scene } => {
            println!("{outcome}");
            if outcome != "met" {
                eprintln!("{scene}");
                return TIMED_OUT;
            }
        }
    }
    SUCCESS
}

fn fail(message: &str) -> u8 {
    eprintln!("casementctl: {message}");
    REFUSED
}

fn fail_with(error: &BusError) -> u8 {
    fail(&error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blank_lines_and_comments_are_not_commands() {
        let input = "move 1 2\n\n  # a note\n  press left  \nsleep 5";
        let commands: Vec<&str> = input.lines().filter(|line| is_command(line)).collect();
        assert_eq!(commands, ["move 1 2", "  press left  ", "sleep 5"]);
    }
}
