// Placeholder for future RNG helpers. The RNG itself (a `ChaCha8Rng`) lives
// directly inside `BattleState` for now — when the resolver actually needs
// randomness (crit rolls, accuracy checks), wrap convenience helpers here so
// the rest of the backend doesn't depend on `rand`/`rand_chacha` traits
// directly.
