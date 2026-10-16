//! What a `Set` on a key of the Privacy module may do, and the edit it becomes.

use super::edit::PrivacyEdit;
use super::keys::{OFF, ON};
use casement_core::policy::Power;
use casement_core::privilege::Subject;

/// What one key lets a caller do with `Set`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Write {
    /// Nothing: a read-out, the administrator's row or an empty section.
    Refused(Why),
    /// Switch the program's power on or off.
    Choose(Subject, Power),
    /// Forget the answer so the program is asked again.
    Forget(Subject, Power),
}

/// Why a key cannot be changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Why {
    /// The administrator set it.
    Administrator,
    /// It is only text.
    ReadOut,
}

/// The edit a `Set` of `value` on a key with `write` asks for, or why it is not one.
pub fn edit_of(write: &Write, value: &toml::Value) -> Result<PrivacyEdit, EditFault> {
    match write {
        Write::Refused(Why::Administrator) => Err(EditFault::Administrator),
        Write::Refused(Why::ReadOut) => Err(EditFault::ReadOut),
        Write::Forget(subject, power) => Ok(PrivacyEdit::Forget {
            subject: subject.clone(),
            power: *power,
        }),
        Write::Choose(subject, power) => match value {
            toml::Value::String(word) if word == ON => Ok(PrivacyEdit::Allow {
                subject: subject.clone(),
                power: *power,
            }),
            toml::Value::String(word) if word == OFF => Ok(PrivacyEdit::Deny {
                subject: subject.clone(),
                power: *power,
            }),
            toml::Value::Boolean(true) => Ok(PrivacyEdit::Allow {
                subject: subject.clone(),
                power: *power,
            }),
            toml::Value::Boolean(false) => Ok(PrivacyEdit::Deny {
                subject: subject.clone(),
                power: *power,
            }),
            _ => Err(EditFault::BadValue),
        },
    }
}

/// Why a `Set` is not an edit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditFault {
    /// The administrator set the row.
    Administrator,
    /// The key is only text.
    ReadOut,
    /// A switch is `on` or `off`.
    BadValue,
}

#[cfg(test)]
mod tests {
    use super::*;
    use casement_core::privilege::ProgramPath;

    fn program(path: &str) -> Subject {
        Subject::Program(ProgramPath::parse(path).expect("a path"))
    }

    #[test]
    fn a_set_is_an_edit_only_where_the_row_may_be_changed() {
        let grim = program("/usr/bin/grim");
        let choose = Write::Choose(grim.clone(), Power::Screen);
        let on = toml::Value::String(ON.to_owned());
        let off = toml::Value::String(OFF.to_owned());
        assert!(matches!(
            edit_of(&choose, &on),
            Ok(PrivacyEdit::Allow { .. })
        ));
        assert!(matches!(
            edit_of(&choose, &off),
            Ok(PrivacyEdit::Deny { .. })
        ));
        assert!(matches!(
            edit_of(&choose, &toml::Value::Boolean(false)),
            Ok(PrivacyEdit::Deny { .. })
        ));
        assert_eq!(
            edit_of(&choose, &toml::Value::Integer(1)),
            Err(EditFault::BadValue)
        );
        assert!(matches!(
            edit_of(
                &Write::Forget(grim, Power::Screen),
                &toml::Value::Boolean(true)
            ),
            Ok(PrivacyEdit::Forget { .. })
        ));
        assert_eq!(
            edit_of(&Write::Refused(Why::Administrator), &on),
            Err(EditFault::Administrator)
        );
        assert_eq!(
            edit_of(&Write::Refused(Why::ReadOut), &on),
            Err(EditFault::ReadOut)
        );
    }
}
