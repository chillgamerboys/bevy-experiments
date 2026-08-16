//! Temporary password admission for discoverable trusted-network sessions.

use std::{
    collections::{BTreeMap, VecDeque},
    fmt,
    net::IpAddr,
    time::Duration,
};

use argon2::{
    password_hash::{
        rand_core::OsRng, PasswordHash, PasswordHasher as _, PasswordVerifier as _, SaltString,
    },
    Argon2,
};
use zeroize::Zeroize as _;

const WINDOW: Duration = Duration::from_secs(60);
const COOLDOWN: Duration = Duration::from_secs(30);
const PER_SOURCE_FAILURES: usize = 5;
const GLOBAL_FAILURES: usize = 30;

/// Validated plaintext session password that redacts and zeroes its allocation.
pub struct SessionPassword(String);

impl SessionPassword {
    /// Accepts 8–64 printable ASCII characters.
    pub fn new(value: impl Into<String>) -> Result<Self, PasswordPolicyError> {
        let value = value.into();
        if !(8..=64).contains(&value.len()) {
            return Err(PasswordPolicyError::InvalidLength);
        }
        if !value
            .bytes()
            .all(|byte| byte.is_ascii_graphic() || byte == b' ')
        {
            return Err(PasswordPolicyError::NotPrintableAscii);
        }
        if value.starts_with(' ') || value.ends_with(' ') {
            return Err(PasswordPolicyError::SurroundingWhitespace);
        }
        Ok(Self(value))
    }

    /// Borrows plaintext only for the pinned encrypted admission handshake.
    #[must_use]
    pub fn expose_for_encrypted_transport(&self) -> &str {
        &self.0
    }

    pub(crate) fn expose_for_verification(&self) -> &str {
        self.expose_for_encrypted_transport()
    }
}

impl Drop for SessionPassword {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

impl fmt::Debug for SessionPassword {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SessionPassword([REDACTED])")
    }
}

/// Host-owned salted Argon2id verifier plus bounded attempt tracking.
pub struct SessionPasswordVerifier {
    encoded_hash: String,
    source_failures: BTreeMap<IpAddr, VecDeque<Duration>>,
    global_failures: VecDeque<Duration>,
    cooldowns: BTreeMap<IpAddr, Duration>,
}

impl SessionPasswordVerifier {
    /// Hashes a validated password with a fresh salt; plaintext is not retained.
    pub fn new(password: &SessionPassword) -> Result<Self, PasswordPolicyError> {
        let salt = SaltString::generate(&mut OsRng);
        let encoded_hash = Argon2::default()
            .hash_password(password.expose_for_verification().as_bytes(), &salt)
            .map_err(|_error| PasswordPolicyError::HashUnavailable)?
            .to_string();
        Ok(Self {
            encoded_hash,
            source_failures: BTreeMap::new(),
            global_failures: VecDeque::new(),
            cooldowns: BTreeMap::new(),
        })
    }

    /// Verifies one attempt, enforcing per-source and global rolling limits.
    pub fn verify(
        &mut self,
        source: IpAddr,
        now: Duration,
        presented: &SessionPassword,
    ) -> Result<(), PasswordAttemptError> {
        self.prune(now);
        if self
            .cooldowns
            .get(&source)
            .is_some_and(|until| *until > now)
            || self.global_failures.len() >= GLOBAL_FAILURES
        {
            return Err(PasswordAttemptError::RateLimited);
        }
        let parsed = PasswordHash::new(&self.encoded_hash)
            .map_err(|_error| PasswordAttemptError::Unavailable)?;
        if Argon2::default()
            .verify_password(presented.expose_for_verification().as_bytes(), &parsed)
            .is_ok()
        {
            self.source_failures.remove(&source);
            self.cooldowns.remove(&source);
            return Ok(());
        }
        let failures = self.source_failures.entry(source).or_default();
        failures.push_back(now);
        self.global_failures.push_back(now);
        if failures.len() >= PER_SOURCE_FAILURES {
            self.cooldowns.insert(source, now + COOLDOWN);
        }
        Err(PasswordAttemptError::Rejected)
    }

    fn prune(&mut self, now: Duration) {
        let cutoff = now.saturating_sub(WINDOW);
        self.global_failures.retain(|attempt| *attempt >= cutoff);
        self.source_failures.retain(|_, attempts| {
            attempts.retain(|attempt| *attempt >= cutoff);
            !attempts.is_empty()
        });
        self.cooldowns.retain(|_, until| *until > now);
    }
}

impl fmt::Debug for SessionPasswordVerifier {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SessionPasswordVerifier")
            .field("encoded_hash", &"[REDACTED VERIFIER]")
            .field("tracked_sources", &self.source_failures.len())
            .finish_non_exhaustive()
    }
}

/// Why a host-selected password violates the shared temporary-session policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasswordPolicyError {
    /// Password was shorter than 8 or longer than 64 bytes.
    InvalidLength,
    /// Password contained non-printable or non-ASCII characters.
    NotPrintableAscii,
    /// Leading or trailing spaces would be ambiguous in UI.
    SurroundingWhitespace,
    /// Platform cryptography could not create a verifier.
    HashUnavailable,
}

impl fmt::Display for PasswordPolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidLength => "session password must contain 8 to 64 characters",
            Self::NotPrintableAscii => "session password must use printable ASCII",
            Self::SurroundingWhitespace => "session password cannot start or end with a space",
            Self::HashUnavailable => "session password verifier is unavailable",
        })
    }
}

impl std::error::Error for PasswordPolicyError {}

/// Generic admission result that avoids revealing password details.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasswordAttemptError {
    /// Password did not match.
    Rejected,
    /// Source or global attempts exceeded the configured budget.
    RateLimited,
    /// Stored verifier could not be used.
    Unavailable,
}

impl fmt::Display for PasswordAttemptError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Rejected => "session password was rejected",
            Self::RateLimited => "too many session password attempts",
            Self::Unavailable => "session password verification is unavailable",
        })
    }
}

impl std::error::Error for PasswordAttemptError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verifier_accepts_exact_password_and_redacts() {
        let password = SessionPassword::new("copper comet").expect("valid password");
        let mut verifier = SessionPasswordVerifier::new(&password).expect("hash succeeds");
        assert!(verifier
            .verify(
                "127.0.0.1".parse().expect("valid IP"),
                Duration::ZERO,
                &password
            )
            .is_ok());
        assert!(!format!("{verifier:?}").contains("copper comet"));
        assert!(!format!("{password:?}").contains("copper comet"));
    }

    #[test]
    fn fifth_failure_starts_source_cooldown() {
        let correct = SessionPassword::new("copper comet").expect("valid password");
        let wrong = SessionPassword::new("silver comet").expect("valid password");
        let mut verifier = SessionPasswordVerifier::new(&correct).expect("hash succeeds");
        let source = "127.0.0.1".parse().expect("valid IP");
        for second in 0..5 {
            assert_eq!(
                verifier.verify(source, Duration::from_secs(second), &wrong),
                Err(PasswordAttemptError::Rejected)
            );
        }
        assert_eq!(
            verifier.verify(source, Duration::from_secs(5), &correct),
            Err(PasswordAttemptError::RateLimited)
        );
        assert!(verifier
            .verify(source, Duration::from_secs(35), &correct)
            .is_ok());
    }
}
