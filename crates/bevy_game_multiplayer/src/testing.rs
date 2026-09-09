//! Deterministic in-memory byte links for multi-App tests.

use std::sync::{
    mpsc::{self, Receiver, SyncSender, TryRecvError, TrySendError},
    Arc, Mutex,
};

use bevy::prelude::Resource;

/// One endpoint of a bounded deterministic in-memory session link.
#[derive(Resource, Clone)]
pub struct InMemoryEndpoint {
    sender: SyncSender<Vec<u8>>,
    receiver: Arc<Mutex<Receiver<Vec<u8>>>>,
    max_message_bytes: usize,
}

impl std::fmt::Debug for InMemoryEndpoint {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("InMemoryEndpoint")
            .field("max_message_bytes", &self.max_message_bytes)
            .finish_non_exhaustive()
    }
}

impl InMemoryEndpoint {
    /// Tries to send one bounded payload without blocking a gameplay/test schedule.
    pub fn send(&self, payload: Vec<u8>) -> Result<(), LinkError> {
        if payload.len() > self.max_message_bytes {
            return Err(LinkError::MessageTooLarge);
        }
        self.sender.try_send(payload).map_err(|error| match error {
            TrySendError::Full(_) => LinkError::Backpressure,
            TrySendError::Disconnected(_) => LinkError::Disconnected,
        })
    }

    /// Receives one queued payload without blocking.
    pub fn try_receive(&self) -> Result<Option<Vec<u8>>, LinkError> {
        let receiver = self
            .receiver
            .lock()
            .map_err(|_poisoned| LinkError::Unavailable)?;
        match receiver.try_recv() {
            Ok(payload) => Ok(Some(payload)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err(LinkError::Disconnected),
        }
    }
}

/// Factory for a host/client pair with bounded queues and messages.
#[derive(Debug, Clone, Copy)]
pub struct InMemorySessionLink;

impl InMemorySessionLink {
    /// Creates paired endpoints suitable for insertion into independent Bevy Apps.
    #[must_use]
    pub fn pair(capacity: usize, max_message_bytes: usize) -> (InMemoryEndpoint, InMemoryEndpoint) {
        let (host_to_client_sender, host_to_client_receiver) = mpsc::sync_channel(capacity);
        let (client_to_host_sender, client_to_host_receiver) = mpsc::sync_channel(capacity);
        (
            InMemoryEndpoint {
                sender: host_to_client_sender,
                receiver: Arc::new(Mutex::new(client_to_host_receiver)),
                max_message_bytes,
            },
            InMemoryEndpoint {
                sender: client_to_host_sender,
                receiver: Arc::new(Mutex::new(host_to_client_receiver)),
                max_message_bytes,
            },
        )
    }
}

/// Why an in-memory link operation failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkError {
    /// Payload exceeded the configured cap.
    MessageTooLarge,
    /// The bounded queue is full; the caller must retry or apply its own policy.
    Backpressure,
    /// The paired endpoint was dropped.
    Disconnected,
    /// Internal synchronization was poisoned.
    Unavailable,
}

impl std::fmt::Display for LinkError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::MessageTooLarge => "in-memory message exceeds the configured limit",
            Self::Backpressure => "in-memory message queue is full",
            Self::Disconnected => "in-memory peer is disconnected",
            Self::Unavailable => "in-memory link is unavailable",
        })
    }
}

impl std::error::Error for LinkError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pair_delivers_in_both_directions_and_enforces_bounds() {
        let (host, client) = InMemorySessionLink::pair(4, 8);
        host.send(vec![1, 2]).expect("host send succeeds");
        assert_eq!(
            client.try_receive().expect("receive succeeds"),
            Some(vec![1, 2])
        );
        client.send(vec![3]).expect("client send succeeds");
        assert_eq!(host.try_receive().expect("receive succeeds"), Some(vec![3]));
        assert_eq!(host.send(vec![0; 9]), Err(LinkError::MessageTooLarge));
    }

    #[test]
    fn full_queue_returns_backpressure_and_recovers_after_receive() {
        let (host, client) = InMemorySessionLink::pair(1, 8);
        assert_eq!(host.send(vec![1]), Ok(()));
        assert_eq!(host.send(vec![2]), Err(LinkError::Backpressure));
        assert_eq!(client.try_receive(), Ok(Some(vec![1])));
        assert_eq!(host.send(vec![2]), Ok(()));
        assert_eq!(client.try_receive(), Ok(Some(vec![2])));
        assert_eq!(client.try_receive(), Ok(None));
        drop(client);
        assert_eq!(host.send(vec![3]), Err(LinkError::Disconnected));
    }

    #[test]
    fn zero_capacity_queue_never_blocks_a_single_threaded_app_pump() {
        let (host, _client) = InMemorySessionLink::pair(0, 8);
        assert_eq!(host.send(vec![1]), Err(LinkError::Backpressure));
    }
}
