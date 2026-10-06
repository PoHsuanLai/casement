//! The session's command language: one line, a verb and `key=value` options, parsed into typed
//! commands before anything is sent. A script writes these lines to the probe's stdin.
//!
//! ```text
//! toplevel NAME [app=ID] [size=WxH] [pattern=checker:N|gradient|solid:HEX] [scale=1|1.5|2] [maximised]
//! redraw NAME [size=WxH] | maximise NAME | unmaximise NAME | move-request NAME | resize-request NAME EDGE
//! popup NAME PARENT [size=WxH] [at=X,Y] [grab] | popup-close NAME
//! layer NAME [level=background|bottom|top|overlay] [kbd=none|exclusive|ondemand] [size=WxH] [anchor=top,left,...]
//! layer-close NAME | lock NAME | unlock NAME | close NAME
//! copy TEXT | paste | drag NAME TEXT
//! ime-start | ime-enable | ime-grab | ime-preedit TEXT | ime-commit TEXT
//! sync | quit
//! ```

use casement_testclient::Pattern;
use casement_testclient::surface::layer::{Anchor, KeyboardInteractivity, Layer};
use casement_testclient::surface::toplevel::ResizeEdge;

/// A size in logical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Size {
    pub(crate) w: u32,
    pub(crate) h: u32,
}

/// How a window draws for its output: the buffer's scale.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Dpi {
    /// 1x.
    One,
    /// 1.5x, a buffer of 1.5 times the size under a viewport destination.
    OneAndAHalf,
    /// 2x, a buffer of twice the size under `set_buffer_scale(2)`.
    Two,
}

impl Dpi {
    /// The buffer's size in pixels for a surface of `logical` size (rounded up).
    pub(crate) fn buffer(self, logical: Size) -> Size {
        let times = |n: u32, num: u32| (n * num).div_ceil(2);
        let num = match self {
            Dpi::One => 2,
            Dpi::OneAndAHalf => 3,
            Dpi::Two => 4,
        };
        Size {
            w: times(logical.w, num),
            h: times(logical.h, num),
        }
    }
}

/// What a toplevel is made with.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ToplevelAsk {
    pub(crate) name: String,
    pub(crate) app_id: String,
    pub(crate) size: Size,
    pub(crate) pattern: Pattern,
    pub(crate) dpi: Dpi,
    pub(crate) maximised: bool,
}

/// What a popup is made with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PopupAsk {
    pub(crate) name: String,
    pub(crate) parent: String,
    pub(crate) size: Size,
    pub(crate) at: (i32, i32),
    pub(crate) grab: bool,
}

/// What a layer surface is made with.
#[derive(Debug, Clone)]
pub(crate) struct LayerCmd {
    pub(crate) name: String,
    pub(crate) level: Layer,
    pub(crate) keyboard: KeyboardInteractivity,
    pub(crate) size: Size,
    pub(crate) anchor: Anchor,
}

/// One command.
#[derive(Debug, Clone)]
pub(crate) enum Command {
    Toplevel(ToplevelAsk),
    Redraw(String, Option<Size>),
    Maximise(String),
    Unmaximise(String),
    MoveRequest(String),
    ResizeRequest(String, ResizeEdge),
    Popup(PopupAsk),
    PopupClose(String),
    Layer(LayerCmd),
    LayerClose(String),
    Lock(String),
    Unlock(String),
    Close(String),
    Copy(String),
    Paste,
    Drag(String, String),
    ImeStart,
    ImeEnable,
    ImeGrab,
    ImePreedit(String),
    ImeCommit(String),
    Sync,
    Quit,
}

/// The words of a line after its verb: bare words and `key=value` options.
struct Words<'a> {
    bare: Vec<&'a str>,
    options: Vec<(&'a str, &'a str)>,
}

impl<'a> Words<'a> {
    fn of(rest: &'a str) -> Words<'a> {
        let mut words = Words {
            bare: Vec::new(),
            options: Vec::new(),
        };
        for word in rest.split_whitespace() {
            match word.split_once('=') {
                Some(pair) => words.options.push(pair),
                None => words.bare.push(word),
            }
        }
        words
    }

    fn option(&self, key: &str) -> Option<&'a str> {
        self.options
            .iter()
            .find_map(|(k, v)| (*k == key).then_some(*v))
    }

    fn has(&self, flag: &str) -> bool {
        self.bare.contains(&flag)
    }

    fn name(&self, verb: &str) -> Result<String, String> {
        self.bare
            .first()
            .map(|name| (*name).to_owned())
            .ok_or_else(|| format!("{verb} needs a NAME"))
    }
}

fn size(text: &str) -> Result<Size, String> {
    let bad = || format!("not a size (WxH): {text}");
    let (w, h) = text.split_once('x').ok_or_else(bad)?;
    match (w.parse(), h.parse()) {
        (Ok(w @ 1..), Ok(h @ 1..)) => Ok(Size { w, h }),
        _ => Err(bad()),
    }
}

fn pattern(text: &str) -> Result<Pattern, String> {
    let bad = || format!("not a pattern: {text}");
    match text.split_once(':') {
        None if text == "gradient" => Ok(Pattern::Gradient),
        None if text == "coordinates" => Ok(Pattern::CoordinateEncoding),
        Some(("checker", n)) => n
            .parse()
            .map(|cell| Pattern::Checker { cell })
            .map_err(|_| bad()),
        Some(("solid", hex)) => u32::from_str_radix(hex, 16)
            .map(|rgb| Pattern::Solid(0xff00_0000 | rgb))
            .map_err(|_| bad()),
        _ => Err(bad()),
    }
}

fn dpi(text: &str) -> Result<Dpi, String> {
    match text {
        "1" => Ok(Dpi::One),
        "1.5" => Ok(Dpi::OneAndAHalf),
        "2" => Ok(Dpi::Two),
        _ => Err(format!("scale is 1, 1.5 or 2: {text}")),
    }
}

fn edge(text: &str) -> Result<ResizeEdge, String> {
    [
        ("top", ResizeEdge::Top),
        ("bottom", ResizeEdge::Bottom),
        ("left", ResizeEdge::Left),
        ("right", ResizeEdge::Right),
        ("top-left", ResizeEdge::TopLeft),
        ("top-right", ResizeEdge::TopRight),
        ("bottom-left", ResizeEdge::BottomLeft),
        ("bottom-right", ResizeEdge::BottomRight),
    ]
    .into_iter()
    .find_map(|(word, edge)| (word == text).then_some(edge))
    .ok_or_else(|| format!("not an edge: {text}"))
}

fn level(text: &str) -> Result<Layer, String> {
    match text {
        "background" => Ok(Layer::Background),
        "bottom" => Ok(Layer::Bottom),
        "top" => Ok(Layer::Top),
        "overlay" => Ok(Layer::Overlay),
        _ => Err(format!("not a layer: {text}")),
    }
}

fn keyboard(text: &str) -> Result<KeyboardInteractivity, String> {
    match text {
        "none" => Ok(KeyboardInteractivity::None),
        "exclusive" => Ok(KeyboardInteractivity::Exclusive),
        "ondemand" => Ok(KeyboardInteractivity::OnDemand),
        _ => Err(format!("not a keyboard mode: {text}")),
    }
}

fn anchor(text: &str) -> Result<Anchor, String> {
    text.split(',')
        .filter(|edge| !edge.is_empty() && *edge != "none")
        .try_fold(Anchor::empty(), |all, edge| {
            let one = match edge {
                "top" => Anchor::Top,
                "bottom" => Anchor::Bottom,
                "left" => Anchor::Left,
                "right" => Anchor::Right,
                _ => return Err(format!("not an anchor edge: {edge}")),
            };
            Ok(all | one)
        })
}

fn point(text: &str) -> Result<(i32, i32), String> {
    let bad = || format!("not a point (X,Y): {text}");
    let (x, y) = text.split_once(',').ok_or_else(bad)?;
    Ok((x.parse().map_err(|_| bad())?, y.parse().map_err(|_| bad())?))
}

/// The command `line` says.
pub(crate) fn parse(line: &str) -> Result<Command, String> {
    let line = line.trim();
    let (verb, rest) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
    let rest = rest.trim();
    let words = Words::of(rest);
    let named = || words.name(verb);
    let sized = |key: &str, default: Size| words.option(key).map_or(Ok(default), size);
    match verb {
        "toplevel" => Ok(Command::Toplevel(ToplevelAsk {
            name: named()?,
            app_id: words
                .option("app")
                .unwrap_or("org.example.probe")
                .to_owned(),
            size: sized("size", Size { w: 400, h: 300 })?,
            pattern: words
                .option("pattern")
                .map_or(Ok(Pattern::Gradient), pattern)?,
            dpi: words.option("scale").map_or(Ok(Dpi::One), dpi)?,
            maximised: words.has("maximised"),
        })),
        "redraw" => Ok(Command::Redraw(
            named()?,
            words.option("size").map(size).transpose()?,
        )),
        "maximise" => Ok(Command::Maximise(named()?)),
        "unmaximise" => Ok(Command::Unmaximise(named()?)),
        "move-request" => Ok(Command::MoveRequest(named()?)),
        "resize-request" => match words.bare.as_slice() {
            [name, edge_word] => Ok(Command::ResizeRequest((*name).to_owned(), edge(edge_word)?)),
            _ => Err("resize-request needs NAME and EDGE".to_owned()),
        },
        "popup" => match words.bare.as_slice() {
            [name, parent, ..] => Ok(Command::Popup(PopupAsk {
                name: (*name).to_owned(),
                parent: (*parent).to_owned(),
                size: sized("size", Size { w: 100, h: 50 })?,
                at: words.option("at").map_or(Ok((20, 20)), point)?,
                grab: words.has("grab"),
            })),
            _ => Err("popup needs NAME and PARENT".to_owned()),
        },
        "popup-close" => Ok(Command::PopupClose(named()?)),
        "layer" => Ok(Command::Layer(LayerCmd {
            name: named()?,
            level: words.option("level").map_or(Ok(Layer::Top), level)?,
            keyboard: words
                .option("kbd")
                .map_or(Ok(KeyboardInteractivity::None), keyboard)?,
            size: sized("size", Size { w: 200, h: 100 })?,
            anchor: words.option("anchor").map_or(Ok(Anchor::empty()), anchor)?,
        })),
        "layer-close" => Ok(Command::LayerClose(named()?)),
        "lock" => Ok(Command::Lock(named()?)),
        "unlock" => Ok(Command::Unlock(named()?)),
        "close" => Ok(Command::Close(named()?)),
        "copy" if !rest.is_empty() => Ok(Command::Copy(rest.to_owned())),
        "paste" => Ok(Command::Paste),
        "drag" => match rest.split_once(char::is_whitespace) {
            Some((name, text)) => Ok(Command::Drag(name.to_owned(), text.trim().to_owned())),
            None => Err("drag needs NAME and TEXT".to_owned()),
        },
        "ime-start" => Ok(Command::ImeStart),
        "ime-enable" => Ok(Command::ImeEnable),
        "ime-grab" => Ok(Command::ImeGrab),
        "ime-preedit" if !rest.is_empty() => Ok(Command::ImePreedit(rest.to_owned())),
        "ime-commit" if !rest.is_empty() => Ok(Command::ImeCommit(rest.to_owned())),
        "sync" => Ok(Command::Sync),
        "quit" => Ok(Command::Quit),
        "" => Err("empty command".to_owned()),
        other => Err(format!("unknown command {other}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_toplevel_takes_options_in_any_order_with_defaults() {
        let Ok(Command::Toplevel(ask)) =
            parse("toplevel w1 scale=1.5 maximised size=640x480 pattern=checker:3 app=a.b")
        else {
            panic!("a toplevel");
        };
        assert_eq!(ask.name, "w1");
        assert_eq!(ask.app_id, "a.b");
        assert_eq!(ask.size, Size { w: 640, h: 480 });
        assert_eq!(ask.pattern, Pattern::Checker { cell: 3 });
        assert_eq!(ask.dpi, Dpi::OneAndAHalf);
        assert!(ask.maximised);
        let Ok(Command::Toplevel(plain)) = parse("toplevel w2") else {
            panic!("a toplevel");
        };
        assert_eq!(plain.size, Size { w: 400, h: 300 });
        assert_eq!(plain.dpi, Dpi::One);
        assert!(!plain.maximised);
    }

    #[test]
    fn text_commands_keep_the_rest_of_the_line() {
        assert!(
            matches!(parse("copy hello  big world"), Ok(Command::Copy(t)) if t == "hello  big world")
        );
        assert!(
            matches!(parse("drag w1 some text"), Ok(Command::Drag(n, t)) if n == "w1" && t == "some text")
        );
        assert!(matches!(parse("ime-commit あ"), Ok(Command::ImeCommit(t)) if t == "あ"));
    }

    #[test]
    fn bad_lines_say_why_and_never_panic() {
        for line in [
            "",
            "frobnicate",
            "toplevel",
            "toplevel w size=0x5",
            "toplevel w size=big",
            "toplevel w scale=3",
            "toplevel w pattern=stripes",
            "resize-request w sideways",
            "resize-request w",
            "popup p",
            "popup p w at=3",
            "layer l level=floor",
            "layer l anchor=middle",
            "copy",
            "drag w",
        ] {
            assert!(parse(line).is_err(), "{line:?}");
        }
    }

    #[test]
    fn buffers_are_the_scale_times_the_size() {
        let logical = Size { w: 400, h: 300 };
        assert_eq!(Dpi::One.buffer(logical), logical);
        assert_eq!(Dpi::OneAndAHalf.buffer(logical), Size { w: 600, h: 450 });
        assert_eq!(Dpi::Two.buffer(logical), Size { w: 800, h: 600 });
        assert_eq!(
            Dpi::OneAndAHalf.buffer(Size { w: 3, h: 3 }),
            Size { w: 5, h: 5 }
        );
    }

    #[test]
    fn layers_and_popups_parse_their_options() {
        let Ok(Command::Layer(layer)) =
            parse("layer l level=overlay kbd=exclusive size=10x20 anchor=top,left")
        else {
            panic!("a layer");
        };
        assert_eq!(layer.level, Layer::Overlay);
        assert_eq!(layer.keyboard, KeyboardInteractivity::Exclusive);
        assert_eq!(layer.size, Size { w: 10, h: 20 });
        assert_eq!(layer.anchor, Anchor::Top | Anchor::Left);
        let Ok(Command::Popup(popup)) = parse("popup p w grab at=5,-6 size=30x40") else {
            panic!("a popup");
        };
        assert_eq!((popup.name.as_str(), popup.parent.as_str()), ("p", "w"));
        assert!(popup.grab);
        assert_eq!(popup.at, (5, -6));
        assert_eq!(popup.size, Size { w: 30, h: 40 });
    }
}
