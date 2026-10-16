//! Consent decisions: what a program gets for a power, from the administrator's floor and the
//! person's book (precedence), the book's and the floor's text, and the caps.

use casement_core::privilege::{
    Book, BookError, Decision, Engine, Fallback, Floor, FloorError, FloorRule, Grant, Identity,
    MAX_FILE_BYTES, MAX_GRANTS, ProgramPath, SandboxAppId, Subject, UnixSeconds, Verdict, decide,
    decide_trusted,
};
use casement_protocols::vocab::{Memory, Power, WireCode};
use proptest::prelude::*;

fn program(path: &str) -> Subject {
    Subject::Program(ProgramPath::parse(path).expect("a path"))
}

fn flatpak(id: &str) -> Subject {
    Subject::Sandboxed {
        app: SandboxAppId::parse(id).expect("an app id"),
        engine: Engine::Flatpak,
    }
}

fn grant(power: Power, subject: Subject, verdict: Verdict, at: u64) -> Grant {
    Grant {
        power,
        subject,
        verdict,
        at: UnixSeconds(at),
    }
}

fn any_power() -> impl Strategy<Value = Power> {
    proptest::sample::select(<Power as WireCode>::ALL.to_vec())
}

fn any_subject() -> impl Strategy<Value = Subject> {
    prop_oneof![
        proptest::sample::select(vec![
            "/usr/bin/grim",
            "/usr/bin/wtype",
            "/opt/app/bin/tool",
            "/usr/bin/python3.13",
        ])
        .prop_map(program),
        proptest::sample::select(vec!["org.gnome.Loupe", "org.example.Tool"]).prop_map(flatpak),
    ]
}

fn any_verdict() -> impl Strategy<Value = Verdict> {
    prop_oneof![Just(Verdict::Allow), Just(Verdict::Deny)]
}

fn any_opinion() -> impl Strategy<Value = Option<Verdict>> {
    prop_oneof![Just(None), any_verdict().prop_map(Some)]
}

fn any_fallback() -> impl Strategy<Value = Fallback> {
    prop_oneof![Just(Fallback::Ask), Just(Fallback::Deny)]
}

fn any_grant() -> impl Strategy<Value = Grant> {
    (
        any_power(),
        any_subject(),
        any_verdict(),
        0u64..2_000_000_000,
    )
        .prop_map(|(power, subject, verdict, at)| grant(power, subject, verdict, at))
}

/// The floor and the book that say `floor_rule` and `book_rule` about `subject` and `power`,
/// with `fallback` as the floor's default for the power.
fn records(
    subject: &Subject,
    power: Power,
    floor_rule: Option<Verdict>,
    book_rule: Option<Verdict>,
    fallback: Fallback,
) -> (Floor, Book) {
    let floor = Floor::empty().with_fallback(power, fallback);
    let floor = match floor_rule {
        Some(verdict) => floor.with_rule(FloorRule {
            power,
            subject: subject.clone(),
            verdict,
        }),
        None => floor,
    };
    let book = match book_rule {
        Some(verdict) => Book::empty().with(grant(power, subject.clone(), verdict, 1)),
        None => Book::empty(),
    };
    (floor, book)
}

proptest! {
    /// The administrator's refusal, then the person's refusal, then the administrator's
    /// pre-approval, then the person's allow, then the floor's default, then a question.
    #[test]
    fn precedence_is_floor_then_book_then_default(
        power in any_power(),
        floor_rule in any_opinion(),
        book_rule in any_opinion(),
        fallback in any_fallback(),
    ) {
        let subject = program("/usr/bin/grim");
        let (floor, book) = records(&subject, power, floor_rule, book_rule, fallback);
        let got = decide(&floor, &book, &Identity::Named(subject), power);
        let want = match (floor_rule, book_rule) {
            (Some(Verdict::Deny), _) | (_, Some(Verdict::Deny)) => Decision::Closed,
            (Some(Verdict::Allow), _) | (_, Some(Verdict::Allow)) => Decision::Open,
            (None, None) => match fallback {
                Fallback::Deny => Decision::Closed,
                Fallback::Ask => Decision::Ask(Memory::Remembered),
            },
        };
        prop_assert_eq!(got, want);
    }

    /// Rows about another program or another power change nothing.
    #[test]
    fn other_rows_do_not_matter(
        power in any_power(),
        floor_rule in any_opinion(),
        book_rule in any_opinion(),
        fallback in any_fallback(),
        noise in proptest::collection::vec(any_grant(), 0..12),
    ) {
        let subject = program("/usr/bin/grim");
        let (floor, book) = records(&subject, power, floor_rule, book_rule, fallback);
        let quiet = decide(&floor, &book, &Identity::Named(subject.clone()), power);
        let noisy_book = noise
            .into_iter()
            .filter(|row| row.subject != subject || row.power != power)
            .fold(book, Book::with);
        prop_assert_eq!(decide(&floor, &noisy_book, &Identity::Named(subject), power), quiet);
    }

    /// A program that cannot be named is never let in, and its answer is never kept.
    #[test]
    fn an_unnamed_program_is_never_open_and_never_remembered(
        power in any_power(),
        fallback in any_fallback(),
        noise in proptest::collection::vec(any_grant(), 0..12),
    ) {
        let floor = Floor::empty().with_fallback(power, fallback);
        let book = noise.into_iter().fold(Book::empty(), Book::with);
        let got = decide(&floor, &book, &Identity::Unnamed, power);
        let want = match fallback {
            Fallback::Deny => Decision::Closed,
            Fallback::Ask => Decision::Ask(Memory::ThisTimeOnly),
        };
        prop_assert_eq!(got, want);
    }

    /// An interpreter's book rows are not read: it asks for this connection every time.
    #[test]
    fn an_interpreter_is_never_a_standing_guest(power in any_power(), verdict in any_verdict()) {
        let subject = program("/usr/bin/python3.13");
        let book = Book::empty().with(grant(power, subject.clone(), verdict, 1));
        let got = decide(&Floor::empty(), &book, &Identity::Named(subject), power);
        prop_assert_eq!(got, Decision::Ask(Memory::ThisTimeOnly));
    }

    /// A trusted service is in unless a record refuses it by name.
    #[test]
    fn a_trusted_program_is_open_unless_refused_by_name(
        power in any_power(),
        floor_rule in any_opinion(),
        book_rule in any_opinion(),
        fallback in any_fallback(),
    ) {
        let subject = program("/usr/libexec/transom");
        let (floor, book) = records(&subject, power, floor_rule, book_rule, fallback);
        let got = decide_trusted(&floor, &book, &Identity::Named(subject), power);
        let refused = floor_rule == Some(Verdict::Deny) || book_rule == Some(Verdict::Deny);
        prop_assert_eq!(got, if refused { Decision::Closed } else { Decision::Open });
    }

    /// The book's text and back is the same book.
    #[test]
    fn the_book_survives_its_text(rows in proptest::collection::vec(any_grant(), 0..40)) {
        let book = rows.into_iter().fold(Book::empty(), Book::with);
        let text = book.to_toml().expect("renders");
        let back = Book::from_toml(&text).expect("reads");
        prop_assert_eq!(back.dropped, 0);
        prop_assert_eq!(back.value, book);
    }
}

#[test]
fn an_interpreter_by_name_is_never_remembered_and_others_are() {
    let floor = Floor::empty();
    for name in [
        "python3",
        "python3.13",
        "perl",
        "ruby",
        "node",
        "deno",
        "bun",
        "java",
        "lua5.4",
        "sh",
        "bash",
        "zsh",
        "fish",
        "wine64",
    ] {
        let path = format!("/usr/bin/{name}");
        assert_eq!(
            floor.remembers(&program(&path)),
            Memory::ThisTimeOnly,
            "{path}"
        );
    }
    for name in [
        "grim",
        "wtype",
        "shell-tool",
        "bashful",
        "nodejs-helper",
        "cliphist",
    ] {
        let path = format!("/usr/bin/{name}");
        assert_eq!(
            floor.remembers(&program(&path)),
            Memory::Remembered,
            "{path}"
        );
    }
    assert_eq!(
        floor.remembers(&flatpak("org.gnome.Loupe")),
        Memory::Remembered
    );
}

#[test]
fn the_floor_can_add_to_the_programs_never_remembered() {
    let tool = ProgramPath::parse("/opt/app/bin/tool").expect("a path");
    let floor = Floor::empty().with_never_remembered(tool);
    assert_eq!(
        floor.remembers(&program("/opt/app/bin/tool")),
        Memory::ThisTimeOnly
    );
    assert_eq!(
        floor.remembers(&program("/opt/app/bin/other")),
        Memory::Remembered
    );
}

#[test]
fn a_full_book_drops_its_oldest_answer() {
    let rows = (0..MAX_GRANTS + 5).map(|n| {
        let path = format!("/usr/bin/p{n}");
        grant(Power::Screen, program(&path), Verdict::Allow, n as u64)
    });
    let book = rows.fold(Book::empty(), Book::with);
    assert_eq!(book.len(), MAX_GRANTS);
    assert_eq!(book.get(&program("/usr/bin/p0"), Power::Screen), None);
    assert_eq!(book.get(&program("/usr/bin/p4"), Power::Screen), None);
    assert_eq!(
        book.get(&program("/usr/bin/p5"), Power::Screen),
        Some(Verdict::Allow)
    );
    let newest = format!("/usr/bin/p{}", MAX_GRANTS + 4);
    assert_eq!(
        book.get(&program(&newest), Power::Screen),
        Some(Verdict::Allow)
    );
}

#[test]
fn a_later_answer_replaces_an_earlier_one_and_forgetting_removes_it() {
    let grim = program("/usr/bin/grim");
    let book = Book::empty()
        .with(grant(Power::Screen, grim.clone(), Verdict::Allow, 1))
        .with(grant(Power::Screen, grim.clone(), Verdict::Deny, 2));
    assert_eq!(book.len(), 1);
    assert_eq!(book.get(&grim, Power::Screen), Some(Verdict::Deny));
    assert_eq!(book.get(&grim, Power::Keys), None);
    assert!(book.without(&grim, Power::Screen).is_empty());
}

#[test]
fn a_program_path_has_one_spelling() {
    for good in ["/usr/bin/grim", "/a", "/opt/x y/bin/tool"] {
        assert!(ProgramPath::parse(good).is_ok(), "{good}");
    }
    let long = format!("/{}", "a".repeat(4096));
    for bad in [
        "",
        "usr/bin/grim",
        "/",
        "/usr//bin",
        "/usr/../bin/grim",
        "/usr/./bin",
        "/usr/bin/",
        "/usr/bin/grim (deleted)",
        "/usr/b\nin",
        long.as_str(),
    ] {
        assert!(ProgramPath::parse(bad).is_err(), "{bad:?}");
    }
    assert_eq!(
        ProgramPath::parse("/usr/bin/grim")
            .expect("a path")
            .file_name(),
        "grim"
    );
}

const BOOK: &str = r#"
version = 1
unknown_key = true

[[grant]]
power = "screen"
answer = "allow"
at = 1760619000
subject = { program = "/usr/bin/grim" }

[[grant]]
power = "clipboard"
answer = "deny"
at = 1760619100
subject = { app = "org.gnome.Loupe", sandbox = "flatpak" }
"#;

#[test]
fn the_book_reads_its_two_subject_forms_and_ignores_unknown_keys() {
    let loaded = Book::from_toml(BOOK).expect("reads");
    assert_eq!(loaded.dropped, 0);
    assert_eq!(
        loaded.value.get(&program("/usr/bin/grim"), Power::Screen),
        Some(Verdict::Allow)
    );
    assert_eq!(
        loaded
            .value
            .get(&flatpak("org.gnome.Loupe"), Power::Clipboard),
        Some(Verdict::Deny)
    );
}

#[test]
fn a_row_that_does_not_parse_is_dropped_never_let_in() {
    let text = r#"
version = 1
[[grant]]
power = "screen"
answer = "allow"
at = 1
subject = { program = "relative/path" }
[[grant]]
power = "telepathy"
answer = "allow"
at = 1
subject = { program = "/usr/bin/grim" }
[[grant]]
power = "screen"
answer = "allow"
at = 1
subject = { program = "/usr/bin/grim", app = "org.example.App", sandbox = "flatpak" }
[[grant]]
power = "keys"
answer = "allow"
at = 1
subject = { program = "/usr/bin/wtype" }
"#;
    let loaded = Book::from_toml(text).expect("reads");
    assert_eq!(loaded.dropped, 3);
    assert_eq!(loaded.value.len(), 1);
    assert_eq!(
        loaded.value.get(&program("/usr/bin/wtype"), Power::Keys),
        Some(Verdict::Allow)
    );
}

#[test]
fn a_book_of_another_version_or_a_wrong_shape_or_a_huge_one_is_refused() {
    assert!(matches!(
        Book::from_toml("version = 2\n"),
        Err(BookError::Version(2))
    ));
    assert!(matches!(Book::from_toml(""), Err(BookError::Parse(_))));
    assert!(matches!(
        Book::from_toml("version = \"one\""),
        Err(BookError::Parse(_))
    ));
    let huge = format!("version = 1\n{}", "#".repeat(MAX_FILE_BYTES));
    assert!(matches!(Book::from_toml(&huge), Err(BookError::TooLarge)));
}

#[test]
fn only_the_first_rows_of_a_long_book_are_read() {
    let row = |n: usize| {
        format!(
            "[[grant]]\npower = \"screen\"\nanswer = \"allow\"\nat = {n}\nsubject = {{ program = \"/usr/bin/p{n}\" }}\n"
        )
    };
    let text = (0..MAX_GRANTS + 20).fold("version = 1\n".to_owned(), |all, n| all + &row(n));
    assert!(
        text.len() <= MAX_FILE_BYTES,
        "the fixture fits the file cap"
    );
    let loaded = Book::from_toml(&text).expect("reads");
    assert_eq!(loaded.value.len(), MAX_GRANTS);
    assert_eq!(loaded.dropped, 20);
}

const FLOOR: &str = r#"
[default]
screen = "ask"
lock   = "deny"

[never_remember]
programs = ["/usr/bin/python3.13", "/opt/app/bin/tool"]

[[rule]]
power   = "clipboard"
subject = { program = "/usr/bin/cliphist" }
answer  = "allow"

[[rule]]
power   = "windows"
subject = { app = "org.example.Spy", sandbox = "snap" }
answer  = "deny"
"#;

#[test]
fn the_floor_reads_its_defaults_programs_and_rules() {
    let floor = Floor::from_toml(FLOOR).expect("reads");
    assert_eq!(floor.fallback(Power::Screen), Fallback::Ask);
    assert_eq!(floor.fallback(Power::Lock), Fallback::Deny);
    assert_eq!(floor.fallback(Power::Keys), Fallback::Ask);
    assert_eq!(
        floor.remembers(&program("/opt/app/bin/tool")),
        Memory::ThisTimeOnly
    );
    assert_eq!(
        floor.rule(&program("/usr/bin/cliphist"), Power::Clipboard),
        Some(Verdict::Allow)
    );
    let spy = Subject::Sandboxed {
        app: SandboxAppId::parse("org.example.Spy").expect("an app id"),
        engine: Engine::Snap,
    };
    assert_eq!(floor.rule(&spy, Power::Windows), Some(Verdict::Deny));
    assert_eq!(floor.rules().len(), 2);
    assert_eq!(Floor::from_toml("").expect("reads"), Floor::empty());
}

#[test]
fn the_floor_fails_loudly() {
    assert!(matches!(
        Floor::from_toml("[default]\nscreen = \"allow\"\n"),
        Err(FloorError::DefaultAllow(Power::Screen))
    ));
    assert!(matches!(
        Floor::from_toml("[default]\nscreen = \"maybe\"\n"),
        Err(FloorError::Parse(_))
    ));
    let bad_subject =
        "[[rule]]\npower = \"keys\"\nanswer = \"deny\"\nsubject = { program = \"usr/bin/x\" }\n";
    assert!(matches!(
        Floor::from_toml(bad_subject),
        Err(FloorError::Subject)
    ));
    let bad_never = "[never_remember]\nprograms = [\"python3\"]\n";
    assert!(matches!(
        Floor::from_toml(bad_never),
        Err(FloorError::Subject)
    ));
    let huge = format!("# {}", "x".repeat(MAX_FILE_BYTES));
    assert!(matches!(Floor::from_toml(&huge), Err(FloorError::TooLarge)));
}

#[test]
fn merging_floors_keeps_the_refusal_and_every_rule() {
    let a = Floor::empty()
        .with_fallback(Power::Lock, Fallback::Deny)
        .with_fallback(Power::Screen, Fallback::Ask);
    let b = Floor::empty()
        .with_fallback(Power::Lock, Fallback::Ask)
        .with_fallback(Power::Screen, Fallback::Deny)
        .with_rule(FloorRule {
            power: Power::Keys,
            subject: program("/usr/bin/wtype"),
            verdict: Verdict::Allow,
        });
    let both = a.merge(b);
    assert_eq!(both.fallback(Power::Lock), Fallback::Deny);
    assert_eq!(both.fallback(Power::Screen), Fallback::Deny);
    assert_eq!(both.rules().len(), 1);
}

#[test]
fn a_refusal_in_the_floor_beats_its_own_pre_approval() {
    let grim = program("/usr/bin/grim");
    let floor =
        [Verdict::Allow, Verdict::Deny]
            .into_iter()
            .fold(Floor::empty(), |floor, verdict| {
                floor.with_rule(FloorRule {
                    power: Power::Screen,
                    subject: grim.clone(),
                    verdict,
                })
            });
    assert_eq!(floor.rule(&grim, Power::Screen), Some(Verdict::Deny));
}
