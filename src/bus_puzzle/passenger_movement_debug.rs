// src/bus_puzzle/passenger_movement_debug.rs - 简化的调试系统

use crate::bus_puzzle::{GameStateEnum, PathfindingAgent, PathfindingGraph};
use bevy::prelude::*;

pub struct PassengerMovementDebugPlugin;

impl Plugin for PassengerMovementDebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (debug_passenger_states, debug_pathfinding_status)
                .run_if(in_state(GameStateEnum::Playing)),
        );
    }
}

// F7 - 调试乘客状态
fn debug_passenger_states(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    passengers: Query<(Entity, &PathfindingAgent, &Transform)>,
) {
    if keyboard_input.just_pressed(KeyCode::F7) {
        info!("=== 乘客状态调试 ===");

        for (entity, agent, transform) in passengers.iter() {
            info!("乘客 {:?}:", entity);
            info!("  颜色: {:?}", agent.color);
            info!("  路线: {} -> {}", agent.origin, agent.destination);
            info!("  状态: {:?}", agent.state);
            info!("  位置: {:?}", transform.translation);
            info!("  路径长度: {}", agent.current_path.len());
            info!(
                "  当前步骤: {}/{}",
                agent.current_step,
                agent.current_path.len()
            );
            info!("  耐心: {:.1}/{:.1}", agent.patience, agent.max_patience);
            info!("  等待时间: {:.1}", agent.waiting_time);

            if !agent.current_path.is_empty() {
                info!("  路径详情:");
                for (i, node) in agent.current_path.iter().enumerate() {
                    let marker = if i == agent.current_step {
                        " -> "
                    } else {
                        "    "
                    };
                    info!(
                        "{}步骤 {}: {:?} 类型: {:?}",
                        marker, i, node.position, node.node_type
                    );
                }
            } else {
                warn!("  没有路径！");
            }
            info!(""); // 空行分隔
        }
    }
}

// F11 - 调试寻路图状态
fn debug_pathfinding_status(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    pathfinding_graph: Res<PathfindingGraph>,
    passengers: Query<&PathfindingAgent>,
) {
    if keyboard_input.just_pressed(KeyCode::F11) {
        info!("=== 寻路系统状态 ===");
        info!("寻路图节点数: {}", pathfinding_graph.nodes.len());
        info!("寻路图连接数: {}", pathfinding_graph.connections.len());
        info!("站点查找表: {}", pathfinding_graph.station_lookup.len());

        for (name, pos) in &pathfinding_graph.station_lookup {
            info!("  站点 {}: {:?}", name, pos);
        }

        info!("乘客状态统计:");
        let mut state_counts = std::collections::HashMap::new();
        for agent in passengers.iter() {
            *state_counts
                .entry(format!("{:?}", agent.state))
                .or_insert(0) += 1;
        }
        for (state, count) in state_counts {
            info!("  {}: {} 个乘客", state, count);
        }
    }
}
