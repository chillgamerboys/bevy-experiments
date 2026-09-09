//! Typed status definitions and runtime instances; no callback or script engine.

use serde::{Deserialize, Serialize};

use crate::{ActorId, DamageKind};

/// A status catalog key; fixture modifiers are not granted by initial hero skills.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum StatusKind {
    /// Damage at the next three bearer turn starts.
    Bleed,
    /// Direct incoming damage reduction until the next bearer turn start.
    Brace,
    /// Synthetic/variant additive speed buff.
    Haste,
    /// Synthetic/variant additive outgoing damage debuff.
    Weakened,
}

/// Presentation and cleansing classification, not an implicit behavior selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum StatusTag {
    /// Beneficial modifier.
    Buff,
    /// Harmful modifier or effect.
    Debuff,
    /// Periodic bleeding damage.
    Bleeding,
}

/// Explicit combat boundary at which a status may trigger or expire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Boundary {
    /// Immediately before the bearer may choose its action.
    OwnerTurnStart,
    /// After the bearer's committed action.
    OwnerTurnEnd,
    /// After every initiative entry has been consumed.
    RoundEnd,
}

/// An instance's lifetime; triggers precede matching lifetime decrement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct DurationClock {
    /// Boundary counted by the lifetime.
    pub boundary: Boundary,
    /// Number of future matching boundaries remaining on application.
    pub ticks: u8,
}

/// Derived statistic; base values are never edited by status application/removal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Stat {
    /// Effective speed, clamped to 0..100, used at the next round roll.
    Speed,
    /// Additive outgoing direct damage, final damage clamped at zero.
    OutgoingDamage,
    /// Additive incoming direct damage, final damage clamped at zero.
    IncomingDirectDamage,
}

/// Additive contribution per instance potency (no floating-point composition).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Modifier {
    /// Derived statistic affected.
    pub stat: Stat,
    /// Signed multiplier for the instance's positive potency.
    pub per_potency: i16,
}

/// One-instance-per-bearer-and-kind reapplication policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Reapplication {
    /// Refresh duration and source; preserve the existing potency, do not stack.
    Refresh,
}

/// Effects shared by skills and status triggers. Effects are resolved in order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Effect {
    /// Direct fixed damage before source/target modifiers.
    Damage(u16),
    /// Damage equal to status potency, ignoring direct-damage modifiers.
    StatusDamage(DamageKind),
    /// Heal a standing target, capped by maximum HP.
    Heal(u16),
    /// Apply a catalog status with its validated potency and duration.
    ApplyStatus(StatusKind),
    /// Remove all matching tagged statuses.
    Cleanse(StatusTag),
    /// Move target toward the rear (positive) or front (negative).
    Move(i8),
    /// Swap the source and ally positions, including downed allies.
    SwapWithSource,
    /// Restore this percentage of maximum HP to a downed hero, rounding up.
    Rescue(u8),
}

/// Immutable typed status content.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct StatusDefinition {
    /// Stable catalog identifier.
    pub kind: StatusKind,
    /// Display name.
    pub name: &'static str,
    /// Plain-language rule explanation, also suitable for keyboard inspection.
    pub description: &'static str,
    /// Semantic classifications.
    pub tags: &'static [StatusTag],
    /// Explicit ordering priority; ties use stable instance ID.
    pub priority: u8,
    /// Positive initial potency.
    pub potency: u16,
    /// Lifetime clock and initial duration.
    pub duration: DurationClock,
    /// Trigger timing, absent for a pure modifier with expiry only.
    pub trigger: Option<Boundary>,
    /// Ordered trigger behavior.
    pub effects: &'static [Effect],
    /// Additive derived stat contributions.
    pub modifiers: &'static [Modifier],
    /// Same-kind aggregation contract.
    pub reapplication: Reapplication,
    /// Whether downing removes this specific status.
    pub remove_on_downed: bool,
}

/// A live public effect instance. Its source may have died since application.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatusInstance {
    /// Monotonic encounter-local identity, independent of rendering entities.
    pub id: u64,
    /// Catalog definition.
    pub kind: StatusKind,
    /// Character carrying the status, not a formation position.
    pub bearer: ActorId,
    /// Most recent source; death does not implicitly remove its status.
    pub source: ActorId,
    /// Positive magnitude; current definitions deliberately do not stack.
    pub potency: u16,
    /// Remaining future matching boundaries/ticks.
    pub remaining: u8,
    /// First boundary sequence after application; prevents same-boundary recursion.
    pub eligible_boundary: u64,
}

/// Why an instance stopped applying.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RemovalReason {
    /// Duration reached zero.
    Expired,
    /// A cleanse skill explicitly removed the matching classification.
    Cleansed,
    /// The bearer's downing policy removed it.
    Downed,
    /// Enemy death removed its active effects.
    Defeated,
}

/// Resolve immutable rules for a status; classifiers never select behavior.
#[must_use]
pub const fn status_definition(kind: StatusKind) -> StatusDefinition {
    match kind {
        StatusKind::Bleed => StatusDefinition {
            kind,
            name: "Bleed",
            description:
                "Take 2 damage at the start of your next 3 turns. Refreshes; does not stack.",
            tags: &[StatusTag::Debuff, StatusTag::Bleeding],
            priority: 20,
            potency: 2,
            duration: DurationClock {
                boundary: Boundary::OwnerTurnStart,
                ticks: 3,
            },
            trigger: Some(Boundary::OwnerTurnStart),
            effects: &[Effect::StatusDamage(DamageKind::Bleed)],
            modifiers: &[],
            reapplication: Reapplication::Refresh,
            remove_on_downed: true,
        },
        StatusKind::Brace => StatusDefinition {
            kind,
            name: "Brace",
            description:
                "Take 2 less direct damage until your next turn starts. Does not reduce bleed.",
            tags: &[StatusTag::Buff],
            priority: 10,
            potency: 2,
            duration: DurationClock {
                boundary: Boundary::OwnerTurnStart,
                ticks: 1,
            },
            trigger: None,
            effects: &[],
            modifiers: &[Modifier {
                stat: Stat::IncomingDirectDamage,
                per_potency: -1,
            }],
            reapplication: Reapplication::Refresh,
            remove_on_downed: true,
        },
        StatusKind::Haste => StatusDefinition {
            kind,
            name: "Haste",
            description:
                "+3 Speed for 2 round ends. The already-rolled initiative order is unchanged.",
            tags: &[StatusTag::Buff],
            priority: 10,
            potency: 3,
            duration: DurationClock {
                boundary: Boundary::RoundEnd,
                ticks: 2,
            },
            trigger: None,
            effects: &[],
            modifiers: &[Modifier {
                stat: Stat::Speed,
                per_potency: 1,
            }],
            reapplication: Reapplication::Refresh,
            remove_on_downed: false,
        },
        StatusKind::Weakened => StatusDefinition {
            kind,
            name: "Weakened",
            description: "Deal 2 less direct damage for your next 2 turn ends.",
            tags: &[StatusTag::Debuff],
            priority: 10,
            potency: 2,
            duration: DurationClock {
                boundary: Boundary::OwnerTurnEnd,
                ticks: 2,
            },
            trigger: None,
            effects: &[],
            modifiers: &[Modifier {
                stat: Stat::OutgoingDamage,
                per_potency: -1,
            }],
            reapplication: Reapplication::Refresh,
            remove_on_downed: false,
        },
    }
}
