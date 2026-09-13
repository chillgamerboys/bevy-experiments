//! Authoritative construction positions; combat continues to use compact scenarios.

use super::*;
use labyrinth_rules::Team;

/// One actor's stable leading rank in the construction board.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormationPlacement {
    /// Stable actor identity, independent of the selected preset.
    pub actor: ActorId,
    /// One-based front-to-back leading rank, from one through six.
    pub rank: u8,
}

/// Shared lobby positions and host-assigned places, including empty places.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LobbyFormation {
    /// Heroes in ascending leading-rank order.
    pub heroes: Vec<FormationPlacement>,
    /// Enemies in ascending leading-rank order.
    pub enemies: Vec<FormationPlacement>,
    /// Participant owner of each hero rank; zero is the host, including empty ranks.
    pub hero_owners: [u8; PARTY_SIZE],
}

impl LobbyFormation {
    /// Construct a compact board for stock and portable scenarios.
    pub fn compact(scenario: &Scenario) -> Self {
        let placements = |roster: &[ScenarioActor]| {
            let mut rank = 1;
            roster
                .iter()
                .map(|actor| {
                    let placement = FormationPlacement {
                        actor: actor.id,
                        rank,
                    };
                    rank += actor.actor.footprint;
                    placement
                })
                .collect()
        };
        Self {
            heroes: placements(&scenario.heroes),
            enemies: placements(&scenario.enemies),
            hero_owners: [0; PARTY_SIZE],
        }
    }

    /// Select a side without interpreting actor appearance as team membership.
    pub fn placements(&self, team: Team) -> &[FormationPlacement] {
        match team {
            Team::Heroes => &self.heroes,
            Team::Enemies => &self.enemies,
        }
    }

    pub(super) fn placements_mut(&mut self, team: Team) -> &mut Vec<FormationPlacement> {
        match team {
            Team::Heroes => &mut self.heroes,
            Team::Enemies => &mut self.enemies,
        }
    }

    /// Current leading rank for a stable identity, including sparse constructions.
    pub fn rank(&self, actor: ActorId) -> Option<u8> {
        self.heroes
            .iter()
            .chain(&self.enemies)
            .find(|p| p.actor == actor)
            .map(|p| p.rank)
    }

    /// Resolve every covered rank of a multi-rank character to the same identity.
    pub fn occupant(&self, scenario: &Scenario, team: Team, rank: u8) -> Option<ActorId> {
        self.placements(team).iter().find_map(|placement| {
            let actor = roster(scenario, team)
                .iter()
                .find(|a| a.id == placement.actor)?;
            (rank >= placement.rank
                && u16::from(rank) < u16::from(placement.rank) + u16::from(actor.actor.footprint))
            .then_some(actor.id)
        })
    }

    /// Preview the same bounds, collisions and place permissions used by authority.
    /// `replacing` excludes only that actor's current footprint. `player` is absent
    /// for the host; a guest may choose a type only inside their assigned places.
    pub fn placement_error(
        &self,
        scenario: &Scenario,
        team: Team,
        rank: u8,
        footprint: u8,
        replacing: Option<ActorId>,
        player: Option<u8>,
    ) -> Option<String> {
        let end = u16::from(rank) + u16::from(footprint);
        if rank == 0 || footprint == 0 || end > PARTY_SIZE as u16 + 1 {
            return Some("That character does not fit within ranks 1–6.".into());
        }
        if player.is_some() && team == Team::Enemies {
            return Some("Only the host constructs the enemy formation.".into());
        }
        if team == Team::Heroes {
            let owner = self.hero_owners[usize::from(rank - 1)];
            if player.is_some_and(|slot| slot != owner) {
                return Some("Choose a character in your assigned places.".into());
            }
            if (rank..end as u8).any(|r| self.hero_owners[usize::from(r - 1)] != owner) {
                return Some("This footprint crosses places assigned to different players. The host must assign the whole span to one player.".into());
            }
        }
        for r in rank..end as u8 {
            if let Some(actor) = self
                .occupant(scenario, team, r)
                .filter(|id| Some(*id) != replacing)
            {
                let name = roster(scenario, team)
                    .iter()
                    .find(|a| a.id == actor)
                    .map_or("another character", |a| a.actor.name.as_str());
                return Some(format!(
                    "Rank {r} is occupied by {name}. Move or remove that character first."
                ));
            }
        }
        None
    }

    /// The first precise deployment problem; trailing unused capacity is legal.
    pub fn deployment_error(&self, scenario: &Scenario) -> Option<String> {
        for (team, label) in [(Team::Heroes, "Party"), (Team::Enemies, "Enemies")] {
            let placements = self.placements(team);
            if placements.is_empty() {
                return Some(format!(
                    "{label} needs at least one character before deployment."
                ));
            }
            let last = placements
                .iter()
                .filter_map(|p| {
                    roster(scenario, team)
                        .iter()
                        .find(|a| a.id == p.actor)
                        .map(|a| p.rank + a.actor.footprint - 1)
                })
                .max()
                .unwrap_or(0);
            let gaps = (1..=last)
                .filter(|rank| self.occupant(scenario, team, *rank).is_none())
                .map(|rank| rank.to_string())
                .collect::<Vec<_>>();
            if !gaps.is_empty() {
                return Some(format!(
                    "{label} has empty rank{} {} before occupied places. Fill or move characters into the gap before deployment.",
                    if gaps.len() == 1 { "" } else { "s" },
                    gaps.join(", ")
                ));
            }
        }
        if scenario
            .heroes
            .iter()
            .all(|actor| actor.starting_hp == Some(0))
        {
            return Some("At least one hero must start standing before deployment.".into());
        }
        None
    }

    pub(super) fn validate(&self, scenario: &Scenario) -> Result<(), &'static str> {
        for team in [Team::Heroes, Team::Enemies] {
            let placements = self.placements(team);
            let actors = roster(scenario, team);
            if placements.len() != actors.len()
                || placements.len() > PARTY_SIZE
                || placements.windows(2).any(|p| p[0].rank >= p[1].rank)
            {
                return Err("Formation placements must match the ordered roster.");
            }
            let mut occupied = BTreeSet::new();
            for (placement, actor) in placements.iter().zip(actors) {
                if placement.actor != actor.id
                    || placement.rank == 0
                    || actor.actor.footprint == 0
                    || u16::from(placement.rank) + u16::from(actor.actor.footprint)
                        > PARTY_SIZE as u16 + 1
                {
                    return Err("Invalid formation actor or footprint.");
                }
                for rank in placement.rank..placement.rank + actor.actor.footprint {
                    if !occupied.insert(rank) {
                        return Err("Formation footprints overlap.");
                    }
                    if team == Team::Heroes
                        && self.hero_owners[usize::from(rank - 1)]
                            != self.hero_owners[usize::from(placement.rank - 1)]
                    {
                        return Err("A character cannot span different controllers' places.");
                    }
                }
            }
        }
        Ok(())
    }

    pub(super) fn assign_actor(&mut self, scenario: &Scenario, actor: ActorId, owner: u8) {
        if let Some((rank, footprint)) = self.rank(actor).zip(
            scenario
                .heroes
                .iter()
                .find(|a| a.id == actor)
                .map(|a| a.actor.footprint),
        ) {
            for r in rank..rank + footprint {
                self.hero_owners[usize::from(r - 1)] = owner;
            }
        }
    }
}

pub(super) fn roster(scenario: &Scenario, team: Team) -> &[ScenarioActor] {
    match team {
        Team::Heroes => &scenario.heroes,
        Team::Enemies => &scenario.enemies,
    }
}

pub(super) fn roster_mut(scenario: &mut Scenario, team: Team) -> &mut Vec<ScenarioActor> {
    match team {
        Team::Heroes => &mut scenario.heroes,
        Team::Enemies => &mut scenario.enemies,
    }
}

impl PartyAuthority {
    pub(super) fn check_construction_revision(
        &self,
        setup: u64,
        assignments: u64,
    ) -> Result<(), String> {
        if !self.in_lobby() {
            return Err("Formation construction is available in the lobby.".into());
        }
        if setup != self.setup_revision {
            return Err("Setup changed. Reload your draft before applying.".into());
        }
        if assignments != self.assignment_revision {
            return Err("Character assignments changed. Refresh your draft.".into());
        }
        Ok(())
    }

    pub(super) fn actor_team(&self, actor: ActorId) -> Option<Team> {
        [Team::Heroes, Team::Enemies]
            .into_iter()
            .find(|team| roster(&self.scenario, *team).iter().any(|a| a.id == actor))
    }

    pub(super) fn configure_draft(
        &mut self,
        mut scenario: Scenario,
        mut formation: LobbyFormation,
        expected_revision: u64,
    ) -> Result<(), String> {
        if !self.in_lobby() {
            return Err("Battle builds and formation are configured in the lobby.".into());
        }
        if expected_revision != self.setup_revision {
            return Err("Setup changed. Reload your draft before applying.".into());
        }
        for team in [Team::Heroes, Team::Enemies] {
            formation.placements_mut(team).sort_by_key(|p| p.rank);
            roster_mut(&mut scenario, team)
                .sort_by_key(|a| formation.rank(a.id).unwrap_or(u8::MAX));
        }
        scenario
            .validate_preparation(&self.catalog)
            .map_err(|e| e.to_string())?;
        formation.validate(&scenario).map_err(str::to_owned)?;
        if formation
            .hero_owners
            .iter()
            .any(|owner| !self.players.iter().any(|p| p.slot == *owner && p.occupied))
        {
            return Err("Every reserved place requires an admitted controller.".into());
        }
        if scenario == self.scenario && formation == self.formation {
            return Ok(());
        }
        let mut company = Self::company_for(&scenario, &self.catalog, &self.company)?;
        for member in &mut company {
            let rank = formation
                .rank(member.actor)
                .ok_or("Unknown character place.")?;
            member.owner = formation.hero_owners[usize::from(rank - 1)];
        }
        self.scenario = scenario;
        self.formation = formation;
        self.company = company;
        self.setup_revision += 1;
        self.assignment_revision += 1;
        for player in &mut self.players {
            player.ready = false;
        }
        Ok(())
    }

    pub(super) fn place_scenario_actor(
        &mut self,
        slot: u8,
        team: Team,
        rank: u8,
        preset: &labyrinth_rules::catalog::ContentId,
        expected_revision: u64,
    ) -> Result<(), String> {
        let build = ActorBuild::from_preset(&self.catalog, preset).map_err(|e| e.to_string())?;
        let replacing = self.formation.occupant(&self.scenario, team, rank);
        let rank = replacing
            .and_then(|id| self.formation.rank(id))
            .unwrap_or(rank);
        if let Some(error) = self.formation.placement_error(
            &self.scenario,
            team,
            rank,
            build.footprint,
            replacing,
            (slot != 0).then_some(slot),
        ) {
            return Err(error);
        }
        let mut scenario = self.scenario.clone();
        let mut formation = self.formation.clone();
        let id = if let Some(id) = replacing {
            id
        } else {
            let used = scenario
                .heroes
                .iter()
                .chain(&scenario.enemies)
                .map(|a| a.id.0)
                .collect::<BTreeSet<_>>();
            ActorId(
                (1..=u16::MAX)
                    .find(|id| !used.contains(id))
                    .ok_or("No free character identity.")?,
            )
        };
        let actors = roster_mut(&mut scenario, team);
        if let Some(current) = actors.iter_mut().find(|a| a.id == id) {
            current.actor = build;
            current.starting_hp = None;
            current.starting_statuses.clear();
        } else {
            actors.push(ScenarioActor {
                id,
                actor: build,
                controller: if team == Team::Heroes {
                    ControllerPolicy::Manual
                } else {
                    ControllerPolicy::Ai
                },
                starting_hp: None,
                starting_statuses: Vec::new(),
            });
            formation
                .placements_mut(team)
                .push(FormationPlacement { actor: id, rank });
        }
        self.configure_draft(scenario, formation, expected_revision)
    }

    pub(super) fn move_scenario_actor(
        &mut self,
        actor: ActorId,
        rank: u8,
        expected_revision: u64,
    ) -> Result<(), String> {
        let team = self.actor_team(actor).ok_or("Unknown character.")?;
        let configured = roster(&self.scenario, team)
            .iter()
            .find(|a| a.id == actor)
            .ok_or("Unknown character.")?;
        if let Some(error) = self.formation.placement_error(
            &self.scenario,
            team,
            rank,
            configured.actor.footprint,
            Some(actor),
            None,
        ) {
            return Err(error);
        }
        let mut formation = self.formation.clone();
        formation
            .placements_mut(team)
            .iter_mut()
            .find(|p| p.actor == actor)
            .ok_or("Unknown formation character.")?
            .rank = rank;
        self.configure_draft(self.scenario.clone(), formation, expected_revision)
    }
}
