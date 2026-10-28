//! What a toplevel's client declared, as our types: its app id (a claim, never a proof), its
//! role and modality from the dialog hint, and its size limits.

use casement_core::ClientKey;
use casement_core::geometry::LogicalSize;
use casement_core::window::SizeLimits;
use casement_protocols::vocab::{Modality, WindowRole};
use porter_core::ClaimedId;
use smithay::wayland::shell::xdg::dialog::ToplevelDialogHint;

/// The id a client declared with `set_app_id`, or one of its own making when it declared none
/// (or one that is not a well-formed id): a client with no name is its own app, so two such
/// clients never share a Dock tile.
pub fn claimed_of(app_id: Option<&str>, client: ClientKey) -> ClaimedId {
    app_id
        .and_then(|text| ClaimedId::parse(text).ok())
        .unwrap_or_else(|| {
            ClaimedId::parse(&format!("unnamed.client-{}", client.0))
                .expect("a short run of letters, digits, dots and a hyphen is a well-formed id")
        })
}

/// The window role the dialog hint implies: a dialog or a modal one is a `Dialog`; a toplevel
/// with no `xdg_dialog` object is a `Document` until `quire_window_v1.set_role` says otherwise.
pub fn role_of(hint: ToplevelDialogHint) -> WindowRole {
    match hint {
        ToplevelDialogHint::Unknown => WindowRole::Document,
        ToplevelDialogHint::Dialog | ToplevelDialogHint::Modal => WindowRole::Dialog,
    }
}

/// What the dialog hint says the window blocks: a modal dialog blocks its parent (a sheet), a
/// plain dialog blocks nothing but is known to be one, anything else is not a dialog.
pub fn modality_of(hint: ToplevelDialogHint) -> Modality {
    match hint {
        ToplevelDialogHint::Unknown => Modality::None,
        ToplevelDialogHint::Dialog => Modality::Modeless,
        ToplevelDialogHint::Modal => Modality::WindowModal,
    }
}

/// The limits `set_min_size` and `set_max_size` describe: zero on the minimum's axis is no
/// minimum; zero on the maximum's axis is no maximum on that axis, and none on both is
/// unbounded. Negative sizes (which smithay lets through to us as written) count as zero.
pub fn limits_of(min: (i32, i32), max: (i32, i32)) -> SizeLimits {
    let side = |value: i32| u32::try_from(value).unwrap_or(0);
    let (max_w, max_h) = (side(max.0), side(max.1));
    SizeLimits {
        min: LogicalSize {
            w: side(min.0),
            h: side(min.1),
        },
        max: match (max_w, max_h) {
            (0, 0) => None,
            (w, h) => Some(LogicalSize {
                w: if w == 0 { u32::MAX } else { w },
                h: if h == 0 { u32::MAX } else { h },
            }),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_declared_id_is_kept_and_a_missing_one_is_per_client() {
        let rows: [(&str, Option<&str>, u64, &str); 5] = [
            ("declared", Some("org.quire.Mail"), 1, "org.quire.Mail"),
            ("none", None, 7, "unnamed.client-7"),
            ("empty", Some(""), 8, "unnamed.client-8"),
            ("slash", Some("a/b"), 9, "unnamed.client-9"),
            ("control", Some("a\nb"), 10, "unnamed.client-10"),
        ];
        for (case, app_id, client, want) in rows {
            assert_eq!(
                claimed_of(app_id, ClientKey(client)).as_str(),
                want,
                "{case}"
            );
        }
    }

    #[test]
    fn the_dialog_hint_gives_role_and_modality() {
        let rows = [
            (
                ToplevelDialogHint::Unknown,
                WindowRole::Document,
                Modality::None,
            ),
            (
                ToplevelDialogHint::Dialog,
                WindowRole::Dialog,
                Modality::Modeless,
            ),
            (
                ToplevelDialogHint::Modal,
                WindowRole::Dialog,
                Modality::WindowModal,
            ),
        ];
        for (hint, role, modality) in rows {
            assert_eq!(role_of(hint), role, "{hint:?}");
            assert_eq!(modality_of(hint), modality, "{hint:?}");
        }
    }

    #[test]
    fn limits_treat_zero_as_unbounded_per_axis() {
        let none = limits_of((0, 0), (0, 0));
        assert_eq!(none.min, LogicalSize { w: 0, h: 0 });
        assert_eq!(none.max, None);
        let both = limits_of((200, 100), (800, 600));
        assert_eq!(both.min, LogicalSize { w: 200, h: 100 });
        assert_eq!(both.max, Some(LogicalSize { w: 800, h: 600 }));
        let one_axis = limits_of((0, 0), (800, 0));
        assert_eq!(
            one_axis.max,
            Some(LogicalSize {
                w: 800,
                h: u32::MAX
            })
        );
        let negative = limits_of((-5, 3), (-1, -1));
        assert_eq!(negative.min, LogicalSize { w: 0, h: 3 });
        assert_eq!(negative.max, None);
    }
}
