// 模块声明
pub mod components;
pub mod config;
pub mod connection_system;
pub mod debug_info;
pub mod events;
pub mod interaction;
pub mod level_system;
pub mod passenger_movement_debug;
pub mod pathfinding;
pub mod resources;
pub mod splash;
pub mod ui_audio;
pub mod utils;

use bevy::{
    audio::{PlaybackMode, Volume},
    platform::collections::HashMap,
};
// 重新导出主要类型
pub use components::*;
pub use config::*;
pub use debug_info::*;
pub use events::*;
pub use interaction::*;
pub use level_system::*;
pub use passenger_movement_debug::*;
pub use pathfinding::*;
pub use resources::*;
pub use ui_audio::*;
pub use utils::*;

use crate::bus_puzzle::{
    connection_system::FixedConnectionSystemPlugin, splash::SplashPlugin, LevelCompleteData,
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
            PassengerMovementDebugPlugin,
            FixedConnectionSystemPlugin,
            DebugInfoPlugin,
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
            .add_systems(OnExit(GameStateEnum::MainMenu), load_current_level)
            .add_systems(OnEnter(GameStateEnum::Loading), load_current_level)
            .add_systems(OnExit(GameStateEnum::Loading), cleanup_loading_state)
            // .add_systems(OnEnter(GameStateEnum::MainMenu), default_game_state)
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
    time: Res<Time>,
) {
    level_manager.current_level_index = 0;

    let mut tutorial_level = create_tutorial_level();

    // 确保初始化时所有乘客需求计数都为0
    for demand in &mut tutorial_level.passenger_demands {
        demand.spawned_count = 0;
    }

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
    mut level_complete_data: ResMut<LevelCompleteData>, // 添加LevelCompleteData
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

    // 第一步：重置关卡完成数据
    level_complete_data.final_score = 0;
    level_complete_data.completion_time = 0.0;

    // 第二步：清理所有现有的游戏实体
    cleanup_game_world(
        &mut commands,
        existing_tiles,
        existing_stations,
        existing_segments,
        existing_passengers,
        existing_previews,
    );

    // 第三步：重置寻路图
    pathfinding_graph.connections.clear();
    pathfinding_graph.nodes.clear();
    pathfinding_graph.station_lookup.clear();
    pathfinding_graph.route_network.clear();

    // 第四步：获取关卡数据
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

    // 第五步：重置游戏状态
    reset_game_state(&mut game_state, &level_data, time.elapsed_secs());

    // 第六步：重新生成关卡地图
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

    // 创建一个新的关卡数据副本，重置所有乘客需求的计数
    let mut reset_level_data = level_data.clone();
    for demand in &mut reset_level_data.passenger_demands {
        demand.spawned_count = 0; // 重置乘客生成计数
        info!(
            "重置乘客需求: {:?} {} -> {} (计数重置为0)",
            demand.color, demand.origin, demand.destination
        );
    }

    // 设置关卡数据
    game_state.current_level = Some(reset_level_data);

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

fn update_game_score(mut game_state: ResMut<GameState>, passengers: Query<&PathfindingAgent>) {
    if let Some(level_data) = &game_state.current_level {
        let base_points = level_data.scoring.base_points;

        // 网络效率奖励：基于乘客到达率和路径效率
        let network_efficiency = calculate_network_efficiency(&game_state, &passengers);
        let efficiency_bonus =
            (network_efficiency * level_data.scoring.efficiency_bonus as f32) as u32;

        // 速度奖励：根据关卡配置的时间阈值
        let speed_bonus = if game_state.game_time < 60.0 {
            level_data.scoring.speed_bonus
        } else {
            0
        };

        // 成本奖励：根据关卡配置的成本阈值
        let cost_threshold = match level_data.id.as_str() {
            "tutorial_01" => 10, // 教学关卡：更宽松的成本要求
            "level_02_transfer" => 15,
            "level_03_multiple_routes" => 25,
            "level_04_time_pressure" => 20,
            _ => 15, // 默认值
        };

        let cost_bonus = if game_state.total_cost <= cost_threshold {
            level_data.scoring.cost_bonus
        } else {
            0
        };

        // 更新分数结构
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
    mut commands: Commands,
    audio_assets: Res<AudioAssets>,
    audio_settings: Res<AudioSettings>,
) {
    let gave_up_count = passengers
        .iter()
        .filter(|agent| matches!(agent.state, AgentState::GaveUp))
        .count() as u32;

    // 检查乘客放弃失败条件
    if gave_up_count > 3 {
        // 设置失败数据
        commands.insert_resource(GameOverData {
            reason: format!("太多乘客放弃了行程 ({} 人)", gave_up_count),
            final_score: game_state.score.total_score,
            game_time: game_state.game_time,
            passengers_gave_up: gave_up_count,
        });

        // 播放失败音效
        if !audio_settings.is_muted {
            commands.spawn((
                AudioPlayer::new(audio_assets.error_sound.clone()),
                PlaybackSettings {
                    mode: PlaybackMode::Despawn,
                    volume: Volume::Linear(
                        audio_settings.sfx_volume * audio_settings.master_volume * 1.2,
                    ),
                    ..default()
                },
            ));
        }

        next_state.set(GameStateEnum::GameOver);
        warn!("游戏失败：太多乘客放弃了行程 ({} 人)", gave_up_count);
        return;
    }

    // 检查时间限制失败条件
    if let Some(level_data) = &game_state.current_level {
        for objective in &level_data.objectives {
            if let ObjectiveType::TimeLimit(time_limit) = &objective.condition_type {
                if game_state.game_time > *time_limit {
                    // 设置失败数据
                    commands.insert_resource(GameOverData {
                        reason: format!(
                            "时间超限 ({:.1}s / {:.1}s)",
                            game_state.game_time, time_limit
                        ),
                        final_score: game_state.score.total_score,
                        game_time: game_state.game_time,
                        passengers_gave_up: gave_up_count,
                    });

                    // 播放失败音效
                    if !audio_settings.is_muted {
                        commands.spawn((
                            AudioPlayer::new(audio_assets.error_sound.clone()),
                            PlaybackSettings {
                                mode: PlaybackMode::Despawn,
                                volume: Volume::Linear(
                                    audio_settings.sfx_volume * audio_settings.master_volume * 1.2,
                                ),
                                ..default()
                            },
                        ));
                    }

                    next_state.set(GameStateEnum::GameOver);
                    warn!("游戏失败：时间超限 ({:.1}s)", game_state.game_time);
                    return;
                }
            }
        }

        // 可选：预算超支检查 (严重超支才算失败)
        for objective in &level_data.objectives {
            if let ObjectiveType::MaxCost(max_cost) = &objective.condition_type {
                // 给予一些缓冲，避免意外超支导致立即失败
                let cost_limit = max_cost + (max_cost / 2); // 150% 的预算作为硬限制
                if game_state.total_cost > cost_limit {
                    commands.insert_resource(GameOverData {
                        reason: format!("预算严重超支 ({} / {})", game_state.total_cost, max_cost),
                        final_score: game_state.score.total_score,
                        game_time: game_state.game_time,
                        passengers_gave_up: gave_up_count,
                    });

                    if !audio_settings.is_muted {
                        commands.spawn((
                            AudioPlayer::new(audio_assets.error_sound.clone()),
                            PlaybackSettings {
                                mode: PlaybackMode::Despawn,
                                volume: Volume::Linear(
                                    audio_settings.sfx_volume * audio_settings.master_volume * 1.2,
                                ),
                                ..default()
                            },
                        ));
                    }

                    next_state.set(GameStateEnum::GameOver);
                    warn!("游戏失败：预算严重超支");
                    return;
                }
            }
        }
    }
}
