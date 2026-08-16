//! Ordered participant and round sequencing for turn-based games.
//!
//! This crate deliberately knows nothing about Bevy resources, actions,
//! legality, combat, or victory. A game can wrap [`TurnOrder`] in its own
//! resource and translate returned transitions into domain-specific effects.

/// An error returned when constructing or changing a turn order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnOrderError {
    /// A turn order must begin with at least one participant.
    EmptyRoster,
    /// The same participant appears more than once.
    DuplicateParticipant {
        /// Index of the first occurrence.
        first_index: usize,
        /// Index of the duplicate occurrence.
        duplicate_index: usize,
    },
    /// The requested participant is not in the roster.
    UnknownParticipant,
    /// No participant remains to take a turn.
    Finished,
}

impl std::fmt::Display for TurnOrderError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyRoster => formatter.write_str("a turn order requires a participant"),
            Self::DuplicateParticipant {
                first_index,
                duplicate_index,
            } => write!(
                formatter,
                "participant at index {duplicate_index} duplicates index {first_index}"
            ),
            Self::UnknownParticipant => formatter.write_str("participant is not in the turn order"),
            Self::Finished => formatter.write_str("the turn order has no remaining participants"),
        }
    }
}

impl std::error::Error for TurnOrderError {}

/// Facts produced by one explicit advancement.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnTransition<P> {
    /// Participant whose turn ended.
    pub previous: P,
    /// Participant whose turn begins.
    pub current: P,
    /// Current one-based round after advancing.
    pub round: u64,
    /// Round that ended when the roster wrapped, if any.
    pub completed_round: Option<u64>,
}

/// Facts produced by removing a participant.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParticipantRemoval<P> {
    /// Participant that was removed.
    pub removed: P,
    /// Whether the removed participant owned the current turn.
    pub was_current: bool,
    /// Participant that owns the turn after removal, if one remains.
    pub current: Option<P>,
    /// Unchanged one-based round number.
    pub round: u64,
}

/// A unique ordered roster with a current participant and one-based round.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnOrder<P> {
    participants: Vec<P>,
    current_index: Option<usize>,
    round: u64,
}

impl<P: Clone + Eq> TurnOrder<P> {
    /// Constructs an order from a non-empty collection of unique participants.
    pub fn new(participants: impl IntoIterator<Item = P>) -> Result<Self, TurnOrderError> {
        let participants = participants.into_iter().collect::<Vec<_>>();
        if participants.is_empty() {
            return Err(TurnOrderError::EmptyRoster);
        }
        for duplicate_index in 0..participants.len() {
            let Some(duplicate) = participants.get(duplicate_index) else {
                continue;
            };
            if let Some(first_index) = participants
                .iter()
                .take(duplicate_index)
                .position(|participant| participant == duplicate)
            {
                return Err(TurnOrderError::DuplicateParticipant {
                    first_index,
                    duplicate_index,
                });
            }
        }
        Ok(Self {
            participants,
            current_index: Some(0),
            round: 1,
        })
    }

    /// Returns the current participant, or `None` after every participant is removed.
    #[must_use]
    pub fn current(&self) -> Option<&P> {
        self.current_index
            .and_then(|index| self.participants.get(index))
    }

    /// Returns the stable participant order.
    #[must_use]
    pub fn participants(&self) -> &[P] {
        &self.participants
    }

    /// Returns the current one-based round.
    #[must_use]
    pub const fn round(&self) -> u64 {
        self.round
    }

    /// Advances to the next participant, incrementing the round only on wrap.
    pub fn advance(&mut self) -> Result<TurnTransition<P>, TurnOrderError> {
        let Some(previous_index) = self.current_index else {
            return Err(TurnOrderError::Finished);
        };
        let Some(previous) = self.participants.get(previous_index).cloned() else {
            return Err(TurnOrderError::Finished);
        };
        let wraps = previous_index + 1 >= self.participants.len();
        let next_index = if wraps { 0 } else { previous_index + 1 };
        let completed_round = wraps.then_some(self.round);
        if wraps {
            self.round = self.round.saturating_add(1);
        }
        self.current_index = Some(next_index);
        let Some(current) = self.participants.get(next_index).cloned() else {
            return Err(TurnOrderError::Finished);
        };
        Ok(TurnTransition {
            previous,
            current,
            round: self.round,
            completed_round,
        })
    }

    /// Removes one participant without implicitly advancing the round.
    pub fn remove(&mut self, participant: &P) -> Result<ParticipantRemoval<P>, TurnOrderError> {
        let Some(remove_index) = self
            .participants
            .iter()
            .position(|candidate| candidate == participant)
        else {
            return Err(TurnOrderError::UnknownParticipant);
        };
        let Some(current_index) = self.current_index else {
            return Err(TurnOrderError::Finished);
        };
        let was_current = remove_index == current_index;
        let removed = self.participants.remove(remove_index);
        self.current_index = if self.participants.is_empty() {
            None
        } else if remove_index < current_index {
            Some(current_index - 1)
        } else if was_current && current_index >= self.participants.len() {
            Some(0)
        } else {
            Some(current_index)
        };
        Ok(ParticipantRemoval {
            removed,
            was_current,
            current: self.current().cloned(),
            round: self.round,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn construction_rejects_empty_and_duplicate_rosters() {
        assert_eq!(TurnOrder::<u8>::new([]), Err(TurnOrderError::EmptyRoster));
        assert_eq!(
            TurnOrder::new([1, 2, 1]),
            Err(TurnOrderError::DuplicateParticipant {
                first_index: 0,
                duplicate_index: 2,
            })
        );
    }

    #[test]
    fn advancement_wraps_and_increments_round() -> Result<(), TurnOrderError> {
        let mut turns = TurnOrder::new(["player", "opponent"])?;
        assert_eq!(turns.current(), Some(&"player"));
        assert_eq!(turns.advance()?.current, "opponent");
        let wrapped = turns.advance()?;
        assert_eq!(wrapped.current, "player");
        assert_eq!(wrapped.completed_round, Some(1));
        assert_eq!(turns.round(), 2);
        Ok(())
    }

    #[test]
    fn removals_preserve_or_select_the_correct_cursor() -> Result<(), TurnOrderError> {
        let mut turns = TurnOrder::new(['a', 'b', 'c', 'd'])?;
        turns.advance()?;
        turns.advance()?;
        assert_eq!(turns.current(), Some(&'c'));
        assert_eq!(turns.remove(&'a')?.current, Some('c'));
        assert_eq!(turns.remove(&'c')?.current, Some('d'));
        assert_eq!(turns.remove(&'d')?.current, Some('b'));
        let final_removal = turns.remove(&'b')?;
        assert_eq!(final_removal.current, None);
        assert_eq!(turns.round(), 1);
        assert_eq!(turns.advance(), Err(TurnOrderError::Finished));
        Ok(())
    }

    #[test]
    fn removing_current_at_every_cursor_selects_its_successor_without_wrapping_round() {
        for (cursor, expected) in [(0, 'b'), (1, 'c'), (2, 'a')] {
            let mut turns =
                TurnOrder::new(['a', 'b', 'c']).expect("the table fixture is non-empty and unique");
            for _ in 0..cursor {
                turns.advance().expect("the table fixture cannot finish");
            }
            let current = *turns.current().expect("the table fixture has a cursor");
            let removal = turns
                .remove(&current)
                .expect("the current participant is in the roster");
            assert!(removal.was_current);
            assert_eq!(removal.current, Some(expected));
            assert_eq!(removal.round, 1);
            assert_eq!(turns.round(), 1);
        }
    }

    #[test]
    fn equal_operation_sequences_produce_equal_transitions() -> Result<(), TurnOrderError> {
        let mut left = TurnOrder::new([10, 20, 30])?;
        let mut right = left.clone();
        assert_eq!(left.advance()?, right.advance()?);
        assert_eq!(left.remove(&30)?, right.remove(&30)?);
        assert_eq!(left.advance()?, right.advance()?);
        assert_eq!(left, right);
        Ok(())
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_round_trip_is_deterministic() -> Result<(), Box<dyn std::error::Error>> {
        let mut turns = TurnOrder::new([10_u8, 20_u8])?;
        turns.advance()?;
        let encoded = serde_json::to_string(&turns)?;
        let decoded: TurnOrder<u8> = serde_json::from_str(&encoded)?;
        assert_eq!(decoded, turns);
        assert_eq!(serde_json::to_string(&decoded)?, encoded);
        Ok(())
    }
}
