# Mechanics coverage matrix

This is the accepted built-in catalog contract, with independent expected-state
scenarios in [mechanics tests](../rules/tests/mechanics.rs). All use the same
saved Scenario and authoritative Combat reducer as gameplay. No test owns another
mechanics implementation. IDs below map directly to `mechanics_active::<id>`.

## Active Skills

Each row has five named tests: `resulting_state`, `source_rank_boundaries`,
`target_boundaries_and_atomic_rejections`, `equipment_eligibility`, and
`life_state_and_team_eligibility`. Offensive life cases also damage a corpse
created by a real command and assert that status/movement followups are suppressed.
Rank tests vary one boundary at a time, not a Cartesian product. Invalid actions
preserve full Combat state (including RNG/counters); legal commits validate the
result and replay identically. Current rank/use limits never remove a frozen Skill.

Before this change, weapon moves had a broad damage/preview smoke loop; support
and natural moves had selected legacy/unit coverage. These rows add explicit
public-scenario outcomes and all six source/target rank boundaries.

| Skill | Expected state change | Source → target ranks | Grant / prerequisite | Uses | Before → added coverage |
|---|---|---|---|---|---|
| `hurled_scrap` | 2 direct damage | 1,2,3,4,5,6 → 1,2,3,4,5,6 | Personal | Unlimited | Selected unit/legacy → five scenario tests |
| `spare_bandage` | Heal 3, capped at max HP | 1,2,3,4,5,6 → 1,2,3,4,5,6 | Personal | 2 | Selected unit/legacy → five scenario tests |
| `crushing_blow` | 7 direct damage | 1,2,3,4 → 1,2,3,4 | Personal | Unlimited | Selected unit/legacy → five scenario tests |
| `front_strike` | 7 direct damage | 1,2 → 1,2 | gatekeeper_spear | Unlimited | Weapon smoke → five scenario tests |
| `long_reach` | 5 direct damage | 1,2,3,4 → 1,2,3,4 | gatekeeper_spear | Unlimited | Weapon smoke → five scenario tests |
| `driving_blow` | 3 direct damage; Push up to 2 ranks | 1,2 → 1,2 | gatekeeper_spear | Unlimited | Weapon smoke → five scenario tests |
| `field_dressing` | Heal 8, capped at max HP | 1,2,3,4,5,6 → 1,2,3,4,5,6 | Personal | 2 | Selected unit/legacy → five scenario tests |
| `bleeding_cut` | 3 direct damage; Apply Bleed (2 damage × 3 owner starts) | 1,2,3,4 → 1,2,3,4 | knifehand_daggers | Unlimited | Weapon smoke → five scenario tests |
| `deep_strike` | 6 direct damage | 1,2,3,4 → 1,2 | knifehand_daggers | Unlimited | Weapon smoke → five scenario tests |
| `thrown_knife` | 4 direct damage | 3,4,5,6 → 1,2,3,4,5,6 | knifehand_daggers | Unlimited | Weapon smoke → five scenario tests |
| `clean_blade` | Remove Bleeding; no healing | 1,2,3,4,5,6 → 1,2,3,4,5,6 | Personal | Unlimited | Selected unit/legacy → five scenario tests |
| `back_rank_shot` | 7 direct damage | 3,4,5,6 → 5,6 | scout_bow | Unlimited | Weapon smoke → five scenario tests |
| `snap_shot` | 4 direct damage | 1,2,3,4,5,6 → 1,2,3,4,5,6 | scout_bow | Unlimited | Weapon smoke → five scenario tests |
| `hook_shot` | 3 direct damage; Pull past 1 preceding occupant, any size | 3,4,5,6 → 3,4,5,6 | scout_bow | Unlimited | Weapon smoke → five scenario tests |
| `exchange` | Swap with other living/dying ally | 1,2,3,4,5,6 → 1,2,3,4,5,6 | Personal | Unlimited | Selected unit/legacy → five scenario tests |
| `mend` | Heal 8, capped at max HP | 3,4,5,6 → 1,2,3,4,5,6 | Personal | 3 | Selected unit/legacy → five scenario tests |
| `staunch` | Remove Bleeding; no healing | 1,2,3,4,5,6 → 1,2,3,4,5,6 | Personal | Unlimited | Selected unit/legacy → five scenario tests |
| `staff_strike` | 3 direct damage | 1,2,3,4,5,6 → 1,2,3,4 | medic_staff | Unlimited | Weapon smoke → five scenario tests |
| `rally` | Rescue dying hero to 50% max HP, ceil | 1,2,3,4,5,6 → 1,2,3,4,5,6 | Personal | 2 | Selected unit/legacy → five scenario tests |
| `brutal_strike` | 5 direct damage | 1,2 → 1,2 | Personal | Unlimited | Selected unit/legacy → five scenario tests |
| `ragged_cut` | 2 direct damage; Apply Bleed (2 damage × 3 owner starts) | 1,2,3,4,5,6 → 1,2,3,4 | Personal | Unlimited | Selected unit/legacy → five scenario tests |
| `hollow_bolt` | 4 direct damage | 1,2,3,4,5,6 → 1,2,3,4,5,6 | hollow_bow | Unlimited | Weapon smoke → five scenario tests |
| `dagger_stab` | 5 direct damage | 1,2,3,4 → 1,2 | dagger | Unlimited | Weapon smoke → five scenario tests |
| `dagger_throw` | 4 direct damage | 3,4,5,6 → 1,2,3,4,5,6 | dagger | Unlimited | Weapon smoke → five scenario tests |
| `greatsword_cleave` | 6 direct damage; Distinct occupants of ranks 1–2, once each | 1,2 → 1,2 | greatsword | Unlimited | Weapon smoke → five scenario tests |
| `greatsword_thrust` | 8 direct damage | 1,2,3,4 → 1,2 | greatsword | Unlimited | Weapon smoke → five scenario tests |
| `axe_chop` | 10 direct damage | 1,2 → 1,2 | two_handed_axe | Unlimited | Weapon smoke → five scenario tests |
| `spear_thrust` | 6 direct damage | 1,2,3,4 → 1,2,3,4 | spear | Unlimited | Weapon smoke → five scenario tests |
| `spear_shove` | 2 direct damage; Push up to 1 ranks | 1,2,3,4 → 1,2 | spear | Unlimited | Weapon smoke → five scenario tests |
| `bow_aimed_shot` | 7 direct damage | 5,6 → 1,2,3,4,5,6 | bow | Unlimited | Weapon smoke → five scenario tests |
| `bow_quick_shot` | 4 direct damage | 1,2,3,4,5,6 → 1,2,3,4,5,6 | bow | Unlimited | Weapon smoke → five scenario tests |
| `staff_blow` | 3 direct damage | 1,2,3,4,5,6 → 1,2,3,4 | staff | Unlimited | Weapon smoke → five scenario tests |
| `staff_push` | 2 direct damage; Push up to 1 ranks | 1,2,3,4 → 1,2,3,4 | staff | Unlimited | Weapon smoke → five scenario tests |
| `assassin_feint` | 4 direct damage | 1,2,3,4,5,6 → 1,2,3,4 | Personal; dagger kind required | Unlimited | Selected unit/legacy → five scenario tests |

Enemy damage Skills accept damageable opponents and single-target corpse attacks
on either side. Heal/cleanse target standing allies; Field Dressing/Clean Blade
are self-only. Rally targets dying heroes; Exchange accepts another standing or
dying ally. Damage caps at remaining health; dying hits add one failure. A lethal
hit suppresses subsequent movement/status effects. These eligibility rules are
separate from movement resistance, which is not implemented.

## Passive Abilities

| Ability | Prerequisites and expected contribution | Existing coverage | Added scenarios |
|---|---|---|---|
| Bleeding Dagger Technique (`assassin_bleeding_dagger`) | Dagger kind and eligible Dagger Stab; append one Bleed (2 × 3 starts, or 2 starts on Resilient) | Resolver dedup and one runtime combination | Individual/combined upgrade execution, missing weapon/Skill, duplicate personal/equipment sources, inactive passives excluded from actions |
| Duelist Dagger Training (`duelist_dagger_power`) | Dagger kind and eligible Dagger Stab; +2 direct damage (5 → 7) | Resolver dedup and combined preview | Individual/combined committed HP, kind-vs-starter mismatch, duplicate grants; authored rank-extension adapter |
| Resilient (`resilient`) | No built-in equipment prerequisite; reduce new/refreshed finite debuffs by one, minimum one | Build-duration and selected runtime/starting checks | Bleed/Weakened application and expiry; refresh source/identity; buffs unchanged; explicit saved clocks; snapshot restore; duplicate grants; distinct reducers; authored exact-item/kind restriction |

These cases live in [passive scenarios](../rules/tests/mechanics_passives/mod.rs).
Knifehand Daggers are dagger-kind but do not grant Dagger Stab, so both dagger
upgrades remain inactive. No built-in weapon currently grants an Ability; authored
equipment grants are tested without inventing new stock content.

## Equipment and Moveset contracts

The eleven items and their exact ordered grants are asserted in
`mechanics_build::all_eleven_items_grant_exact_ordered_moves_without_passive_actions`.
The Skill table lists every item grant. Additional build scenarios prove:

- Personal kind/exact-item requirements remain selected while inactive, then
  reactivate with the required equipment; an exact item is stricter than its kind.
- Equipment-only Skills cannot be copied into personal selections.
- Multiple grant paths preserve provenance but execute a Skill/Ability once.
- Empty Movesets retain universal actions; passives cannot become Skill actions.
- Existing encounters keep their frozen catalog after later catalog edits.

Retained `build_contract_tests` and `catalog_tests` cover malformed requirements,
combined upgrade validation, source restrictions, beyond-eight Movesets, tampered
resolved DTOs and all ten starter presets. These are not duplicated by the new
scenario suite.

## Interactions and additional state assertions

| Contract | Scenarios / expected outcomes | Prior evidence and discrepancy |
|---|---|---|
| Hook Shot | Calibration: target 5 → 3, blocker 3–4 → 4–5, HP 100 → 97; all 15 legal adjacent custom-width pairs; sole six-rank target remains at edge; lethal hit leaves corpse unmoved | Old rank-budget behavior reproduced; changed by explicit user decision |
| Pushes | Configured 1/2-rank neighbors despite conflicting appearance; partial and blocked 2-rank pushes; trailing empty capacity and edges | Found and fixed preset-size lookup defect; retained preview push cases |
| Exchange/Reposition | Whole footprints swap; identity/build preserved; downed targets accepted; corpses rejected | Existing initiative/status identity tests retained |
| Limited resources | All limited heals exhaust; Rally and legacy alias share counter; rejected exhausted cast is inert; Dagger Throw remains unlimited | Adds public full-use lifecycle over selected unit checks |
| Healing/rescue | Caps, full-health casts, no incidental cleanse, 25% universal vs 50% Rally rounding; nonzero failures reset | Existing life/preview cases retained |
| Damage modifiers | Brace −2 and Weakened −2, clamp at zero; Bleed ignores Brace | Existing pure modifier/boundary cases retained |
| Status life | Bleed owner-start ticks and refresh; Weakened owner-end expiry; Haste two round ends without changing current order; Brace next owner-start expiry; cleanse only Bleeding | Haste/Weakened are implemented fixture statuses, not built-in Skill grants |
| Corpses and terminal effects | New per-offensive-Skill corpse/dying cases; support rejection; retained cleave capture/compaction and lethal followup tests | Existing lifecycle tests cover corpse Bleed/expiry, creation-round exclusion, death saves and terminal evaluation |
| Adapter | `session::tests::mechanics::hook_shot_request_projects_the_accepted_whole_occupant_pull`; indexed request, forecast, HP, ranks, validated session roundtrip | Checks command/projection boundary, not desktop input |
| Description | `presentation::tests::pull_description_counts_occupants_while_push_counts_ranks`; actual displacement label no longer compares ranks to occupant steps | Updated for changed pull units |

## Remaining boundaries

- Scenario combat formations are compact. Sparse preparation drafts are covered
  by the existing session spatial suite; gaps must be repaired before deployment.
  Smaller combat rosters retain empty rear capacity. No sparse combat engine added.
- No movement/status resistance, immunity stat, mana/ammunition consumption,
  arbitrary stacking, reactions, inventory, progression or RL training API exists.
  Life-state suppression and nonstacking refresh are tested; absent mechanics are
  not treated as implemented or silently assigned hypothetical rules.
- Existing deterministic lifecycle tests supply death/round/corpse coverage;
  the new suite supplies catalog-wide public-entry acceptance. Together they
  provide engineering evidence, not balance, usability or exhaustive state-space proof.
- No screenshots, resolution sweeps, manual walkthroughs, network/process E2E or
  training runs are required for this backend scope.
