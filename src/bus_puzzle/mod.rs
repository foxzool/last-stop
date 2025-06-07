// 模块声明
pub mod components;
pub mod config;
pub mod events;
pub mod interaction;
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
pub use events::*;
pub use interaction::*;
pub use level_system::*;
pub use passenger_movement_debug::*;
pub use passenger_test::*;
pub use pathfinding::*;
pub use resources::*;
pub use ui_audio::*;
pub use utils::*;

use crate::bus_puzzle::splash::SplashPlugin;
use bevy::prelude::*;
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
    // 初始化第一个关卡
    level_manager.current_level_index = 0;

    // 创建教学关卡
    let tutorial_level = create_tutorial_level();
    generate_level_map(
        &mut commands,
        &asset_server,
        &tutorial_level,
        level_manager.tile_size,
    );

    // 初始化库存
    let mut inventory = HashMap::new();
    for segment in &tutorial_level.available_segments {
        inventory.insert(segment.segment_type.clone(), segment.count);
    }

    game_state.current_level = Some(tutorial_level);
    game_state.player_inventory = inventory;
    game_state.objectives_completed = vec![false; 1]; // 教学关卡只有一个目标

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
        // 这里应该从文件或数据库加载关卡
        // 现在使用示例关卡
        let level_data = match level_id.as_str() {
            "tutorial_01" => create_tutorial_level(),
            _ => create_tutorial_level(), // 回退到教学关卡
        };

        // 重置游戏状态
        game_state.current_level = Some(level_data.clone());
        game_state.placed_segments.clear();
        game_state.total_cost = 0;
        game_state.game_time = 0.0;
        game_state.is_paused = false;
        game_state.objectives_completed = vec![false; level_data.objectives.len()];
        game_state.score = GameScore::default();

        // 重置库存
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

        // 计算效率奖励
        let network_efficiency = calculate_network_efficiency(&game_state, &passengers);
        let efficiency_bonus =
            (network_efficiency * level_data.scoring.efficiency_bonus as f32) as u32;

        // 计算速度奖励（基于剩余时间）
        let speed_bonus = if game_state.game_time < 60.0 {
            level_data.scoring.speed_bonus
        } else {
            0
        };

        // 计算成本奖励（基于节约的成本）
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
    // 检查是否有乘客因为耐心耗尽而放弃
    let gave_up_count = passengers
        .iter()
        .filter(|agent| matches!(agent.state, AgentState::GaveUp))
        .count();

    // 如果太多乘客放弃，游戏失败
    if gave_up_count > 3 {
        next_state.set(GameStateEnum::GameOver);
        warn!("太多乘客放弃了行程，游戏失败");
    }

    // 检查时间限制
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
