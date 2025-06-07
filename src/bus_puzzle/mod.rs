// 模块声明
pub mod components;
pub mod config;
pub mod connection_debug;
pub mod connection_fix;
pub mod connection_system;
pub mod events;
pub mod interaction;
pub mod junction_movement;
pub mod junction_pathfinding;
pub mod level_system;
pub mod passenger_movement_debug;
pub mod passenger_test;
pub mod pathfinding;
pub mod resources;
pub mod splash;
pub mod ui_audio;
pub mod utils;

use bevy::platform::collections::HashMap;
// 重新导出主要类型
pub use components::*;
pub use config::*;
pub use connection_debug::*;
pub use connection_fix::*;
pub use events::*;
pub use interaction::*;
pub use junction_movement::*;
pub use level_system::*;
pub use passenger_movement_debug::*;
pub use passenger_test::*;
pub use pathfinding::*;
pub use resources::*;
pub use ui_audio::*;
pub use utils::*;

use crate::bus_puzzle::{connection_system::ConnectionSystemPlugin, splash::SplashPlugin};
use bevy::prelude::*;
use crate::bus_puzzle::junction_pathfinding::JunctionPathfindingPlugin;
// ============ 游戏主循环集成 ============

pub struct BusPuzzleGamePlugin;

impl Plugin for BusPuzzleGamePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            SplashPlugin,
            LevelGenerationPlugin,
            PathfindingPlugin,
            PuzzleInteractionPlugin,
            GameUIPlugin,
            PassengerTestPlugin,
            PassengerMovementDebugPlugin,
            // ConnectionDebugPlugin,
            // ConnectionFixPlugin,
            JunctionPathfindingPlugin,
            JunctionMovementPlugin,
            ConnectionSystemPlugin,
        ));

        app.init_resource::<GameState>()
            .init_state::<GameStateEnum>();

        app.add_event::<SegmentPlacedEvent>()
            .add_event::<SegmentRemovedEvent>()
            .add_event::<ObjectiveCompletedEvent>()
            .add_event::<LevelCompletedEvent>()
            .add_event::<InventoryUpdatedEvent>()
            .add_event::<PassengerSpawnedEvent>()
            .add_event::<PassengerArrivedEvent>();

        app.add_systems(Startup, initialize_game)
            .add_systems(OnEnter(GameStateEnum::Loading), load_current_level)
            .add_systems(
                Update,
                (update_game_score, check_level_failure_conditions)
                    .run_if(in_state(GameStateEnum::Playing)),
            );
    }
}

fn initialize_game(
    mut commands: Commands,
    mut level_manager: ResMut<LevelManager>,
    mut game_state: ResMut<GameState>,
    asset_server: Res<AssetServer>,
) {
    level_manager.current_level_index = 0;

    let tutorial_level = create_tutorial_level();
    generate_level_map(
        &mut commands,
        &asset_server,
        &tutorial_level,
        level_manager.tile_size,
    );

    let mut inventory = HashMap::new();
    for segment in &tutorial_level.available_segments {
        inventory.insert(segment.segment_type.clone(), segment.count);
    }

    game_state.current_level = Some(tutorial_level);
    game_state.player_inventory = inventory;
    game_state.objectives_completed = vec![false; 1];

    info!("游戏初始化完成");
}

fn load_current_level(
    mut game_state: ResMut<GameState>,
    level_manager: Res<LevelManager>,
    mut next_state: ResMut<NextState<GameStateEnum>>,
) {
    if let Some(level_id) = level_manager
        .available_levels
        .get(level_manager.current_level_index)
    {
        let level_data = match level_id.as_str() {
            "tutorial_01" => create_tutorial_level(),
            _ => create_tutorial_level(),
        };

        game_state.current_level = Some(level_data.clone());
        game_state.placed_segments.clear();
        game_state.total_cost = 0;
        game_state.game_time = 0.0;
        game_state.is_paused = false;
        game_state.objectives_completed = vec![false; level_data.objectives.len()];
        game_state.score = GameScore::default();

        let mut inventory = HashMap::new();
        for segment in &level_data.available_segments {
            inventory.insert(segment.segment_type.clone(), segment.count);
        }
        game_state.player_inventory = inventory;

        next_state.set(GameStateEnum::Playing);
        info!("加载关卡: {}", level_id);
    }
}

fn update_game_score(mut game_state: ResMut<GameState>, passengers: Query<&PathfindingAgent>) {
    if let Some(level_data) = &game_state.current_level {
        let base_points = level_data.scoring.base_points;

        let network_efficiency = calculate_network_efficiency(&game_state, &passengers);
        let efficiency_bonus =
            (network_efficiency * level_data.scoring.efficiency_bonus as f32) as u32;

        let speed_bonus = if game_state.game_time < 60.0 {
            level_data.scoring.speed_bonus
        } else {
            0
        };

        let cost_bonus = if game_state.total_cost < 15 {
            level_data.scoring.cost_bonus
        } else {
            0
        };

        game_state.score = GameScore {
            base_points,
            efficiency_bonus,
            speed_bonus,
            cost_bonus,
            total_score: base_points + efficiency_bonus + speed_bonus + cost_bonus,
        };
    }
}

fn check_level_failure_conditions(
    game_state: Res<GameState>,
    passengers: Query<&PathfindingAgent>,
    mut next_state: ResMut<NextState<GameStateEnum>>,
) {
    let gave_up_count = passengers
        .iter()
        .filter(|agent| matches!(agent.state, AgentState::GaveUp))
        .count();

    if gave_up_count > 3 {
        next_state.set(GameStateEnum::GameOver);
        warn!("太多乘客放弃了行程，游戏失败");
    }

    if let Some(level_data) = &game_state.current_level {
        for objective in &level_data.objectives {
            if let ObjectiveType::TimeLimit(time_limit) = &objective.condition_type {
                if game_state.game_time > *time_limit {
                    next_state.set(GameStateEnum::GameOver);
                    warn!("时间超限，游戏失败");
                }
            }
        }
    }
}
