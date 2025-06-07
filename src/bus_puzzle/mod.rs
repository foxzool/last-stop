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
pub use events::*;
pub use interaction::*;
pub use level_system::*;
pub use passenger_movement_debug::*;
pub use passenger_test::*;
pub use pathfinding::*;
pub use resources::*;
pub use ui_audio::*;
pub use utils::*;

use crate::bus_puzzle::{
    connection_system::FixedConnectionSystemPlugin,
    junction_pathfinding::JunctionPathfindingPlugin, splash::SplashPlugin,
};
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
            JunctionPathfindingPlugin,
            FixedConnectionSystemPlugin,
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
            .add_systems(OnExit(GameStateEnum::Loading), cleanup_loading_state)
            .add_systems(
                Update,
                (
                    update_game_score,
                    check_level_failure_conditions,
                    debug_level_reset,  // 新增调试功能
                    debug_level_status, // 新增关卡状态调试
                )
                    .run_if(in_state(GameStateEnum::Playing)),
            );
    }
}

fn initialize_game(
    mut commands: Commands,
    mut level_manager: ResMut<LevelManager>,
    mut game_state: ResMut<GameState>,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
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
        inventory.insert(segment.segment_type, segment.count);
    }

    game_state.current_level = Some(tutorial_level);
    game_state.player_inventory = inventory;
    game_state.objectives_completed = vec![false; 1];
    game_state.level_start_time = time.elapsed_secs(); // 设置开始时间

    info!(
        "游戏初始化完成，开始时间: {:.1}s",
        game_state.level_start_time
    );
}

fn load_current_level(
    mut commands: Commands,
    mut game_state: ResMut<GameState>,
    level_manager: Res<LevelManager>,
    mut next_state: ResMut<NextState<GameStateEnum>>,
    asset_server: Res<AssetServer>,
    mut pathfinding_graph: ResMut<PathfindingGraph>,
    time: Res<Time>,
    // 清理现有的游戏实体
    existing_tiles: Query<Entity, With<GridTile>>,
    existing_stations: Query<Entity, With<StationEntity>>,
    existing_segments: Query<Entity, With<RouteSegment>>,
    existing_passengers: Query<Entity, With<PathfindingAgent>>,
    existing_previews: Query<Entity, With<SegmentPreview>>,
) {
    info!(
        "开始加载关卡，当前索引: {}",
        level_manager.current_level_index
    );

    // 第一步：清理所有现有的游戏实体
    cleanup_game_world(
        &mut commands,
        existing_tiles,
        existing_stations,
        existing_segments,
        existing_passengers,
        existing_previews,
    );

    // 第二步：重置寻路图
    pathfinding_graph.connections.clear();
    pathfinding_graph.nodes.clear();
    pathfinding_graph.station_lookup.clear();
    pathfinding_graph.route_network.clear();

    // 第三步：获取关卡数据
    let level_data = if let Some(level_id) = level_manager
        .available_levels
        .get(level_manager.current_level_index)
    {
        match level_id.as_str() {
            "tutorial_01" => create_tutorial_level(),
            "level_02_transfer" => create_transfer_level(),
            "level_03_multiple_routes" => create_multiple_routes_level(),
            "level_04_time_pressure" => create_time_pressure_level(),
            _ => {
                warn!("未知关卡ID: {}, 使用教学关卡", level_id);
                create_tutorial_level()
            }
        }
    } else {
        warn!("无效的关卡索引: {}", level_manager.current_level_index);
        return;
    };

    // 第四步：重置游戏状态
    reset_game_state(&mut game_state, &level_data, time.elapsed_secs());

    // 第五步：重新生成关卡地图
    generate_level_map(
        &mut commands,
        &asset_server,
        &level_data,
        level_manager.tile_size,
    );

    next_state.set(GameStateEnum::Playing);
    info!("关卡加载完成: {}", level_data.name);
}

/// 清理游戏世界中的所有实体
fn cleanup_game_world(
    commands: &mut Commands,
    tiles: Query<Entity, With<GridTile>>,
    stations: Query<Entity, With<StationEntity>>,
    segments: Query<Entity, With<RouteSegment>>,
    passengers: Query<Entity, With<PathfindingAgent>>,
    previews: Query<Entity, With<SegmentPreview>>,
) {
    info!("清理游戏世界实体...");

    // 清理地形瓦片
    for entity in tiles.iter() {
        commands.entity(entity).despawn();
    }

    // 清理站点
    for entity in stations.iter() {
        commands.entity(entity).despawn();
    }

    // 清理路线段
    for entity in segments.iter() {
        commands.entity(entity).despawn();
    }

    // 清理乘客
    for entity in passengers.iter() {
        commands.entity(entity).despawn();
    }

    // 清理预览
    for entity in previews.iter() {
        commands.entity(entity).despawn();
    }

    info!("游戏世界清理完成");
}

/// 重置游戏状态
fn reset_game_state(game_state: &mut GameState, level_data: &LevelData, system_time: f32) {
    info!("重置游戏状态...");

    // 设置关卡数据
    game_state.current_level = Some(level_data.clone());

    // 清理已放置的路线段
    game_state.placed_segments.clear();

    // 重置计分和计时
    game_state.total_cost = 0;
    game_state.game_time = 0.0;
    game_state.level_start_time = system_time; // 记录关卡开始时间
    game_state.is_paused = false;
    game_state.score = GameScore::default();

    // 重置目标完成状态
    game_state.objectives_completed = vec![false; level_data.objectives.len()];

    // 重置乘客统计
    game_state.passenger_stats = PassengerStats {
        total_spawned: 0,
        total_arrived: 0,
        total_gave_up: 0,
    };

    // 重置库存
    let mut inventory = HashMap::new();
    for segment in &level_data.available_segments {
        inventory.insert(segment.segment_type, segment.count);
    }
    game_state.player_inventory = inventory;

    info!("游戏状态重置完成，关卡开始时间: {:.1}s", system_time);
}

/// 清理加载状态时的临时资源
fn cleanup_loading_state() {
    info!("清理加载状态");
}

/// F5 - 调试关卡重置功能
fn debug_level_reset(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameStateEnum>>,
    game_state: Res<GameState>,
) {
    if keyboard_input.just_pressed(KeyCode::F5) {
        info!("🔄 手动触发关卡重置");
        info!("当前游戏时间: {:.1}s", game_state.game_time);
        info!(
            "当前乘客统计: 生成={}, 到达={}, 放弃={}",
            game_state.passenger_stats.total_spawned,
            game_state.passenger_stats.total_arrived,
            game_state.passenger_stats.total_gave_up
        );
        info!("当前库存状态: {:?}", game_state.player_inventory);

        next_state.set(GameStateEnum::Loading);
    }
}

/// F6 - 调试关卡状态
fn debug_level_status(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    level_manager: Res<LevelManager>,
    game_state: Res<GameState>,
) {
    if keyboard_input.just_pressed(KeyCode::F6) {
        info!("=== 关卡状态调试 ===");
        info!("当前关卡索引: {}", level_manager.current_level_index);
        info!("总关卡数: {}", level_manager.available_levels.len());

        for (i, level_id) in level_manager.available_levels.iter().enumerate() {
            let is_current = i == level_manager.current_level_index;
            let is_unlocked = level_manager
                .unlocked_levels
                .get(i)
                .copied()
                .unwrap_or(false);
            let marker = if is_current { " <- 当前" } else { "" };
            let status = if is_unlocked {
                "已解锁"
            } else {
                "未解锁"
            };

            info!("  关卡 {}: {} ({}){}", i, level_id, status, marker);
        }

        if let Some(level_data) = &game_state.current_level {
            info!("当前关卡详情:");
            info!("  ID: {}", level_data.id);
            info!("  名称: {}", level_data.name);
            info!("  难度: {}", level_data.difficulty);
            info!("  目标数: {}", level_data.objectives.len());
        }

        let next_index = level_manager.current_level_index + 1;
        if next_index < level_manager.available_levels.len() {
            info!(
                "下一关: {} (索引: {})",
                level_manager.available_levels[next_index], next_index
            );
        } else {
            info!("这是最后一关！");
        }
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
