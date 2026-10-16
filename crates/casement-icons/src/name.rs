//! An icon name is a name, never a path.

/// The longest icon name accepted, in bytes. Theme names are short; a longer one is not a name.
const NAME_LIMIT: usize = 128;

/// A plain icon name, the kind an icon theme is searched by: ASCII letters, digits and `-`, `_`,
/// `.` and `+`, not empty, not starting with a dot (no `.hidden`, no `..`), at most 128 bytes.
/// A `/` is never in one, so a name cannot lead out of the theme folders.
///
/// A client chooses the names it sets through `xdg_toplevel_icon`, so this is the edge that keeps
/// `/dev/zero` or `../../etc/x` from being looked up as an icon.
///
/// ```
/// use casement_icons::IconName;
///
/// assert!(IconName::parse("org.gnome.Nautilus").is_some());
/// assert!(IconName::parse("/dev/zero").is_none());
/// assert!(IconName::parse("../x").is_none());
/// assert!(IconName::parse(".hidden").is_none());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IconName<'a>(&'a str);

impl<'a> IconName<'a> {
    /// `text` as a name, or nothing when it is not a plain one.
    pub fn parse(text: &'a str) -> Option<IconName<'a>> {
        let plain = !text.is_empty()
            && text.len() <= NAME_LIMIT
            && !text.starts_with('.')
            && text
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"-_.+".contains(&byte));
        plain.then_some(IconName(text))
    }

    /// The name.
    pub fn as_str(&self) -> &'a str {
        self.0
    }
}
