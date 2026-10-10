//! The session's command language: one line, a verb and `key=value` options, parsed into typed
//! commands before anything is sent.
//!
//! ```text
//! map NAME class=C [instance=I] [size=WxH] [at=X,Y] [kind=normal|override-redirect]
//! unmap NAME | close NAME | focus NAME | activate NAME | info NAME
//! own-clipboard TEXT | paste | sync | quit
//! ```
//!
//! `focus` is `SetInputFocus`; `activate` is the `_NET_ACTIVE_WINDOW` client message to the root
//! window a toolkit sends to raise a window.

/// Whether the window is managed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    /// An ordinary window.
    Normal,
    /// A window the window manager must not manage (menus, tooltips).
    OverrideRedirect,
}

/// What a window is made with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MapAsk {
    pub(crate) name: String,
    pub(crate) instance: String,
    pub(crate) class: String,
    pub(crate) size: (u16, u16),
    pub(crate) at: (i16, i16),
    pub(crate) kind: Kind,
}

/// One command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Command {
    Map(MapAsk),
    Unmap(String),
    Close(String),
    Focus(String),
    Activate(String),
    Info(String),
    OwnClipboard(String),
    Paste,
    Sync,
    Quit,
}

/// Parses one line; the error is what the session prints.
pub(crate) fn parse(line: &str) -> Result<Command, String> {
    let line = line.trim();
    let (verb, rest) = line.split_once(' ').unwrap_or((line, ""));
    let mut words = rest.split_whitespace();
    match verb {
        "map" => {
            let name = next_name(&mut words, verb)?;
            map(name, words.collect())
        }
        "unmap" => Ok(Command::Unmap(next_name(&mut words, verb)?)),
        "close" => Ok(Command::Close(next_name(&mut words, verb)?)),
        "focus" => Ok(Command::Focus(next_name(&mut words, verb)?)),
        "activate" => Ok(Command::Activate(next_name(&mut words, verb)?)),
        "info" => Ok(Command::Info(next_name(&mut words, verb)?)),
        "own-clipboard" if !rest.is_empty() => Ok(Command::OwnClipboard(rest.to_owned())),
        "own-clipboard" => Err("own-clipboard needs the text".to_owned()),
        "paste" => Ok(Command::Paste),
        "sync" => Ok(Command::Sync),
        "quit" => Ok(Command::Quit),
        other => Err(format!("unknown command '{other}'")),
    }
}

/// The next word, as a window name.
fn next_name(words: &mut std::str::SplitWhitespace<'_>, verb: &str) -> Result<String, String> {
    words
        .next()
        .map(str::to_owned)
        .ok_or_else(|| format!("{verb} needs a window name"))
}

fn map(name: String, options: Vec<&str>) -> Result<Command, String> {
    let mut ask = MapAsk {
        name,
        instance: String::new(),
        class: String::new(),
        size: (200, 150),
        at: (0, 0),
        kind: Kind::Normal,
    };
    for option in options {
        let (key, value) = option
            .split_once('=')
            .ok_or_else(|| format!("'{option}' is not key=value"))?;
        match key {
            "class" => ask.class = value.to_owned(),
            "instance" => ask.instance = value.to_owned(),
            "size" => ask.size = pair(value, 'x').ok_or_else(|| format!("bad size '{value}'"))?,
            "at" => ask.at = pair(value, ',').ok_or_else(|| format!("bad position '{value}'"))?,
            "kind" => {
                ask.kind = match value {
                    "normal" => Kind::Normal,
                    "override-redirect" => Kind::OverrideRedirect,
                    other => return Err(format!("unknown kind '{other}'")),
                }
            }
            other => return Err(format!("unknown option '{other}'")),
        }
    }
    if ask.class.is_empty() {
        return Err("map needs class=C".to_owned());
    }
    if ask.instance.is_empty() {
        ask.instance = ask.class.to_lowercase();
    }
    Ok(Command::Map(ask))
}

/// `AxB` as two numbers.
fn pair<T: std::str::FromStr>(text: &str, separator: char) -> Option<(T, T)> {
    let (a, b) = text.split_once(separator)?;
    Some((a.parse().ok()?, b.parse().ok()?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_map_line_is_read_with_its_defaults() {
        let Ok(Command::Map(ask)) = parse("map w class=Xprobe size=300x200 kind=override-redirect")
        else {
            panic!("not a map");
        };
        assert_eq!(ask.name, "w");
        assert_eq!(
            (ask.class.as_str(), ask.instance.as_str()),
            ("Xprobe", "xprobe")
        );
        assert_eq!(ask.size, (300, 200));
        assert_eq!(ask.kind, Kind::OverrideRedirect);
    }

    #[test]
    fn clipboard_text_keeps_its_spaces_and_bad_lines_are_refused() {
        assert_eq!(
            parse("own-clipboard hello  there"),
            Ok(Command::OwnClipboard("hello  there".to_owned()))
        );
        assert!(parse("map w").is_err(), "no class");
        assert!(parse("map w class=A size=3").is_err());
        assert!(parse("dance").is_err());
        assert!(parse("focus").is_err());
    }
}
