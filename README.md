# Last Stop - Bus Route Puzzle Game

A bus route planning puzzle game built with the Bevy engine, where players place route segments to connect stations and
help passengers reach their destinations.

## 🎮 Game Overview

### Core Gameplay

- **Route Building**: Place and rotate bus route segments (straight, curves, T-junctions, etc.) on a grid map
- **Passenger Transport**: Passengers travel from origin stations and need transfers to reach destinations
- **Chain Reactions**: Each new route affects the entire traffic network's passenger flow distribution
- **Smart Bus System**: Autonomous buses using advanced pathfinding algorithms

### Key Features

- 🚌 **Intelligent Bus System**: Buses automatically discover routes and operate
- 🔄 **Dynamic Transfers**: Passengers can switch between different lines at transfer points
- ⏱️ **Real-time Simulation**: Passengers have patience values requiring timely transport services
- 🎯 **Diverse Objectives**: Multi-dimensional challenges including efficiency, cost, and time
- 📊 **Detailed Analytics**: Complete passenger and operational data tracking

## 🛠️ Technical Implementation

### Architecture

- **Bevy ECS**: Modular architecture based on Entity Component System
- **Smart Pathfinding**: A* algorithm-driven route planning system
- **State Management**: Complete game state and scene management
- **UI System**: Responsive user interface and audio system

### Core Systems

1. **Route System** (`route_segment.rs`)
    - 6 route segment types: straight, curve, T-split, cross, bridge, tunnel
    - Dynamic rotation and connection validation
    - Terrain restriction handling

2. **Pathfinding System** (`pathfinding.rs`)
    - A* algorithm implementation
    - Multi-modal path calculation (walking, bus, transfer)
    - Real-time path optimization

3. **Bus System** (`bus_pathfinding_system.rs`)
    - Intelligent route discovery
    - Autonomous driving and station stops
    - Dynamic passenger management

4. **Passenger System** (`passenger_boarding_system.rs`)
    - Smart boarding/alighting logic
    - Waiting and riding state management
    - Patience and satisfaction systems

## 🎯 Level Design

### Progressive Difficulty

1. **Tutorial Level**: Learn basic operations and connection concepts
2. **Transfer Challenge**: Master multi-route coordination and transfer mechanics
3. **Network Optimization**: Find optimal solutions under constraints
4. **Time Pressure**: Test quick response and adaptation abilities

### Scoring System

- **Base Points**: Complete basic objectives
- **Efficiency Bonus**: Optimize transfer counts and path lengths
- **Speed Bonus**: Complete challenges quickly
- **Cost Bonus**: Save construction costs

## 🕹️ Controls

### Basic Operations

- **Left Mouse**: Place selected route segment
- **Right Mouse**: Rotate route segment
- **Delete/X Key**: Remove route segment at cursor position
- **WASD/Arrow Keys**: Move camera
- **Mouse Wheel**: Zoom view
- **Escape**: Pause/Resume game

### Debug Hotkeys

- **F1**: Show detailed debug information
- **F2**: Passenger spawn statistics
- **F3**: Manually spawn test passenger
- **F4**: Smart bus route discovery
- **F5**: Smart bus status debugging
- **F6**: Passenger boarding system debug
- **F7**: Passenger movement state details
- **F8**: Connection system debug
- **F9**: Score calculation debug
- **F12**: Test game over interface

## 🚀 Build and Run

### System Requirements

- Rust 1.75+
- OpenGL/Vulkan compatible graphics card

### Build Steps

```bash
# Clone repository
git clone <repository-url>
cd bus-puzzle-game

# Development build (includes debug features)
cargo run --features dev

# Release build
cargo run --release
```

### Web Build

```bash
# Install trunk
cargo install trunk

# Build WASM version
trunk build --release
```

## 📁 Project Structure

```
src/
├── main.rs                 # Main entry point
├── bus_puzzle/             # Core game module
│   ├── mod.rs              # Module exports
│   ├── components.rs       # Game component definitions
│   ├── config.rs           # Game configuration constants
│   ├── level_system.rs     # Level system
│   ├── pathfinding.rs      # Pathfinding algorithms
│   ├── bus_pathfinding_system.rs  # Smart bus system
│   ├── passenger_boarding_system.rs  # Passenger boarding system
│   ├── connection_system.rs        # Connection validation system
│   ├── interaction.rs      # Player interaction
│   ├── ui_audio.rs         # UI and audio
│   ├── debug_info.rs       # Debug information
│   └── ...                 # Other system modules
└── dev_tools.rs            # Development tools (dev only)
```

## 🎨 Assets

### Texture Assets

```
assets/textures/
├── routes/                 # Route segment textures
│   ├── straight.png
│   ├── curve.png
│   ├── t_split.png
│   └── ...
├── stations/              # Station textures
│   ├── bus_stop.png
│   └── ...
├── passengers/            # Passenger icons
│   ├── red.png
│   └── ...
└── terrain/              # Terrain textures
    ├── grass.png
    └── ...
```

### Audio Assets

```
assets/audio/
├── background_music.ogg
├── place_segment.ogg
├── passenger_arrive.ogg
└── ...
```

## 🔧 Development Features

### Debug System

- Real-time connection status visualization
- Passenger behavior tracking
- Performance metrics monitoring
- State machine debugging

### Hot Reload

The game supports asset hot reloading for convenient development and debugging.

### Extensibility

- Modular component system
- Configurable level data
- Plugin-based feature extensions

## 🎮 Game Mechanics Deep Dive

### Chain Reaction System

- **Passenger Flow Chains**: New routes redistribute passenger flows
- **Transfer Chains**: Multi-route intersections create new travel possibilities
- **Time Chains**: Bus intervals affect passenger waiting experience

### Smart Buses

- Uses passenger-validated pathfinding algorithms
- Automatically discovers optimal operating routes
- Dynamic turnaround and round-trip operations
- Intelligent passenger loading and station stops

### Passenger Behavior

- Realistic waiting and riding states
- Patience-based abandonment mechanism
- Smart boarding/alighting decisions
- Diverse travel demands

## 📈 Performance Optimization

- ECS architecture ensures efficient system updates
- Intelligent path caching mechanisms
- Optimized rendering pipeline
- Memory-friendly resource management

## 🤝 Contributing

Contributions are welcome! Please ensure:

1. Follow existing code style
2. Add appropriate documentation comments
3. Include necessary test cases
4. Update relevant documentation

## 📄 License

This project is dual-licensed under:

### Apache License 2.0 or MIT License

You may use this project under either license:

- **Apache License 2.0** - See [LICENSE-APACHE](LICENSE-APACHE) file
- **MIT License** - See [LICENSE-MIT](LICENSE-MIT) file

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this project by you, as
defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

## 🙏 Acknowledgments

- [Bevy Engine](https://bevyengine.org/) - Excellent Rust game engine
- Game design inspired by classic transport planning games

---

Enjoy building your bus empire! 🚌✨
