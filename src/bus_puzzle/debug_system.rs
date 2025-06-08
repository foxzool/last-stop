// src/bus_puzzle/debug_system.rs

use crate::{
    bus_puzzle,
    bus_puzzle::{
        calculate_network_efficiency, AgentState, GameOverData, GameState, GameStateEnum,
        LevelManager, PathfindingAgent,
    },
};
use bevy::prelude::*;

pub struct DebugSystem;

impl Plugin for DebugSystem {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                debug_info_system,
                debug_state_switch,
                debug_level_reset,       // 新增调试功能
                debug_level_status,      // 新增关卡状态调试
                debug_score_calculation, // 新增分数计算调试
                debug_trigger_game_over, // 新增：测试游戏失败菜单
            ),
        );
    }
}

fn debug_info_system(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    game_state: Res<bus_puzzle::GameState>,
    passengers: Query<&bus_puzzle::PathfindingAgent>,
    placed_segments: Query<&bus_puzzle::RouteSegment>,
    current_state: Res<State<bus_puzzle::GameStateEnum>>,
    time: Res<Time>,
) {
    if keyboard_input.just_pressed(KeyCode::F1) {
        info!("=== 详细调试信息 ===");
        info!("当前游戏状态: {:?}", current_state.get());
        info!("游戏时间: {:.1}秒", game_state.game_time);
        info!("系统运行时间: {:.1}秒", time.elapsed_secs());
        info!("总成本: {}", game_state.total_cost);
        info!("已放置路段数: {}", placed_segments.iter().count());

        // 乘客详细信息
        info!("乘客总数: {}", passengers.iter().count());
        let mut state_counts = std::collections::HashMap::new();
        for agent in passengers.iter() {
            *state_counts
                .entry(format!("{:?}", agent.state))
                .or_insert(0) += 1;
        }
        for (state, count) in state_counts {
            info!("  {}: {} 个乘客", state, count);
        }

        let arrived_count = passengers
            .iter()
            .filter(|agent| matches!(agent.state, bus_puzzle::AgentState::Arrived))
            .count();

        info!("已到达乘客数: {}", arrived_count);
        info!("目标完成情况: {:?}", game_state.objectives_completed);
        info!("当前得分: {}", game_state.score.total_score);

        // 关卡信息
        if let Some(level_data) = &game_state.current_level {
            info!("当前关卡: {} ({})", level_data.name, level_data.id);
            info!("关卡尺寸: {:?}", level_data.grid_size);
            info!("站点数: {}", level_data.stations.len());
            info!("乘客需求数: {}", level_data.passenger_demands.len());

            for (i, demand) in level_data.passenger_demands.iter().enumerate() {
                info!(
                    "  需求{}: {:?} {} -> {} (生成率: {}/秒)",
                    i, demand.color, demand.origin, demand.destination, demand.spawn_rate
                );
            }
        } else {
            warn!("没有关卡数据！");
        }

        info!("=== 按 F2 查看乘客生成详情，F3 手动生成测试乘客，F12 测试游戏失败菜单 ===");
    }
}

// 添加快速切换游戏状态的调试功能
fn debug_state_switch(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    current_state: Res<State<bus_puzzle::GameStateEnum>>,
    mut next_state: ResMut<NextState<bus_puzzle::GameStateEnum>>,
) {
    if keyboard_input.just_pressed(KeyCode::F4) {
        match current_state.get() {
            bus_puzzle::GameStateEnum::MainMenu => {
                next_state.set(bus_puzzle::GameStateEnum::Playing);
                info!("切换到游戏状态");
            }
            bus_puzzle::GameStateEnum::Playing => {
                next_state.set(bus_puzzle::GameStateEnum::MainMenu);
                info!("切换到主菜单");
            }
            _ => {
                next_state.set(bus_puzzle::GameStateEnum::Playing);
                info!("强制切换到游戏状态");
            }
        }
    }
}

/// F12 - 调试：手动触发游戏失败（测试失败菜单）
fn debug_trigger_game_over(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameStateEnum>>,
    mut commands: Commands,
    game_state: Res<GameState>,
    passengers: Query<&PathfindingAgent>,
) {
    if keyboard_input.just_pressed(KeyCode::F12) {
        let gave_up_count = passengers
            .iter()
            .filter(|agent| matches!(agent.state, AgentState::GaveUp))
            .count() as u32;

        // 模拟一个失败情况用于测试
        commands.insert_resource(GameOverData {
            reason: "手动触发测试失败".to_string(),
            final_score: game_state.score.total_score,
            game_time: game_state.game_time,
            passengers_gave_up: gave_up_count,
        });

        next_state.set(GameStateEnum::GameOver);
        info!("🧪 手动触发游戏失败菜单用于测试");
    }
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
    passengers: Query<&PathfindingAgent>,
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

            // 详细分数调试信息
            info!("=== 分数系统调试 ===");
            info!("当前分数: {}", game_state.score.total_score);
            info!("  基础分: {}", game_state.score.base_points);
            info!("  效率奖励: {}", game_state.score.efficiency_bonus);
            info!("  速度奖励: {}", game_state.score.speed_bonus);
            info!("  成本奖励: {}", game_state.score.cost_bonus);

            // 分数计算详情
            let network_efficiency = calculate_network_efficiency(&game_state, &passengers);
            info!("网络效率评分: {:.2}", network_efficiency);
            info!("游戏时间: {:.1}秒", game_state.game_time);
            info!("总成本: {}", game_state.total_cost);
            info!("已放置路段数: {}", game_state.placed_segments.len());

            // 乘客状态统计
            let total_passengers = passengers.iter().count();
            let arrived_count = passengers
                .iter()
                .filter(|agent| matches!(agent.state, AgentState::Arrived))
                .count();
            let gave_up_count = passengers
                .iter()
                .filter(|agent| matches!(agent.state, AgentState::GaveUp))
                .count();

            info!(
                "乘客状态: 总计={}, 到达={}, 放弃={}",
                total_passengers, arrived_count, gave_up_count
            );
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

/// F9 - 调试分数计算
fn debug_score_calculation(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    game_state: Res<GameState>,
    passengers: Query<&PathfindingAgent>,
) {
    if keyboard_input.just_pressed(KeyCode::F9) {
        info!("=== 分数计算详细调试 ===");

        if let Some(level_data) = &game_state.current_level {
            info!("关卡: {} ({})", level_data.name, level_data.id);
            info!("当前游戏时间: {:.1}秒", game_state.game_time);
            info!("当前总成本: {}", game_state.total_cost);
            info!("已放置路段数: {}", game_state.placed_segments.len());

            // 分数组成部分
            let base_points = level_data.scoring.base_points;
            info!("基础分数: {}", base_points);

            // 网络效率计算
            let network_efficiency = calculate_network_efficiency(&game_state, &passengers);
            let efficiency_bonus =
                (network_efficiency * level_data.scoring.efficiency_bonus as f32) as u32;
            info!(
                "网络效率: {:.2} -> 效率奖励: {}",
                network_efficiency, efficiency_bonus
            );

            // 速度奖励
            let speed_bonus = if game_state.game_time < 60.0 {
                level_data.scoring.speed_bonus
            } else {
                0
            };
            info!(
                "速度奖励: {} (条件: <60秒, 当前: {:.1}秒)",
                speed_bonus, game_state.game_time
            );

            // 成本奖励
            let cost_threshold = match level_data.id.as_str() {
                "tutorial_01" => 10,
                "level_02_transfer" => 15,
                "level_03_multiple_routes" => 25,
                "level_04_time_pressure" => 20,
                _ => 15,
            };
            let cost_bonus = if game_state.total_cost <= cost_threshold {
                level_data.scoring.cost_bonus
            } else {
                0
            };
            info!(
                "成本奖励: {} (条件: ≤{}, 当前: {})",
                cost_bonus, cost_threshold, game_state.total_cost
            );

            // 总分
            let total_calculated = base_points + efficiency_bonus + speed_bonus + cost_bonus;
            info!(
                "计算总分: {} + {} + {} + {} = {}",
                base_points, efficiency_bonus, speed_bonus, cost_bonus, total_calculated
            );
            info!("当前实际总分: {}", game_state.score.total_score);

            // 乘客统计
            let total_passengers = passengers.iter().count();
            let arrived_count = passengers
                .iter()
                .filter(|agent| matches!(agent.state, AgentState::Arrived))
                .count();
            let gave_up_count = passengers
                .iter()
                .filter(|agent| matches!(agent.state, AgentState::GaveUp))
                .count();

            info!(
                "乘客统计: 总计={}, 到达={}, 放弃={}",
                total_passengers, arrived_count, gave_up_count
            );
            info!("目标完成情况: {:?}", game_state.objectives_completed);
        }
    }
}
