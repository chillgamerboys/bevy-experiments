//! Injected reconnect-credential persistence.

use std::{
    fmt,
    fs::OpenOptions,
    io::{self, Read as _, Write as _},
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};

use atomicwrites::{AllowOverwrite, AtomicFile};
use bevy::prelude::Resource;
use serde::{Deserialize, Serialize};

use crate::{CertificateFingerprint, DirectEndpoint, PeerId, ReconnectCredential, SessionId};

const MAX_FILE_BYTES: usize = 4096;

/// Direct endpoint and certificate identity verified before a credential is stored.
#[derive(Resource, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReconnectEndpointBinding {
    /// Host endpoint.
    pub endpoint: DirectEndpoint,
    /// Exact SPKI pin.
    pub certificate_fingerprint: CertificateFingerprint,
    /// Exact verified certificate expiry.
    pub certificate_expires_unix_seconds: u64,
}

impl ReconnectEndpointBinding {
    /// Validates a reconnect endpoint binding.
    pub fn new(
        endpoint: DirectEndpoint,
        certificate_fingerprint: CertificateFingerprint,
        certificate_expires_unix_seconds: u64,
    ) -> Result<Self, CredentialStoreError> {
        if certificate_expires_unix_seconds == 0 {
            return Err(CredentialStoreError::Malformed);
        }
        Ok(Self {
            endpoint,
            certificate_fingerprint,
            certificate_expires_unix_seconds,
        })
    }
}

/// Credential material persisted by a reconnecting client.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredReconnectCredential {
    /// Concrete host session that issued this credential.
    pub session_id: SessionId,
    /// Verified endpoint and certificate binding.
    pub endpoint_binding: ReconnectEndpointBinding,
    /// Stable peer identity issued by shared security.
    pub peer_id: PeerId,
    /// Current rotating private credential.
    pub reconnect_credential: ReconnectCredential,
}

impl StoredReconnectCredential {
    /// Whether the pinned certificate is no longer usable at `unix_seconds`.
    #[must_use]
    pub const fn is_expired_at(&self, unix_seconds: u64) -> bool {
        unix_seconds >= self.endpoint_binding.certificate_expires_unix_seconds
    }
}

impl fmt::Debug for StoredReconnectCredential {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("StoredReconnectCredential")
            .field("session_id", &self.session_id)
            .field("endpoint_binding", &self.endpoint_binding)
            .field("peer_id", &self.peer_id)
            .field("reconnect_credential", &self.reconnect_credential)
            .finish()
    }
}

/// Injected persistence boundary for temporary reconnect state.
pub trait ReconnectCredentialStore: Send + Sync + 'static {
    /// Reads the current credential, or `None` when none exists.
    fn load(&self) -> Result<Option<StoredReconnectCredential>, CredentialStoreError>;

    /// Atomically replaces the current credential after admission or rotation.
    fn store_atomically(
        &self,
        credential: StoredReconnectCredential,
    ) -> Result<(), CredentialStoreError>;

    /// Deletes the credential only when it belongs to the supplied session.
    fn delete_if_session(&self, session_id: SessionId) -> Result<bool, CredentialStoreError>;

    /// Deletes the credential only after certificate expiry.
    fn delete_if_expired(&self, unix_seconds: u64) -> Result<bool, CredentialStoreError>;
}

/// Bevy resource wrapping an application- or test-owned store.
#[derive(Resource, Clone)]
pub struct ReconnectCredentialStorage(Arc<dyn ReconnectCredentialStore>);

impl ReconnectCredentialStorage {
    /// Wraps a concrete credential store.
    #[must_use]
    pub fn new(store: impl ReconnectCredentialStore) -> Self {
        Self(Arc::new(store))
    }

    /// Borrows the injected store.
    #[must_use]
    pub fn store(&self) -> &dyn ReconnectCredentialStore {
        self.0.as_ref()
    }
}

impl fmt::Debug for ReconnectCredentialStorage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ReconnectCredentialStorage([INJECTED STORE])")
    }
}

/// Thread-safe in-memory store for deterministic tests.
#[derive(Debug, Default)]
pub struct MemoryReconnectCredentialStore(RwLock<Option<StoredReconnectCredential>>);

impl ReconnectCredentialStore for MemoryReconnectCredentialStore {
    fn load(&self) -> Result<Option<StoredReconnectCredential>, CredentialStoreError> {
        self.0
            .read()
            .map(|stored| stored.clone())
            .map_err(|_poisoned| CredentialStoreError::Unavailable)
    }

    fn store_atomically(
        &self,
        credential: StoredReconnectCredential,
    ) -> Result<(), CredentialStoreError> {
        let mut stored = self
            .0
            .write()
            .map_err(|_poisoned| CredentialStoreError::Unavailable)?;
        *stored = Some(credential);
        Ok(())
    }

    fn delete_if_session(&self, session_id: SessionId) -> Result<bool, CredentialStoreError> {
        let mut stored = self
            .0
            .write()
            .map_err(|_poisoned| CredentialStoreError::Unavailable)?;
        if stored
            .as_ref()
            .is_some_and(|credential| credential.session_id == session_id)
        {
            *stored = None;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn delete_if_expired(&self, unix_seconds: u64) -> Result<bool, CredentialStoreError> {
        let mut stored = self
            .0
            .write()
            .map_err(|_poisoned| CredentialStoreError::Unavailable)?;
        if stored
            .as_ref()
            .is_some_and(|credential| credential.is_expired_at(unix_seconds))
        {
            *stored = None;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

/// Atomic bounded file store beneath an application-selected data directory.
#[derive(Debug, Clone)]
pub struct AtomicFileReconnectCredentialStore {
    path: PathBuf,
}

impl AtomicFileReconnectCredentialStore {
    /// Creates a store for one exact file path.
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Exact application-owned path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    fn delete_file(&self) -> Result<(), CredentialStoreError> {
        match std::fs::remove_file(&self.path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(CredentialStoreError::Io(error)),
        }
    }
}

impl ReconnectCredentialStore for AtomicFileReconnectCredentialStore {
    fn load(&self) -> Result<Option<StoredReconnectCredential>, CredentialStoreError> {
        let mut file = match std::fs::File::open(&self.path) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(CredentialStoreError::Io(error)),
        };
        let mut bytes = Vec::with_capacity(MAX_FILE_BYTES + 1);
        std::io::Read::by_ref(&mut file)
            .take(u64::try_from(MAX_FILE_BYTES + 1).unwrap_or(u64::MAX))
            .read_to_end(&mut bytes)?;
        if bytes.len() > MAX_FILE_BYTES {
            return Err(CredentialStoreError::Malformed);
        }
        serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|_error| CredentialStoreError::Malformed)
    }

    fn store_atomically(
        &self,
        credential: StoredReconnectCredential,
    ) -> Result<(), CredentialStoreError> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let bytes =
            serde_json::to_vec(&credential).map_err(|_error| CredentialStoreError::Malformed)?;
        if bytes.len() > MAX_FILE_BYTES {
            return Err(CredentialStoreError::Malformed);
        }
        let mut options = OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt as _;
            options.mode(0o600);
        }
        AtomicFile::new(&self.path, AllowOverwrite)
            .write_with_options(|file| file.write_all(&bytes), options)
            .map_err(io::Error::from)?;
        Ok(())
    }

    fn delete_if_session(&self, session_id: SessionId) -> Result<bool, CredentialStoreError> {
        let Some(stored) = self.load()? else {
            return Ok(false);
        };
        if stored.session_id != session_id {
            return Ok(false);
        }
        self.delete_file()?;
        Ok(true)
    }

    fn delete_if_expired(&self, unix_seconds: u64) -> Result<bool, CredentialStoreError> {
        let Some(stored) = self.load()? else {
            return Ok(false);
        };
        if !stored.is_expired_at(unix_seconds) {
            return Ok(false);
        }
        self.delete_file()?;
        Ok(true)
    }
}

/// Failure at the reconnect persistence boundary.
#[derive(Debug)]
pub enum CredentialStoreError {
    /// Filesystem access failed.
    Io(io::Error),
    /// Stored bytes or supplied binding were malformed.
    Malformed,
    /// In-memory synchronization was unavailable.
    Unavailable,
}

impl From<io::Error> for CredentialStoreError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl fmt::Display for CredentialStoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Io(_) => "reconnect credential storage I/O failed",
            Self::Malformed => "stored reconnect credential is malformed",
            Self::Unavailable => "reconnect credential storage is unavailable",
        })
    }
}

impl std::error::Error for CredentialStoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Malformed | Self::Unavailable => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn credential() -> StoredReconnectCredential {
        StoredReconnectCredential {
            session_id: SessionId::from_bytes([1; 16]),
            endpoint_binding: ReconnectEndpointBinding::new(
                DirectEndpoint::new("host.local", 7777).expect("valid endpoint"),
                CertificateFingerprint::from_bytes([2; 32]),
                2_000_000_000,
            )
            .expect("valid binding"),
            peer_id: PeerId::from_bytes([3; 16]),
            reconnect_credential: ReconnectCredential::from_bytes([4; 32]),
        }
    }

    #[test]
    fn memory_store_replaces_and_deletes_exact_session() {
        let store = MemoryReconnectCredentialStore::default();
        let value = credential();
        store
            .store_atomically(value.clone())
            .expect("memory store succeeds");
        assert_eq!(store.load().expect("load succeeds"), Some(value.clone()));
        assert!(!store
            .delete_if_session(SessionId::from_bytes([9; 16]))
            .expect("delete succeeds"));
        assert!(store
            .delete_if_session(value.session_id)
            .expect("delete succeeds"));
        assert_eq!(store.load().expect("load succeeds"), None);
    }

    #[test]
    fn atomic_store_round_trips_without_debug_disclosure() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let store =
            AtomicFileReconnectCredentialStore::new(directory.path().join("reconnect.json"));
        let value = credential();
        store
            .store_atomically(value.clone())
            .expect("atomic store succeeds");
        assert_eq!(store.load().expect("load succeeds"), Some(value.clone()));
        assert!(!format!("{value:?}").contains("04040404"));
        assert!(store
            .delete_if_expired(2_000_000_000)
            .expect("expiry deletion succeeds"));
    }
}
