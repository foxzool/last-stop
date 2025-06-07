// src/bus_puzzle/junction_movement.rs - 路口内部移动逻辑

use bevy::prelude::*;
use crate::bus_puzzle::{
    GridPos, RouteSegmentType, GameStateEnum, PathfindingAgent, AgentState,
    PASSENGER_Z, LevelManager, RouteSegment, PathNode, PathNodeType
};

pub struct JunctionMovementPlugin;

impl Plugin for JunctionMovementPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (
            handle_junction_movement,
            debug_junction_movement,
        ).run_if(in_state(GameStateEnum::Playing)));
    }
}

/// 处理乘客在路口的移动
fn handle_junction_movement(
    time: Res<Time>,
    mut passengers: Query<(&mut PathfindingAgent, &mut Transform)>,
    route_segments: Query<&RouteSegment>,
    level_manager: Res<LevelManager>,
) {
    let dt = time.delta_secs();
    let tile_size = level_manager.tile_size;
    
    let (grid_width, grid_height) = if let Some(level_data) = &level_manager.current_level {
        level_data.grid_size
    } else {
        return;
    };

    for (mut agent, mut transform) in passengers.iter_mut() {
        if !matches!(agent.state, AgentState::Traveling) {
            continue;
        }

        if agent.current_step >= agent.current_path.len() {
            continue;
        }

        let current_node = &agent.current_path[agent.current_step];
        
        // 检查当前目标是否是路口
        if let Some(junction_segment) = find_junction_at_position(current_node.position, &route_segments) {
            handle_junction_traversal(
                &mut agent,
                &mut transform,
                &junction_segment,
                dt,
                tile_size,
                grid_width,
                grid_height,
            );
        }
    }
}

/// 处理路口穿越逻辑
fn handle_junction_traversal(
    agent: &mut PathfindingAgent,
    transform: &mut Transform,
    junction: &RouteSegment,
    dt: f32,
    tile_size: f32,
    grid_width: u32,
    grid_height: u32,
) {
    let junction_center = junction.grid_pos.to_world_pos(tile_size, grid_width, grid_height);
    let current_pos = transform.translation;
    
    // 获取乘客状态：是否已经在路口内部
    let distance_to_center = current_pos.distance(junction_center);
    let junction_radius = tile_size * 0.3; // 路口内部区域半径
    
    if distance_to_center > junction_radius {
        // 乘客还在路口外围，先移动到中心
        move_to_junction_center(agent, transform, junction_center, dt);
    } else {
        // 乘客已在路口内部，可以移动到下一个目标
        move_through_junction(agent, transform, dt, tile_size, grid_width, grid_height);
    }
}

/// 移动到路口中心
fn move_to_junction_center(
    agent: &mut PathfindingAgent,
    transform: &mut Transform,
    junction_center: Vec3,
    dt: f32,
) {
    let direction = (junction_center - transform.translation).normalize_or_zero();
    let speed = 80.0; // 在路口内移动速度稍慢
    
    let movement = Vec3::new(direction.x, direction.y, 0.0) * speed * dt;
    transform.translation += movement;
    transform.translation.z = PASSENGER_Z;
    
    // 检查是否到达中心
    let distance_to_center = transform.translation.distance(junction_center);
    if distance_to_center < 5.0 {
        // 到达中心，设置为路口中心位置
        transform.translation = junction_center + Vec3::Z * PASSENGER_Z;
        info!("乘客 {:?} 到达路口中心", agent.color);
    }
}

/// 从路口中心移动到下一个目标
fn move_through_junction(
    agent: &mut PathfindingAgent,
    transform: &mut Transform,
    dt: f32,
    tile_size: f32,
    grid_width: u32,
    grid_height: u32,
) {
    // 检查是否有下一个节点
    let next_step = agent.current_step + 1;
    if next_step >= agent.current_path.len() {
        agent.state = AgentState::Arrived;
        return;
    }
    
    let next_node = &agent.current_path[next_step];
    let next_target = next_node.position.to_world_pos(tile_size, grid_width, grid_height);
    
    let direction = (next_target - transform.translation).normalize_or_zero();
    let speed = 120.0; // 正常移动速度
    
    let movement = Vec3::new(direction.x, direction.y, 0.0) * speed * dt;
    transform.translation += movement;
    transform.translation.z = PASSENGER_Z;
    
    // 检查是否离开了路口区域
    let distance_to_target = transform.translation.distance(next_target);
    if distance_to_target < 8.0 {
        // 成功穿越路口，移动到下一个节点
        agent.current_step = next_step;
        transform.translation = next_target + Vec3::Z * PASSENGER_Z;
        info!("乘客 {:?} 穿越路口到达 {:?}", agent.color, next_node.position);
    }
}

/// 查找指定位置的路口
fn find_junction_at_position(pos: GridPos, route_segments: &Query<&RouteSegment>) -> Option<RouteSegment> {
    for segment in route_segments.iter() {
        if segment.grid_pos == pos && segment.is_active {
            match segment.segment_type {
                RouteSegmentType::Curve | RouteSegmentType::TSplit | RouteSegmentType::Cross => {
                    return Some(*segment);
                }
                _ => {}
            }
        }
    }
    None
}

/// 调试路口移动
fn debug_junction_movement(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    passengers: Query<(&PathfindingAgent, &Transform)>,
    route_segments: Query<&RouteSegment>,
) {
    if keyboard_input.just_pressed(KeyCode::F11) {
        info!("🚦 路口移动调试");
        
        for (agent, transform) in passengers.iter() {
            if matches!(agent.state, AgentState::Traveling) && agent.current_step < agent.current_path.len() {
                let current_node = &agent.current_path[agent.current_step];
                
                if let Some(junction) = find_junction_at_position(current_node.position, &route_segments) {
                    info!("乘客 {:?} 正在穿越 {:?} 路口", agent.color, junction.segment_type);
                    info!("  当前位置: {:?}", transform.translation);
                    info!("  目标位置: {:?}", current_node.position);
                    info!("  路径步骤: {}/{}", agent.current_step, agent.current_path.len());
                }
            }
        }
    }
}

/// 改进路径，在路口前后插入中间节点
pub fn enhance_path_with_junction_nodes(
    original_path: Vec<PathNode>,
    route_segments: &Query<&RouteSegment>,
) -> Vec<PathNode> {
    let mut enhanced_path = Vec::new();
    
    for (i, node) in original_path.iter().enumerate() {
        // 添加原始节点
        enhanced_path.push(node.clone());
        
        // 检查下一个节点是否是路口
        if i + 1 < original_path.len() {
            let next_node = &original_path[i + 1];
            
            if let Some(junction) = find_junction_at_position(next_node.position, route_segments) {
                // 在路口前插入预备节点
                enhanced_path.push(PathNode {
                    position: junction.grid_pos,
                    node_type: PathNodeType::TransferPoint,
                    estimated_wait_time: 0.2,
                    route_id: Some(format!("junction_approach_{}", junction.grid_pos.x)),
                });
                
                // 插入路口中心节点
                enhanced_path.push(PathNode {
                    position: junction.grid_pos,
                    node_type: PathNodeType::TransferPoint,
                    estimated_wait_time: 0.3,
                    route_id: Some(format!("junction_center_{}", junction.grid_pos.x)),
                });
                
                info!("为 {:?} 路口添加内部节点", junction.segment_type);
            }
        }
    }
    
    enhanced_path
}

/// 检查路径是否需要路口增强
pub fn path_needs_junction_enhancement(path: &[PathNode], route_segments: &Query<&RouteSegment>) -> bool {
    for node in path {
        if find_junction_at_position(node.position, route_segments).is_some() {
            return true;
        }
    }
    false
}
