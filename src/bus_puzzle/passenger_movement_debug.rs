// src/bus_puzzle/passenger_movement_debug.rs

use crate::bus_puzzle::{AgentState, GameStateEnum, GridPos, PathfindingAgent, PathfindingGraph};
use bevy::prelude::*;

pub struct PassengerMovementDebugPlugin;

impl Plugin for PassengerMovementDebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                debug_passenger_states,
                debug_pathfinding_graph,
                simple_movement_test,
                force_passenger_movement,
            )
                .run_if(in_state(GameStateEnum::Playing)),
        );
    }
}

// 调试乘客状态
fn debug_passenger_states(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    passengers: Query<(Entity, &PathfindingAgent, &Transform)>,
    pathfinding_graph: Res<PathfindingGraph>,
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

        // 检查寻路图状态
        info!("寻路图状态:");
        info!("  节点数量: {}", pathfinding_graph.nodes.len());
        info!("  连接数量: {}", pathfinding_graph.connections.len());
        info!("  站点查找表: {}", pathfinding_graph.station_lookup.len());

        for (name, pos) in &pathfinding_graph.station_lookup {
            info!("    {}: {:?}", name, pos);
        }
    }
}

// 调试寻路图
fn debug_pathfinding_graph(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    pathfinding_graph: Res<PathfindingGraph>,
) {
    if keyboard_input.just_pressed(KeyCode::F8) {
        info!("=== 寻路图详细信息 ===");

        info!("节点详情:");
        for (pos, node) in &pathfinding_graph.nodes {
            info!(
                "  位置 {:?}: 类型 {:?}, 站点名 {:?}, 可访问 {}",
                pos, node.node_type, node.station_name, node.is_accessible
            );
        }

        info!("连接详情:");
        for (from, connections) in &pathfinding_graph.connections {
            info!("  从 {:?}:", from);
            for connection in connections {
                info!(
                    "    到 {:?}: 成本 {:.1}, 类型 {:?}, 路线 {:?}",
                    connection.to, connection.cost, connection.connection_type, connection.route_id
                );
            }
        }
    }
}

// 简单移动测试 - 让乘客直线移动到目标位置
fn simple_movement_test(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut passengers: Query<(&mut PathfindingAgent, &mut Transform)>,
    time: Res<Time>,
) {
    if keyboard_input.just_pressed(KeyCode::F9) {
        info!("启动简单移动测试 - 乘客将直线移动到目标位置");

        for (mut agent, mut transform) in passengers.iter_mut() {
            // 设置一个简单的目标位置
            agent.state = AgentState::Traveling;

            // 创建一个简单的路径：当前位置 -> 目标位置（右侧200像素）
            let target_pos = transform.translation + Vec3::new(200.0, 0.0, 0.0);
            info!(
                "设置乘客 {:?} 移动: {:?} -> {:?}",
                agent.color, transform.translation, target_pos
            );
        }
    }

    // 执行简单移动
    let dt = time.delta_secs();
    for (mut agent, mut transform) in passengers.iter_mut() {
        if matches!(agent.state, AgentState::Traveling) {
            // 简单的右移动画
            let speed = 100.0; // 像素/秒
            transform.translation.x += speed * dt;

            // 5秒后停止
            agent.waiting_time += dt;
            if agent.waiting_time > 5.0 {
                agent.state = AgentState::Arrived;
                info!("乘客 {:?} 完成测试移动", agent.color);
            }
        }
    }
}

// 强制乘客移动到指定位置
fn force_passenger_movement(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut passengers: Query<(&mut PathfindingAgent, &mut Transform)>,
) {
    if keyboard_input.just_pressed(KeyCode::F10) {
        info!("强制移动所有乘客到新位置");

        for (mut agent, mut transform) in passengers.iter_mut() {
            // 强制移动到屏幕右侧
            transform.translation.x += 100.0;
            transform.translation.y += 50.0;

            agent.state = AgentState::Traveling;
            info!(
                "强制移动乘客 {:?} 到 {:?}",
                agent.color, transform.translation
            );
        }
    }
}
