//! Transport-independent identities, secret credentials, and acknowledged grants.

use std::{
    fmt,
    hash::{Hash, Hasher},
};

use rand::{rngs::OsRng, RngCore as _};
use serde::{Deserialize, Serialize};

macro_rules! public_id {
    ($name:ident, $length:expr, $doc:literal) => {
        #[doc = $doc]
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        pub struct $name([u8; $length]);

        impl $name {
            /// Encoded byte length.
            pub const BYTE_LENGTH: usize = $length;

            /// Generates a non-zero identifier from the operating system RNG.
            #[must_use]
            pub fn generate() -> Self {
                loop {
                    let mut bytes = [0_u8; $length];
                    OsRng.fill_bytes(&mut bytes);
                    if bytes.iter().any(|byte| *byte != 0) {
                        return Self(bytes);
                    }
                }
            }

            /// Constructs an identifier from exact bytes.
            #[must_use]
            pub const fn from_bytes(bytes: [u8; $length]) -> Self {
                Self(bytes)
            }

            /// Returns exact encoded bytes.
            #[must_use]
            pub const fn to_bytes(self) -> [u8; $length] {
                self.0
            }

            /// Whether this identifier is non-zero.
            #[must_use]
            pub fn is_valid(self) -> bool {
                self.0.iter().any(|byte| *byte != 0)
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(formatter, concat!(stringify!($name), "("))?;
                for byte in self.0.iter().take(4) {
                    write!(formatter, "{byte:02x}")?;
                }
                formatter.write_str("…)")
            }
        }
    };
}

public_id!(
    SessionId,
    16,
    "Stable identity of one host-process session."
);
public_id!(PeerId, 16, "Stable identity assigned to one admitted peer.");

macro_rules! secret {
    ($name:ident, $length:expr, $doc:literal) => {
        #[doc = $doc]
        #[derive(Clone, Copy, Eq, Serialize, Deserialize)]
        pub struct $name([u8; $length]);

        impl $name {
            /// Encoded byte length.
            pub const BYTE_LENGTH: usize = $length;

            /// Generates a credential from the operating system RNG.
            #[must_use]
            pub fn generate() -> Self {
                let mut bytes = [0_u8; $length];
                OsRng.fill_bytes(&mut bytes);
                Self(bytes)
            }

            /// Constructs a credential from exact bytes for decoding and tests.
            #[must_use]
            pub const fn from_bytes(bytes: [u8; $length]) -> Self {
                Self(bytes)
            }

            /// Copies credential bytes for encrypted encoding or protected storage.
            #[must_use]
            pub const fn to_bytes(self) -> [u8; $length] {
                self.0
            }

            /// Compares credentials without a data-dependent early return.
            #[must_use]
            pub fn matches(self, presented: Self) -> bool {
                constant_time_equal(&self.0, &presented.0)
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(concat!(stringify!($name), "([REDACTED])"))
            }
        }

        impl PartialEq for $name {
            fn eq(&self, other: &Self) -> bool {
                constant_time_equal(&self.0, &other.0)
            }
        }

        impl Hash for $name {
            fn hash<H: Hasher>(&self, state: &mut H) {
                self.0.hash(state);
            }
        }
    };
}

secret!(
    InviteToken,
    16,
    "One-time bearer secret carried by a direct code."
);
secret!(
    ReconnectCredential,
    32,
    "Private credential rotated after every successful reconnection."
);

fn constant_time_equal(left: &[u8], right: &[u8]) -> bool {
    use subtle::ConstantTimeEq as _;
    bool::from(left.ct_eq(right))
}

/// Credential presented by an encrypted connection during shared authentication.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmissionCredential {
    /// First admission through a private direct invitation.
    Invite(InviteToken),
    /// Reclaim a previously admitted peer identity.
    Reconnect(ReconnectCredential),
}

/// Successful shared authentication result; games still decide seats and capacity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdmissionGrant {
    /// Stable identity games can bind to their own seat model.
    pub peer: PeerId,
    /// Credential already atomically persisted by the client before acknowledgement.
    pub reconnect_credential: ReconnectCredential,
    /// Whether this reclaimed an existing identity.
    pub reconnected: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credentials_and_debug_are_redacted() {
        let invite = InviteToken::from_bytes([7; 16]);
        let reconnect = ReconnectCredential::from_bytes([9; 32]);
        assert_eq!(format!("{invite:?}"), "InviteToken([REDACTED])");
        assert_eq!(format!("{reconnect:?}"), "ReconnectCredential([REDACTED])");
    }
}
