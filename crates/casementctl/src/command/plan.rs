//! What a `debug` command line asks Debug1 for: the verb's arguments checked and shaped into
//! one [`Call`], before any bus is touched.

use super::CommandError;
use std::time::Duration;

/// How long `debug wait` waits (session time) when `--timeout` is not given.
pub const DEFAULT_WAIT: Duration = Duration::from_secs(10);

/// One call on `org.quire.Casement.Debug1`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Call {
    /// `Inject(lines)`; no lines named means they come on stdin.
    Inject(Vec<String>),
    /// `Wait(predicate, timeout_ms)`.
    Wait {
        /// The predicate text (casement-ipc's wait grammar).
        predicate: String,
        /// The bound, in session time.
        timeout: Duration,
    },
    /// `Clock(verb, amount)`.
    Clock(String, u32),
    /// `Output(verb, args)`.
    Output(String, Vec<String>),
    /// `Windows()`.
    Windows,
    /// `Scene()`.
    Scene,
    /// `Fault(verb, amount)`.
    Fault(String, u32),
    /// `Shot(area, path)`: casement's PNG of its last frame.
    Shot {
        /// Debug1's area text (`output`, `output:NAME`, `region:X,Y,W,H`).
        area: String,
        /// The absolute path casement writes to.
        path: String,
    },
}

fn bad(message: impl Into<String>) -> CommandError {
    CommandError::BadArgument(message.into())
}

fn number(what: &str, text: &str) -> Result<u32, CommandError> {
    text.parse()
        .map_err(|_| bad(format!("{what}: not a number: {text}")))
}

impl Call {
    /// The call the debug verb `verb` (`inject`, `wait`, `clock`, `output`, `windows`, `scene`,
    /// `fault`) with `args` asks for.
    pub fn parse(verb: &str, args: &[String]) -> Result<Call, CommandError> {
        match verb {
            "inject" => Ok(Call::Inject(args.to_vec())),
            "wait" => wait(args),
            "clock" => match args {
                [word, n] if word == "step" => {
                    Ok(Call::Clock(word.clone(), number("clock step", n)?))
                }
                [word, ms] if word == "advance" => {
                    Ok(Call::Clock(word.clone(), number("clock advance", ms)?))
                }
                _ => Err(bad("clock wants `step N` or `advance MS`")),
            },
            "output" => output(args),
            "windows" => no_arguments(args, Call::Windows),
            "scene" => no_arguments(args, Call::Scene),
            "fault" => fault(args),
            other => Err(CommandError::Unknown(format!("debug {other}"))),
        }
    }
}

/// `windows` and `scene` always answer JSON; `--json` is accepted for the plan's spelling.
fn no_arguments(args: &[String], call: Call) -> Result<Call, CommandError> {
    match args {
        [] => Ok(call),
        [flag] if flag == "--json" => Ok(call),
        _ => Err(bad("this verb takes no arguments")),
    }
}

/// `PREDICATE... [--timeout MS]`: the words that are not the option are the predicate.
fn wait(args: &[String]) -> Result<Call, CommandError> {
    let mut words = Vec::new();
    let mut timeout = DEFAULT_WAIT;
    let mut rest = args.iter();
    while let Some(word) = rest.next() {
        if word == "--timeout" {
            let ms = rest.next().ok_or_else(|| bad("--timeout needs a value"))?;
            timeout = Duration::from_millis(u64::from(number("--timeout", ms)?));
        } else {
            words.push(word.as_str());
        }
    }
    if words.is_empty() {
        return Err(bad(
            "wait needs a predicate (mapped|focused|gone APP, frames N, settled)",
        ));
    }
    Ok(Call::Wait {
        predicate: words.join(" "),
        timeout,
    })
}

fn fault(args: &[String]) -> Result<Call, CommandError> {
    let (verb, what) = match args {
        [word, n] => (word.as_str(), n),
        _ => return Err(bad("fault wants a verb and one number")),
    };
    if !matches!(
        verb,
        "panic-after-frames" | "stall-client-ms" | "drop-output"
    ) {
        return Err(bad(format!("fault: unknown verb {verb}")));
    }
    Ok(Call::Fault(verb.to_owned(), number(verb, what)?))
}

/// `list [--json] | add SPEC | remove NAME | set NAME (KEY=VALUE | --scale S | --mode M |
/// --position X,Y | --power on|off)...`.
fn output(args: &[String]) -> Result<Call, CommandError> {
    let (verb, rest) = args
        .split_first()
        .ok_or_else(|| bad("output wants a verb"))?;
    match (verb.as_str(), rest) {
        ("list", []) => Ok(Call::Output("list".to_owned(), Vec::new())),
        ("list", [flag]) if flag == "--json" => Ok(Call::Output("list".to_owned(), Vec::new())),
        ("add" | "remove", [one]) => Ok(Call::Output(verb.clone(), vec![one.clone()])),
        ("set", [name, settings @ ..]) if !settings.is_empty() => {
            let mut sent = vec![name.clone()];
            sent.extend(settings_of(settings)?);
            Ok(Call::Output("set".to_owned(), sent))
        }
        _ => Err(bad(
            "output wants list | add SPEC | remove NAME | set NAME KEY=VALUE...",
        )),
    }
}

/// The `KEY=VALUE` words for `set`: given as such, or as the plan's flags.
fn settings_of(words: &[String]) -> Result<Vec<String>, CommandError> {
    let mut sent = Vec::new();
    let mut rest = words.iter();
    while let Some(word) = rest.next() {
        let flag = word.strip_prefix("--").filter(|f| !f.contains('='));
        match flag {
            None if word.contains('=') => sent.push(word.clone()),
            Some(key @ ("scale" | "mode" | "position" | "power" | "transform")) => {
                let value = rest
                    .next()
                    .ok_or_else(|| bad(format!("--{key} needs a value")))?;
                let value = if key == "scale" {
                    scale120(value)?
                } else {
                    value.clone()
                };
                sent.push(format!("{key}={value}"));
            }
            _ => {
                return Err(bad(format!(
                    "output set: not KEY=VALUE or a known flag: {word}"
                )));
            }
        }
    }
    Ok(sent)
}

/// A scale written as a decimal (`1.5`) in 120ths, the unit casement's outputs use
/// (`1.5` is `180`); a scale that is not a whole number of 120ths is refused.
fn scale120(text: &str) -> Result<String, CommandError> {
    let (whole, fraction) = text.split_once('.').unwrap_or((text, ""));
    let whole: u32 = whole
        .parse()
        .map_err(|_| bad(format!("--scale: not a number: {text}")))?;
    let dangling_dot = text.contains('.') && fraction.is_empty();
    if dangling_dot || fraction.len() > 6 || !fraction.bytes().all(|b| b.is_ascii_digit()) {
        return Err(bad(format!("--scale: not a number: {text}")));
    }
    let millionths: u64 = format!("{fraction:0<6}").parse().unwrap_or(0);
    let total = u64::from(whole) * 120 + millionths * 120 / 1_000_000;
    let exact = (millionths * 120).is_multiple_of(1_000_000);
    match (exact, total) {
        (true, 1..) => Ok(total.to_string()),
        _ => Err(bad(format!(
            "--scale {text} is not a whole number of 120ths (1, 1.25, 1.5, 2)"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(text: &str) -> Vec<String> {
        text.split_whitespace().map(str::to_owned).collect()
    }

    fn planned(verb: &str, text: &str) -> Result<Call, CommandError> {
        Call::parse(verb, &words(text))
    }

    #[test]
    fn wait_joins_the_predicate_and_reads_its_bound() {
        assert_eq!(
            planned("wait", "mapped foot --timeout 2500").expect("wait"),
            Call::Wait {
                predicate: "mapped foot".to_owned(),
                timeout: Duration::from_millis(2500)
            }
        );
        assert_eq!(
            planned("wait", "--timeout 5 frames 3 HEADLESS-1").expect("wait"),
            Call::Wait {
                predicate: "frames 3 HEADLESS-1".to_owned(),
                timeout: Duration::from_millis(5)
            }
        );
        assert_eq!(
            planned("wait", "settled").expect("wait"),
            Call::Wait {
                predicate: "settled".to_owned(),
                timeout: DEFAULT_WAIT
            }
        );
        assert!(planned("wait", "").is_err());
        assert!(planned("wait", "settled --timeout").is_err());
        assert!(planned("wait", "settled --timeout soon").is_err());
    }

    #[test]
    fn clock_and_fault_take_a_verb_and_a_number() {
        assert_eq!(
            planned("clock", "step 3").expect("step"),
            Call::Clock("step".to_owned(), 3)
        );
        assert_eq!(
            planned("clock", "advance 250").expect("advance"),
            Call::Clock("advance".to_owned(), 250)
        );
        assert_eq!(
            planned("fault", "drop-output 1").expect("fault"),
            Call::Fault("drop-output".to_owned(), 1)
        );
        for (verb, text) in [
            ("clock", "step"),
            ("clock", "step -1"),
            ("clock", "rewind 2"),
            ("fault", "explode 1"),
            ("fault", "drop-output"),
        ] {
            assert!(planned(verb, text).is_err(), "{text}");
        }
    }

    #[test]
    fn output_set_takes_pairs_or_the_plans_flags() {
        assert_eq!(
            planned(
                "output",
                "set DP-1 --scale 1.5 --mode 1920x1080@60000 --power off position=0,0"
            )
            .expect("set"),
            Call::Output(
                "set".to_owned(),
                words("DP-1 scale=180 mode=1920x1080@60000 power=off position=0,0")
            )
        );
        assert_eq!(
            planned("output", "list --json").expect("list"),
            Call::Output("list".to_owned(), Vec::new())
        );
        assert_eq!(
            planned("output", "add DP-2:1280x720@60000:120:0,0").expect("add"),
            Call::Output("add".to_owned(), words("DP-2:1280x720@60000:120:0,0"))
        );
        for text in [
            "",
            "set DP-1",
            "set DP-1 loud",
            "set DP-1 --scale",
            "add",
            "remove a b",
            "rotate",
        ] {
            assert!(planned("output", text).is_err(), "{text}");
        }
    }

    #[test]
    fn scales_are_whole_120ths() {
        for (text, expected) in [
            ("1", "120"),
            ("1.25", "150"),
            ("1.5", "180"),
            ("2", "240"),
            ("0.5", "60"),
            ("1.0", "120"),
        ] {
            assert_eq!(scale120(text).expect(text), expected, "{text}");
        }
        for text in ["0", "1.001", "x", "1.", "-1", "1.5x"] {
            assert!(scale120(text).is_err(), "{text}");
        }
    }

    #[test]
    fn inject_keeps_its_lines_and_the_json_verbs_take_nothing() {
        assert_eq!(
            Call::parse("inject", &["move 1 2".to_owned()]).expect("inject"),
            Call::Inject(vec!["move 1 2".to_owned()])
        );
        assert_eq!(planned("windows", "").expect("windows"), Call::Windows);
        assert_eq!(planned("scene", "--json").expect("scene"), Call::Scene);
        assert!(planned("scene", "extra").is_err());
    }
}
