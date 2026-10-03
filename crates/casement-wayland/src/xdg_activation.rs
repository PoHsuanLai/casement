//! `xdg_activation_v1`: tokens and activation requests. A request activates only under A7's
//! rule: the token was asked for by a client that held a recent press, and it is used while
//! recent. smithay stamps tokens with the wall clock (`Instant::now`), so token age is judged
//! against the session's `Mono` kept beside each token here, never smithay's stamp (A36).
//!
//! The verdict is a [`TokenCheck`] the model acts on: `Valid` activates, anything else only asks
//! for the app's attention (03 A7).

use crate::serial::PressRecord;
use crate::states::GlobalFilter;
use casement_core::focus::TokenCheck;
use casement_core::time::Mono;
use casement_core::{ClientKey, Event, ToplevelRequest, WindowId};
use smithay::reexports::wayland_protocols::xdg::activation::v1::server::xdg_activation_v1::XdgActivationV1;
use smithay::reexports::wayland_server::{Dispatch, DisplayHandle, GlobalDispatch};
use smithay::wayland::GlobalData;
use smithay::wayland::xdg_activation::{
    XdgActivationHandler, XdgActivationState, XdgActivationToken, XdgActivationTokenData,
};
use std::collections::BTreeMap;
use std::time::Duration;

/// How long a token stays good: the press that earned it and the use it is put to must both be
/// this recent (A7).
pub const TOKEN_LIFETIME: Duration = Duration::from_secs(5);

/// How long a spent-looking token is remembered, so a late use is still judged (`Stale`, which
/// asks for attention) instead of vanishing.
pub const TOKEN_MEMORY: Duration = Duration::from_secs(60);

/// How many live tokens one client may hold: more are refused.
pub const TOKENS_PER_CLIENT: usize = 32;

/// `xdg_activation_v1`.
#[derive(Debug)]
pub struct XdgActivationPart {
    /// smithay's state.
    pub state: XdgActivationState,
    /// Each live token, on the session clock.
    pub issued: BTreeMap<String, IssuedToken>,
}

/// What the compositor knew when it issued a token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IssuedToken {
    /// When it was issued, on the session clock.
    pub at: Mono,
    /// The client that asked for it, when known.
    pub by: Option<ClientKey>,
    /// Whether that client held a recent press.
    pub proof: PressProof,
}

/// Whether a token was earned by a recent press.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PressProof {
    /// The token's serial answers a recent press delivered to the asking client.
    Pressed,
    /// It does not (no serial, an unknown one, an old one, or another client's).
    Unproven,
}

/// What a token request knew of its asker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenAsk {
    /// The client that asked, when known.
    pub requester: Option<ClientKey>,
    /// Whether it held a recent press.
    pub proof: PressProof,
}

impl XdgActivationPart {
    /// Creates the global (version 1). Every connection sees it.
    pub fn new<D>(display: &DisplayHandle, role_filter: GlobalFilter) -> XdgActivationPart
    where
        D: GlobalDispatch<XdgActivationV1, GlobalData>
            + Dispatch<XdgActivationV1, GlobalData>
            + XdgActivationHandler
            + 'static,
    {
        let _ = role_filter;
        XdgActivationPart {
            state: XdgActivationState::new::<D>(display),
            issued: BTreeMap::new(),
        }
    }
}

/// Whether a token is issued.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenVerdict {
    /// Issue it (it is judged when used).
    Issue,
    /// Refuse it.
    Refuse,
}

/// Whether the press `record` (what the seat remembers of the serial the token carried, for the
/// asking client) earns a token requested at `now`.
pub fn proof_of(record: Option<&PressRecord>, now: Mono) -> PressProof {
    match record {
        Some(press) if now.since(press.at) <= TOKEN_LIFETIME => PressProof::Pressed,
        Some(_) | None => PressProof::Unproven,
    }
}

/// The verdict on a token `issued` (if the compositor issued it) used at `now`: `Valid` only
/// for a token earned by a press and still recent; everything else is `Stale` (an unknown token
/// is a bad one, not a missing one).
pub fn judge(issued: Option<&IssuedToken>, now: Mono) -> TokenCheck {
    match issued {
        Some(token)
            if token.proof == PressProof::Pressed && now.since(token.at) <= TOKEN_LIFETIME =>
        {
            TokenCheck::Valid
        }
        Some(_) | None => TokenCheck::Stale,
    }
}

/// Forgets the tokens older than [`TOKEN_MEMORY`], in the table and in smithay's.
fn forget_old(part: &mut XdgActivationPart, now: Mono) {
    let expired: Vec<String> = part
        .issued
        .iter()
        .filter(|(_, token)| now.since(token.at) > TOKEN_MEMORY)
        .map(|(key, _)| key.clone())
        .collect();
    if expired.is_empty() {
        return;
    }
    part.issued.retain(|key, _| !expired.contains(key));
    part.state
        .retain_tokens(|token, _| !expired.iter().any(|key| key == token.as_str()));
}

/// A client asked for a token at `now`. It is issued unless its asker already holds
/// [`TOKENS_PER_CLIENT`] live ones. (`data` is smithay's stamp, ignored: A36.)
pub fn on_token_request(
    part: &mut XdgActivationPart,
    token: &XdgActivationToken,
    data: &XdgActivationTokenData,
    now: Mono,
    ask: TokenAsk,
) -> TokenVerdict {
    let _ = data;
    forget_old(part, now);
    let held = part
        .issued
        .values()
        .filter(|held| ask.requester.is_some() && held.by == ask.requester)
        .count();
    if held >= TOKENS_PER_CLIENT {
        return TokenVerdict::Refuse;
    }
    part.issued.insert(
        token.as_str().to_owned(),
        IssuedToken {
            at: now,
            by: ask.requester,
            proof: ask.proof,
        },
    );
    TokenVerdict::Issue
}

/// A token is being used at `now`: it is judged on the session clock and spent (tokens are
/// one-use), in the table and in smithay's.
pub fn use_token(
    part: &mut XdgActivationPart,
    token: &XdgActivationToken,
    now: Mono,
) -> TokenCheck {
    let check = judge(part.issued.get(token.as_str()), now);
    part.issued.remove(token.as_str());
    part.state.remove_token(token);
    check
}

/// `window` asked to be activated, and its token checked out as `check`: the model gets the
/// request (`Valid` activates, anything else marks attention, A7).
pub fn on_activate(window: WindowId, check: TokenCheck) -> Vec<Event> {
    vec![Event::ToplevelRequest {
        window,
        request: ToplevelRequest::Activate(check),
    }]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serial::PressKind;
    use casement_core::input::{DeviceClass, Origin};

    fn at(ms: u64) -> Mono {
        Mono(Duration::from_millis(ms))
    }

    fn press(ms: u64) -> PressRecord {
        PressRecord {
            client: ClientKey(1),
            at: at(ms),
            kind: PressKind::Button,
            origin: Origin::Physical(DeviceClass::Pointer),
        }
    }

    fn issued(ms: u64, proof: PressProof) -> IssuedToken {
        IssuedToken {
            at: at(ms),
            by: Some(ClientKey(1)),
            proof,
        }
    }

    /// The A7 rule on session time: a recent press earns the token, and the token is good for
    /// five seconds; nothing here reads a wall clock.
    #[test]
    fn a_token_is_valid_only_when_pressed_and_recent() {
        let rows = [
            (
                "fresh, pressed",
                Some(issued(1_000, PressProof::Pressed)),
                1_500,
                TokenCheck::Valid,
            ),
            (
                "at the limit",
                Some(issued(1_000, PressProof::Pressed)),
                6_000,
                TokenCheck::Valid,
            ),
            (
                "a millisecond over",
                Some(issued(1_000, PressProof::Pressed)),
                6_001,
                TokenCheck::Stale,
            ),
            (
                "fresh, no press",
                Some(issued(1_000, PressProof::Unproven)),
                1_100,
                TokenCheck::Stale,
            ),
            ("unknown token", None, 1_100, TokenCheck::Stale),
            (
                "used before issue (clock never goes back)",
                Some(issued(2_000, PressProof::Pressed)),
                1_000,
                TokenCheck::Valid,
            ),
        ];
        for (case, token, now, want) in rows {
            assert_eq!(judge(token.as_ref(), at(now)), want, "{case}");
        }
    }

    #[test]
    fn a_press_earns_a_token_only_while_recent() {
        let rows = [
            ("no serial answered", None, 100, PressProof::Unproven),
            ("just now", Some(press(100)), 120, PressProof::Pressed),
            ("five seconds", Some(press(100)), 5_100, PressProof::Pressed),
            ("too old", Some(press(100)), 5_101, PressProof::Unproven),
        ];
        for (case, record, now, want) in rows {
            assert_eq!(proof_of(record.as_ref(), at(now)), want, "{case}");
        }
    }

    #[test]
    fn an_activation_is_a_toplevel_request_with_the_verdict() {
        assert_eq!(
            on_activate(WindowId(4), TokenCheck::Stale),
            vec![Event::ToplevelRequest {
                window: WindowId(4),
                request: ToplevelRequest::Activate(TokenCheck::Stale)
            }]
        );
    }
}
