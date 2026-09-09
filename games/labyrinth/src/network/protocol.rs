//! Labyrinth wire vocabulary, distinct from transport and pure combat rules.

use bevy::prelude::*;
use bevy_game_session::{InviteToken, PeerId, ReconnectCredential, SessionId};
use serde::{Deserialize, Serialize};
use zeroize::Zeroize as _;

pub(super) const GAME_ID: &str = "gamekit-labyrinth";
pub(super) const PROTOCOL: &str = "1";
pub(super) const SCHEMA: &str = "labyrinth/v1;four-seats;attempt-scoped-persisted-admission-ack;encounter-turn-watermark;typed-outcomes";

#[derive(Serialize)]
pub(super) struct WirePassword(pub String);

impl<'de> Deserialize<'de> for WirePassword {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl serde::de::Visitor<'_> for Visitor {
            type Value = WirePassword;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("a bounded temporary passphrase")
            }
            fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
                if value.len() > 64 {
                    return Err(E::custom("passphrase exceeds limit"));
                }
                Ok(WirePassword(value.to_owned()))
            }
        }
        deserializer.deserialize_str(Visitor)
    }
}
impl Drop for WirePassword {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}
impl std::fmt::Debug for WirePassword {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Password([REDACTED])")
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) enum Credential {
    Invite(InviteToken),
    Password(WirePassword),
    Reconnect(ReconnectCredential),
}

#[derive(Message, Debug, Serialize, Deserialize)]
pub(super) struct Hello {
    pub session: SessionId,
    pub attempt: SessionId,
    pub fingerprint: [u8; 32],
    pub credential: Credential,
}

#[derive(Message, Debug, Clone, Copy, Serialize, Deserialize)]
pub(super) struct Offer {
    pub session: SessionId,
    pub attempt: SessionId,
    pub peer: PeerId,
    pub slot: u8,
    pub credential: ReconnectCredential,
    pub reconnected: bool,
}

#[derive(Message, Debug, Clone, Copy, Serialize, Deserialize)]
pub(super) struct Persisted {
    pub attempt: SessionId,
    pub credential: ReconnectCredential,
}

#[derive(Message, Debug, Clone, Copy, Serialize, Deserialize)]
pub(super) struct Admitted {
    pub session: SessionId,
    pub attempt: SessionId,
    pub peer: PeerId,
    pub slot: u8,
    pub reconnected: bool,
}

#[derive(Message, Debug, Clone, Copy, Serialize, Deserialize)]
pub(super) enum Refused {
    Admission,
    Incompatible,
    Full,
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
pub(super) struct Refusal {
    pub attempt: SessionId,
    pub reason: Refused,
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
pub(super) struct SnapshotEnvelope {
    pub attempt: SessionId,
    pub snapshot: crate::session::SessionSnapshot,
}

impl Refused {
    pub fn notice(self) -> &'static str {
        match self {
            Self::Admission => "The host refused admission. Check the temporary passphrase or use a fresh invitation.",
            Self::Incompatible => "This host uses a different game or rules version.",
            Self::Full => "The party is full or already in combat. Reserved players can reconnect.",
        }
    }
}

#[derive(Message, Debug, Clone, Copy, Serialize, Deserialize)]
pub(super) struct Closed {
    pub attempt: SessionId,
}
#[derive(Message, Debug, Clone, Copy, Serialize, Deserialize)]
pub(super) struct LeaveSession;
