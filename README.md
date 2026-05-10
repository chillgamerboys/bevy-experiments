# Turn-Based Tactics Game - Learning Bevy ECS

A simple turn-based tactical game built with [Bevy](https://bevyengine.org/) to learn the Entity Component System (ECS) architecture.

## 🎯 Project Goal

This project is designed as a **learning journey** through Bevy's core concepts. Each phase builds on the previous one, introducing new ECS patterns and game development techniques.

## 🚀 Quick Start

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and run
git clone <your-repo-url>
cd bevy-game
cargo run
```

**Controls:**
- **WASD / Arrow Keys** - Pan camera
- **Mouse Click** - Interact with tiles/units (phase dependent)
- **Enter** - Start game (from main menu in Phase 2+)

## 📚 Learning Phases

### ✅ [Phase 1: Grid Rendering and Camera Setup](docs/phase1-grid-rendering.md)

**Status:** Complete | **Goal:** Understand Bevy's rendering and ECS basics

**What you'll learn:**
- Entity Component System fundamentals
- Spawning entities with components
- Resources for global state
- Systems for game logic
- Camera setup and controls
- Coordinate conversion (screen ↔ world ↔ grid)

**Deliverable:** 10×10 checkerboard grid with camera pan and click detection

---

### ✅ [Phase 2: State Management and Input](docs/phase2-state-input.md)

**Status:** Complete | **Goal:** Learn Bevy's state system and input handling

**What you'll learn:**
- State machines with `States` trait
- State transitions with `NextState<T>`
- Conditional system execution
- Input handling patterns
- Main menu creation

**Deliverable:** Main menu → gameplay flow with proper state management

---

### 📋 [Phase 3: Units and Selection](docs/phase3-units-selection.md)

**Status:** Planned | **Goal:** Learn queries, markers, and component-based logic

**What you'll learn:**
- Component composition patterns
- Marker components
- Query filters (`With<T>`, `Without<T>`)
- Dynamic component insertion/removal
- Visual feedback systems

**Deliverable:** Selectable player and enemy units with visual indicators

---

### 📋 [Phase 4: Turn-Based Movement](docs/phase4-movement-turns.md)

**Status:** Planned | **Goal:** Learn state-dependent logic and grid-based movement

**What you'll learn:**
- Sub-states and state composition
- Change detection
- Adjacent tile movement
- Turn-based game loop
- UI systems

**Deliverable:** Working turn-based movement with player/enemy turns

---

### 📋 [Phase 5: Simple AI](docs/phase5-ai.md)

**Status:** Planned | **Goal:** Learn system scheduling and basic game AI

**What you'll learn:**
- AI decision-making in ECS
- System ordering
- Time-based logic
- Randomness in systems
- Complex queries

**Deliverable:** AI-controlled enemies that move toward player units

---

### 📋 [Phase 6: Basic Combat](docs/phase6-combat.md)

**Status:** Planned (Optional) | **Goal:** Learn event systems and inter-entity interactions

**What you'll learn:**
- Event systems
- Entity despawning
- Combat calculations
- UI rendering (health bars)
- Win/lose conditions

**Deliverable:** Combat system with health, damage, and game over states

## 🏗️ Project Structure

```
bevy-game/
├── src/
│   ├── main.rs          # App setup and system registration
│   ├── components.rs    # Component definitions (data)
│   ├── resources.rs     # Resource definitions (global state)
│   ├── systems.rs       # System definitions (logic)
│   └── constants.rs     # Game constants and configuration
├── docs/
│   ├── phase1-grid-rendering.md
│   ├── phase2-state-input.md
│   ├── phase3-units-selection.md
│   ├── phase4-movement-turns.md
│   ├── phase5-ai.md
│   └── phase6-combat.md
├── Cargo.toml          # Dependencies and build config
└── README.md
```

## 🎮 Current Features

- [x] 10×10 grid rendering with checkerboard pattern
- [x] 2D camera with WASD/arrow key panning
- [x] Mouse click detection with grid coordinate conversion
- [x] Screen → World → Grid coordinate systems
- [x] State management (MainMenu, GamePlay)
- [x] Main menu with state transitions
- [x] UI system basics (text, layout)
- [ ] Unit spawning and selection
- [ ] Turn-based movement
- [ ] AI opponents
- [ ] Combat system

## 🧠 Key ECS Concepts

### Components (Data)
Pure data structures with no logic. Examples:
- `GridPosition` - Where an entity is located
- `Unit` - Marks entity as a game unit
- `Tile` - Tile properties (walkable, type)

### Systems (Logic)
Functions that operate on components. Examples:
- `setup_grid` - Creates the game board
- `mouse_input_system` - Handles clicks
- `camera_pan_system` - Moves the camera

### Resources (Global State)
Singletons for game-wide data. Examples:
- `GridMap` - Grid dimensions and lookup
- `TurnManager` - Current turn state
- `SelectionState` - Selected unit tracking

## 🛠️ Technologies

- **Bevy 0.18** - Game engine with ECS architecture
- **Rust 2021** - Systems programming language
- **rand 0.8** - Random number generation (for AI)

## 📖 Learning Resources

- [Bevy Official Docs](https://docs.rs/bevy/0.18.0/bevy/)
- [Bevy Examples](https://github.com/bevyengine/bevy/tree/v0.18.0/examples)
- [Unofficial Bevy Cheat Book](https://bevy-cheatbook.github.io/)
- [ECS FAQ](https://github.com/SanderMertens/ecs-faq)

## ⚙️ Development Tips

### Fast Compilation

The project uses optimized debug builds:
```toml
[profile.dev]
opt-level = 1               # Basic optimization
[profile.dev.package."*"]
opt-level = 3               # Full optimization for dependencies
```

### Dynamic Linking (Optional)

For even faster iteration during development:
```toml
[dependencies]
bevy = { version = "0.18", features = ["dynamic_linking"] }
```

**Note:** Dynamic linking reduces compile times but increases runtime overhead. Use for development only.

## 🐛 Troubleshooting

**Game window doesn't appear:**
- Make sure you're running in debug mode: `cargo run`
- Check terminal for error messages

**Slow performance:**
- Run in release mode: `cargo run --release`
- The project is already optimized for debug builds

**Click detection not working:**
- Make sure the window has focus
- Check terminal logs - coordinates should be printed

## 📝 License

This project is a learning exercise. Feel free to use it however you'd like!

## 🤝 Contributing

This is a personal learning project, but suggestions and improvements are welcome! Open an issue or PR if you have ideas.

---

**Current Phase:** Phase 2 Complete ✅ → Ready for Phase 3 📋

**Next Steps:** Implement units and selection mechanics (see [Phase 3 docs](docs/phase3-units-selection.md))
