// src/bus_puzzle/pathfinding.rs

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    collections::{BinaryHeap, HashMap, HashSet},
};

// 使用相对路径引用同模块下的其他文件
use super::{
    AgentState, Connection, ConnectionType, GameStateEnum, GraphNode, GraphNodeType, GridPos,
    LevelManager, PASSENGER_Z, PathfindingAgent, PathfindingGraph, ROUTE_Z, RouteSegment,
    RouteSegmentType, STATION_Z, StationEntity,
};

// ============ 寻路相关组件 ============

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PathNode {
    pub position: GridPos,
    pub node_type: PathNodeType,
    pub estimated_wait_time: f32,
    pub route_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PathNodeType {
    Station(String),
    RouteSegment,
    TransferPoint,
}

// ============ A* 寻路算法节点 ============

#[derive(Debug, Clone, PartialEq)]
struct AStarNode {
    position: GridPos,
    g_cost: f32, // 从起点到当前节点的实际成本
    h_cost: f32, // 从当前节点到终点的启发式成本
    f_cost: f32, // g_cost + h_cost
    parent: Option<GridPos>,
    route_changes: u32, // 换乘次数
}

impl AStarNode {
    fn new(
        position: GridPos,
        g_cost: f32,
        h_cost: f32,
        parent: Option<GridPos>,
        route_changes: u32,
    ) -> Self {
        Self {
            position,
            g_cost,
            h_cost,
            f_cost: g_cost + h_cost,
            parent,
            route_changes,
        }
    }
}

impl Eq for AStarNode {}

impl PartialOrd for AStarNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for AStarNode {
    fn cmp(&self, other: &Self) -> Ordering {
        // 优先选择 f_cost 较小的节点，如果相等则优先选择换乘次数少的
        match other.f_cost.partial_cmp(&self.f_cost) {
            Some(Ordering::Equal) => other.route_changes.cmp(&self.route_changes),
            Some(ordering) => ordering,
            None => Ordering::Equal,
        }
    }
}

// ============ 寻路系统插件 ============

pub struct PathfindingPlugin;

impl Plugin for PathfindingPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(PathfindingGraph::default())
            .add_systems(
                Update,
                (
                    update_pathfinding_graph_fixed,
                    create_basic_pathfinding_graph, // 创建基础寻路图
                    find_paths_for_new_passengers_fixed,
                    update_passenger_movement_fixed,
                    handle_passenger_transfers,
                    cleanup_finished_passengers,
                    debug_pathfinding_status,
                )
                    .chain()
                    .run_if(in_state(GameStateEnum::Playing)),
            );
    }
}

// ============ 系统实现 ============
// 创建基础寻路图（即使没有路线段也能让乘客移动）
fn create_basic_pathfinding_graph(
    mut pathfinding_graph: ResMut<PathfindingGraph>,
    stations: Query<&StationEntity>,
    route_segments: Query<&RouteSegment>,
    mut graph_created: Local<bool>,
    time: Res<Time>,
) {
    if *graph_created && time.elapsed_secs() as u32 % 10 != 0 {
        return;
    }

    if !*graph_created {
        *graph_created = true;
        info!("创建基础寻路图...");
    }

    pathfinding_graph.nodes.clear();
    pathfinding_graph.connections.clear();
    pathfinding_graph.station_lookup.clear();

    // 添加所有站点
    for station_entity in stations.iter() {
        let station = &station_entity.station_data;
        let pos = station.position;

        pathfinding_graph.nodes.insert(
            pos,
            GraphNode {
                position: pos,
                node_type: GraphNodeType::Station,
                station_name: Some(station.name.clone()),
                is_accessible: true,
            },
        );

        pathfinding_graph
            .station_lookup
            .insert(station.name.clone(), pos);
        info!(
            "添加站点: {} 位置: {:?} (层级: STATION_Z={:.1})",
            station.name, pos, STATION_Z
        );
    }

    // 添加路线段
    for segment in route_segments.iter() {
        if segment.is_active {
            let pos = segment.grid_pos;
            pathfinding_graph.nodes.insert(
                pos,
                GraphNode {
                    position: pos,
                    node_type: GraphNodeType::RouteSegment,
                    station_name: None,
                    is_accessible: true,
                },
            );
            info!(
                "添加路线段: {:?} 位置: {:?} (层级: ROUTE_Z={:.1})",
                segment.segment_type, pos, ROUTE_Z
            );
        }
    }

    // 创建连接
    let station_positions: Vec<GridPos> =
        pathfinding_graph.station_lookup.values().copied().collect();

    if route_segments.is_empty() {
        info!("没有路线段，创建站点间直接连接");
        for &from_pos in &station_positions {
            for &to_pos in &station_positions {
                if from_pos != to_pos {
                    pathfinding_graph
                        .connections
                        .entry(from_pos)
                        .or_insert_with(Vec::new)
                        .push(Connection {
                            to: to_pos,
                            cost: manhattan_distance(from_pos, to_pos) as f32,
                            route_id: Some("direct".to_string()),
                            connection_type: ConnectionType::Walk,
                        });
                }
            }
        }
    }

    info!(
        "寻路图创建完成: {} 个节点, {} 个连接",
        pathfinding_graph.nodes.len(),
        pathfinding_graph.connections.len()
    );
}

// 修复的寻路图更新
fn update_pathfinding_graph_fixed(
    mut pathfinding_graph: ResMut<PathfindingGraph>,
    route_segments: Query<&RouteSegment, Changed<RouteSegment>>,
    stations: Query<&StationEntity, Changed<StationEntity>>,
) {
    if route_segments.is_empty() && stations.is_empty() {
        return;
    }

    info!("检测到路线段或站点变化，更新寻路图");
}

// 修复的乘客寻路
fn find_paths_for_new_passengers_fixed(
    pathfinding_graph: Res<PathfindingGraph>,
    mut passengers: Query<(Entity, &mut PathfindingAgent), Added<PathfindingAgent>>,
) {
    for (entity, mut agent) in passengers.iter_mut() {
        if agent.current_path.is_empty() {
            info!(
                "为新乘客寻找路径: {:?} {} -> {}",
                agent.color, agent.origin, agent.destination
            );

            if let Some(path) =
                find_optimal_path_fixed(&pathfinding_graph, &agent.origin, &agent.destination)
            {
                agent.current_path = path;
                agent.current_step = 0;
                agent.state = AgentState::Traveling;
                agent.waiting_time = 0.0;

                info!(
                    "为乘客 {:?} 找到路径，共 {} 步，开始移动",
                    agent.color,
                    agent.current_path.len()
                );
            } else {
                warn!("无法为乘客 {:?} 找到路径，创建直线路径", agent.color);
                agent.current_path =
                    create_direct_path(&agent.origin, &agent.destination, &pathfinding_graph);
                agent.current_step = 0;
                agent.state = AgentState::Traveling;
            }
        }
    }
}

// 修复的乘客移动
fn update_passenger_movement_fixed(
    time: Res<Time>,
    mut passengers: Query<(&mut PathfindingAgent, &mut Transform)>,
) {
    let dt = time.delta_secs();
    let tile_size = 64.0;

    for (mut agent, mut transform) in passengers.iter_mut() {
        // 确保乘客始终在正确的 Z 层级
        transform.translation.z = PASSENGER_Z;

        match agent.state {
            AgentState::WaitingAtStation => {
                agent.waiting_time += dt;
                if agent.waiting_time > 0.5 && !agent.current_path.is_empty() {
                    agent.state = AgentState::Traveling;
                    agent.waiting_time = 0.0;
                    info!(
                        "乘客 {:?} 开始移动，层级: PASSENGER_Z={:.1}",
                        agent.color, PASSENGER_Z
                    );
                }
            }
            AgentState::Traveling => {
                if agent.current_step < agent.current_path.len() {
                    let current_node = &agent.current_path[agent.current_step];

                    // 计算目标位置，确保在正确的 Z 层
                    let mut target_pos = current_node.position.to_world_pos(tile_size, 10, 8);
                    target_pos.z = PASSENGER_Z;

                    let direction = (target_pos - transform.translation).normalize_or_zero();
                    let speed = 150.0;

                    let distance_to_target = Vec2::new(
                        target_pos.x - transform.translation.x,
                        target_pos.y - transform.translation.y,
                    )
                    .length();

                    if distance_to_target > 10.0 {
                        let movement = Vec3::new(direction.x, direction.y, 0.0) * speed * dt;
                        transform.translation += movement;
                        transform.translation.z = PASSENGER_Z; // 确保 Z 坐标不变

                        if agent.waiting_time.fract() < dt * 2.0 {
                            info!(
                                "乘客 {:?} 移动中: 距离目标 {:.1} 像素, 层级: PASSENGER_Z={:.1}",
                                agent.color, distance_to_target, PASSENGER_Z
                            );
                        }
                    } else {
                        transform.translation = target_pos;
                        transform.translation.z = PASSENGER_Z;
                        agent.current_step += 1;

                        info!(
                            "乘客 {:?} 到达节点 {}/{}, 层级: PASSENGER_Z={:.1}",
                            agent.color,
                            agent.current_step,
                            agent.current_path.len(),
                            PASSENGER_Z
                        );

                        if agent.current_step >= agent.current_path.len() {
                            agent.state = AgentState::Arrived;
                            info!("乘客 {:?} 到达终点！", agent.color);
                        }
                    }
                } else {
                    agent.state = AgentState::Arrived;
                }

                agent.waiting_time += dt;
            }
            AgentState::Transferring => {
                agent.waiting_time += dt;
                if agent.waiting_time > 1.0 {
                    agent.state = AgentState::Traveling;
                    agent.waiting_time = 0.0;
                }
            }
            _ => {}
        }

        agent.patience -= dt * 0.5;
        if agent.patience <= 0.0 && agent.state != AgentState::Arrived {
            agent.state = AgentState::GaveUp;
            warn!("乘客 {:?} 耐心耗尽", agent.color);
        }
    }
}

// 创建直线路径（当找不到正常路径时）
fn create_direct_path(
    origin: &str,
    destination: &str,
    pathfinding_graph: &PathfindingGraph,
) -> Vec<PathNode> {
    let mut path = Vec::new();

    if let (Some(&start_pos), Some(&end_pos)) = (
        pathfinding_graph.station_lookup.get(origin),
        pathfinding_graph.station_lookup.get(destination),
    ) {
        // 起点
        path.push(PathNode {
            position: start_pos,
            node_type: PathNodeType::Station(origin.to_string()),
            estimated_wait_time: 0.0,
            route_id: None,
        });

        // 中间点（如果需要的话）
        if start_pos != end_pos {
            let mid_x = (start_pos.x + end_pos.x) / 2;
            let mid_y = (start_pos.y + end_pos.y) / 2;
            let mid_pos = GridPos::new(mid_x, mid_y);

            path.push(PathNode {
                position: mid_pos,
                node_type: PathNodeType::RouteSegment,
                estimated_wait_time: 1.0,
                route_id: Some("direct".to_string()),
            });
        }

        // 终点
        path.push(PathNode {
            position: end_pos,
            node_type: PathNodeType::Station(destination.to_string()),
            estimated_wait_time: 0.0,
            route_id: None,
        });

        info!(
            "创建直线路径: {} -> {}, {} 步",
            origin,
            destination,
            path.len()
        );
    }

    path
}

// 改进的寻路算法
fn find_optimal_path_fixed(
    graph: &PathfindingGraph,
    origin: &str,
    destination: &str,
) -> Option<Vec<PathNode>> {
    info!("寻找路径: {} -> {}", origin, destination);

    let start_pos = graph.station_lookup.get(origin)?;
    let end_pos = graph.station_lookup.get(destination)?;

    info!("起点位置: {:?}, 终点位置: {:?}", start_pos, end_pos);

    // 如果起点和终点相同
    if start_pos == end_pos {
        return Some(vec![PathNode {
            position: *start_pos,
            node_type: PathNodeType::Station(destination.to_string()),
            estimated_wait_time: 0.0,
            route_id: None,
        }]);
    }

    // 检查是否有连接
    if let Some(connections) = graph.connections.get(start_pos) {
        info!("从起点 {:?} 有 {} 个连接", start_pos, connections.len());
        for conn in connections {
            info!("  连接到: {:?}, 成本: {}", conn.to, conn.cost);
        }

        // 如果有直接连接到终点
        if connections.iter().any(|conn| conn.to == *end_pos) {
            return Some(vec![
                PathNode {
                    position: *start_pos,
                    node_type: PathNodeType::Station(origin.to_string()),
                    estimated_wait_time: 0.0,
                    route_id: None,
                },
                PathNode {
                    position: *end_pos,
                    node_type: PathNodeType::Station(destination.to_string()),
                    estimated_wait_time: 0.0,
                    route_id: None,
                },
            ]);
        }
    } else {
        warn!("起点 {:?} 没有任何连接", start_pos);
    }

    // 使用 A* 算法（简化版本）
    // ... 这里可以添加更复杂的 A* 实现

    None
}

// 调试系统
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

        info!("乘客状态:");
        for agent in passengers.iter() {
            info!(
                "  {:?}: 状态 {:?}, 路径长度 {}, 当前步骤 {}",
                agent.color,
                agent.state,
                agent.current_path.len(),
                agent.current_step
            );
        }
    }
}

// 辅助函数
fn manhattan_distance(pos1: GridPos, pos2: GridPos) -> u32 {
    ((pos1.x - pos2.x).abs() + (pos1.y - pos2.y).abs()) as u32
}

/// 更新寻路图，基于当前的路线段和站点状态
fn update_pathfinding_graph(
    mut pathfinding_graph: ResMut<PathfindingGraph>,
    route_segments: Query<(&RouteSegment, &Transform), Changed<RouteSegment>>,
    stations: Query<&StationEntity, Changed<StationEntity>>,
) {
    // 清除旧的连接
    pathfinding_graph.connections.clear();
    pathfinding_graph.nodes.clear();
    pathfinding_graph.station_lookup.clear();

    // 添加站点节点
    for station_entity in stations.iter() {
        let station = &station_entity.station_data;
        let pos = station.position;

        pathfinding_graph.nodes.insert(
            pos,
            GraphNode {
                position: pos,
                node_type: GraphNodeType::Station,
                station_name: Some(station.name.clone()),
                is_accessible: true,
            },
        );

        pathfinding_graph
            .station_lookup
            .insert(station.name.clone(), pos);
    }

    // 添加路线段节点
    let mut route_segments_by_pos = HashMap::new();
    for (segment, _transform) in route_segments.iter() {
        if segment.is_active {
            let pos = segment.grid_pos;
            route_segments_by_pos.insert(pos, segment);

            pathfinding_graph.nodes.insert(
                pos,
                GraphNode {
                    position: pos,
                    node_type: match segment.segment_type {
                        RouteSegmentType::Cross | RouteSegmentType::TSplit => {
                            GraphNodeType::Intersection
                        }
                        _ => GraphNodeType::RouteSegment,
                    },
                    station_name: None,
                    is_accessible: true,
                },
            );
        }
    }

    // 建立路线段之间的连接
    let mut new_connections = Vec::new(); // 先收集所有连接，然后统一添加

    for (pos, segment) in &route_segments_by_pos {
        let connections = get_segment_connections(*pos, &segment.segment_type, segment.rotation);

        for connection_pos in connections {
            if route_segments_by_pos.contains_key(&connection_pos)
                || pathfinding_graph
                    .station_lookup
                    .values()
                    .any(|&station_pos| station_pos == connection_pos)
            {
                new_connections.push((
                    *pos,
                    Connection {
                        to: connection_pos,
                        cost: 1.0,
                        route_id: Some(format!("route_{}", pos.x + pos.y)), // 简化的路线ID
                        connection_type: ConnectionType::BusRoute,
                    },
                ));
            }
        }
    }

    // 统一添加所有连接，避免借用冲突
    for (from_pos, connection) in new_connections {
        pathfinding_graph
            .connections
            .entry(from_pos)
            .or_insert_with(Vec::new)
            .push(connection);
    }

    // 建立站点与相邻路线段的连接
    let mut station_connections = Vec::new(); // 先收集站点连接

    for (_station_name, &station_pos) in &pathfinding_graph.station_lookup {
        let adjacent_positions = get_adjacent_positions(station_pos);

        for adj_pos in adjacent_positions {
            if route_segments_by_pos.contains_key(&adj_pos) {
                // 站点到路线段
                station_connections.push((
                    station_pos,
                    Connection {
                        to: adj_pos,
                        cost: 0.5,
                        route_id: None,
                        connection_type: ConnectionType::Walk,
                    },
                ));

                // 路线段到站点
                station_connections.push((
                    adj_pos,
                    Connection {
                        to: station_pos,
                        cost: 0.5,
                        route_id: None,
                        connection_type: ConnectionType::Walk,
                    },
                ));
            }
        }
    }

    // 统一添加所有站点连接
    for (from_pos, connection) in station_connections {
        pathfinding_graph
            .connections
            .entry(from_pos)
            .or_insert_with(Vec::new)
            .push(connection);
    }
}

/// 为新生成的乘客寻找路径
fn find_paths_for_new_passengers(
    pathfinding_graph: Res<PathfindingGraph>,
    mut passengers: Query<
        &mut PathfindingAgent,
        (Added<PathfindingAgent>, Without<super::PassengerEntity>),
    >,
) {
    for mut agent in passengers.iter_mut() {
        if let Some(path) = find_optimal_path(&pathfinding_graph, &agent.origin, &agent.destination)
        {
            agent.current_path = path;
            agent.current_step = 0;
            agent.state = AgentState::WaitingAtStation;

            info!(
                "Found path for passenger {:?}, {} steps",
                agent.color,
                agent.current_path.len()
            );
        } else {
            warn!(
                "No path found for passenger {:?} from {} to {}",
                agent.color, agent.origin, agent.destination
            );
            agent.state = AgentState::GaveUp;
        }
    }
}

/// 更新乘客移动
fn update_passenger_movement(
    time: Res<Time>,
    mut passengers: Query<(&mut PathfindingAgent, &mut Transform)>,
    level_manager: Res<LevelManager>,
) {
    let dt = time.delta_secs();
    let tile_size = level_manager.tile_size;

    // 获取网格尺寸信息
    let (grid_width, grid_height) = if let Some(level_data) = &level_manager.current_level {
        level_data.grid_size
    } else {
        return; // 如果没有关卡数据，直接返回
    };

    for (mut agent, mut transform) in passengers.iter_mut() {
        match agent.state {
            AgentState::WaitingAtStation | AgentState::Transferring => {
                agent.waiting_time += dt;
                agent.patience -= dt;

                // 检查是否可以开始下一段行程
                if agent.waiting_time > 2.0
                    && agent.current_step < agent.current_path.len().saturating_sub(1)
                {
                    agent.current_step += 1;
                    agent.state = AgentState::Traveling;
                    agent.waiting_time = 0.0;
                }
            }
            AgentState::Traveling => {
                if agent.current_step < agent.current_path.len() {
                    let current_node = &agent.current_path[agent.current_step];
                    let target_pos =
                        current_node
                            .position
                            .to_world_pos(tile_size, grid_width, grid_height);

                    // 简单的移动插值
                    let direction = (target_pos - transform.translation).normalize();
                    let speed = 100.0; // 像素/秒

                    if transform.translation.distance(target_pos) > 5.0 {
                        transform.translation += direction * speed * dt;
                    } else {
                        // 到达当前节点
                        transform.translation = target_pos;

                        match &current_node.node_type {
                            PathNodeType::Station(station_name) => {
                                if station_name == &agent.destination {
                                    agent.state = AgentState::Arrived;
                                } else {
                                    agent.state = AgentState::Transferring;
                                }
                            }
                            PathNodeType::TransferPoint => {
                                agent.state = AgentState::Transferring;
                            }
                            _ => {
                                // 继续移动到下一个节点
                                agent.current_step += 1;
                                if agent.current_step >= agent.current_path.len() {
                                    agent.state = AgentState::Arrived;
                                }
                            }
                        }
                    }
                }
            }
            _ => {} // Arrived 或 GaveUp 状态不需要更新
        }

        // 检查耐心值
        if agent.patience <= 0.0 && agent.state != AgentState::Arrived {
            agent.state = AgentState::GaveUp;
        }
    }
}

/// 处理乘客换乘逻辑
fn handle_passenger_transfers(mut passengers: Query<&mut PathfindingAgent>) {
    for mut agent in passengers.iter_mut() {
        if agent.state == AgentState::Transferring && agent.waiting_time > 1.0 {
            // 模拟换乘等待时间
            if agent.current_step < agent.current_path.len().saturating_sub(1) {
                agent.current_step += 1;
                agent.state = AgentState::Traveling;
                agent.waiting_time = 0.0;
            }
        }
    }
}

/// 清理已完成的乘客
fn cleanup_finished_passengers(
    mut commands: Commands,
    passengers: Query<(Entity, &PathfindingAgent)>,
) {
    for (entity, agent) in passengers.iter() {
        match agent.state {
            AgentState::Arrived => {
                info!("Passenger {:?} arrived at destination", agent.color);
                commands.entity(entity).despawn();
            }
            AgentState::GaveUp => {
                warn!("Passenger {:?} gave up due to timeout", agent.color);
                commands.entity(entity).despawn();
            }
            _ => {}
        }
    }
}

// ============ 寻路算法实现 ============

/// A* 寻路算法，寻找最优路径
fn find_optimal_path(
    graph: &PathfindingGraph,
    origin: &str,
    destination: &str,
) -> Option<Vec<PathNode>> {
    let start_pos = *graph.station_lookup.get(origin)?;
    let end_pos = *graph.station_lookup.get(destination)?;

    let mut open_set = BinaryHeap::new();
    let mut closed_set = HashSet::new();
    let mut came_from = HashMap::new();

    // 初始化起点
    let start_node = AStarNode::new(start_pos, 0.0, heuristic(start_pos, end_pos), None, 0);
    open_set.push(start_node);

    while let Some(current) = open_set.pop() {
        if current.position == end_pos {
            // 找到目标，重构路径
            return Some(reconstruct_path(came_from, current.position, graph));
        }

        closed_set.insert(current.position);

        // 检查所有邻居
        if let Some(connections) = graph.connections.get(&current.position) {
            for connection in connections {
                if closed_set.contains(&connection.to) {
                    continue;
                }

                let route_changes = if connection.connection_type == ConnectionType::Transfer {
                    current.route_changes + 1
                } else {
                    current.route_changes
                };

                let tentative_g_cost =
                    current.g_cost + connection.cost + (route_changes as f32 * 5.0); // 换乘惩罚

                let neighbor = AStarNode::new(
                    connection.to,
                    tentative_g_cost,
                    heuristic(connection.to, end_pos),
                    Some(current.position),
                    route_changes,
                );

                // 检查是否找到更好的路径
                let should_add = open_set
                    .iter()
                    .find(|node| node.position == connection.to)
                    .map_or(true, |existing| neighbor.f_cost < existing.f_cost);

                if should_add {
                    came_from.insert(connection.to, current.position);
                    open_set.push(neighbor);
                }
            }
        }
    }

    None // 未找到路径
}

/// 启发式函数：曼哈顿距离
fn heuristic(pos1: GridPos, pos2: GridPos) -> f32 {
    ((pos1.x - pos2.x).abs() + (pos1.y - pos2.y).abs()) as f32
}

/// 重构路径
fn reconstruct_path(
    came_from: HashMap<GridPos, GridPos>,
    mut current: GridPos,
    graph: &PathfindingGraph,
) -> Vec<PathNode> {
    let mut path = Vec::new();

    while let Some(&parent) = came_from.get(&current) {
        if let Some(node) = graph.nodes.get(&current) {
            let node_type = match &node.node_type {
                GraphNodeType::Station => {
                    PathNodeType::Station(node.station_name.clone().unwrap_or_default())
                }
                GraphNodeType::Intersection => PathNodeType::TransferPoint,
                GraphNodeType::RouteSegment => PathNodeType::RouteSegment,
            };

            path.push(PathNode {
                position: current,
                node_type,
                estimated_wait_time: 2.0, // 简化的等待时间
                route_id: None,
            });
        }
        current = parent;
    }

    // 添加起点
    if let Some(node) = graph.nodes.get(&current) {
        path.push(PathNode {
            position: current,
            node_type: PathNodeType::Station(node.station_name.clone().unwrap_or_default()),
            estimated_wait_time: 0.0,
            route_id: None,
        });
    }

    path.reverse();
    path
}

// ============ 公共辅助函数 ============

/// 根据路线段类型和旋转角度获取连接位置
pub fn get_segment_connections(
    pos: GridPos,
    segment_type: &RouteSegmentType,
    rotation: u32,
) -> Vec<GridPos> {
    let base_connections = match segment_type {
        RouteSegmentType::Straight => vec![(0, -1), (0, 1)], // 上下连接
        RouteSegmentType::Curve => vec![(0, -1), (1, 0)],    // L型
        RouteSegmentType::TSplit => vec![(0, -1), (0, 1), (1, 0)], // T型
        RouteSegmentType::Cross => vec![(0, -1), (0, 1), (-1, 0), (1, 0)], // 十字
        RouteSegmentType::Bridge | RouteSegmentType::Tunnel => vec![(0, -1), (0, 1)], // 直线
    };

    base_connections
        .into_iter()
        .map(|(dx, dy)| {
            // 根据旋转角度调整连接方向
            let (new_dx, new_dy) = rotate_offset(dx, dy, rotation);
            GridPos::new(pos.x + new_dx, pos.y + new_dy)
        })
        .collect()
}

/// 重建寻路图（从其他模块调用）
pub fn rebuild_pathfinding_graph(
    pathfinding_graph: &mut PathfindingGraph,
    game_state: &super::GameState,
) {
    // 清除旧数据
    pathfinding_graph.connections.clear();
    pathfinding_graph.nodes.clear();
    pathfinding_graph.station_lookup.clear();

    // 从游戏状态重建图结构
    if let Some(level_data) = &game_state.current_level {
        // 添加站点
        for station_data in &level_data.stations {
            let pos = station_data.position;
            let station_name = station_data.name.clone();

            pathfinding_graph.nodes.insert(
                pos,
                GraphNode {
                    position: pos,
                    node_type: GraphNodeType::Station,
                    station_name: Some(station_name.clone()),
                    is_accessible: true,
                },
            );

            pathfinding_graph
                .station_lookup
                .insert(station_name.clone(), pos);
        }

        // 添加已放置的路线段
        for (pos, placed_segment) in &game_state.placed_segments {
            pathfinding_graph.nodes.insert(
                *pos,
                GraphNode {
                    position: *pos,
                    node_type: match placed_segment.segment_type {
                        RouteSegmentType::Cross | RouteSegmentType::TSplit => {
                            GraphNodeType::Intersection
                        }
                        _ => GraphNodeType::RouteSegment,
                    },
                    station_name: None,
                    is_accessible: true,
                },
            );
        }

        // 重建连接
        rebuild_connections(pathfinding_graph, game_state);
    }
}

// ============ 私有辅助函数 ============

/// 旋转偏移坐标
fn rotate_offset(dx: i32, dy: i32, rotation: u32) -> (i32, i32) {
    match rotation % 360 {
        0 => (dx, dy),
        90 => (-dy, dx),
        180 => (-dx, -dy),
        270 => (dy, -dx),
        _ => (dx, dy),
    }
}

/// 获取相邻位置（四个方向）
fn get_adjacent_positions(pos: GridPos) -> Vec<GridPos> {
    vec![
        GridPos::new(pos.x, pos.y - 1), // 上
        GridPos::new(pos.x, pos.y + 1), // 下
        GridPos::new(pos.x - 1, pos.y), // 左
        GridPos::new(pos.x + 1, pos.y), // 右
    ]
}

/// 重建连接关系
fn rebuild_connections(pathfinding_graph: &mut PathfindingGraph, game_state: &super::GameState) {
    // 建立路线段之间的连接
    let mut new_connections = Vec::new();

    for (pos, placed_segment) in &game_state.placed_segments {
        let connections =
            get_segment_connections(*pos, &placed_segment.segment_type, placed_segment.rotation);

        for connection_pos in connections {
            if game_state.placed_segments.contains_key(&connection_pos)
                || pathfinding_graph
                    .station_lookup
                    .values()
                    .any(|&station_pos| station_pos == connection_pos)
            {
                new_connections.push((
                    *pos,
                    Connection {
                        to: connection_pos,
                        cost: 1.0,
                        route_id: Some(format!("route_{}", pos.x + pos.y)),
                        connection_type: ConnectionType::BusRoute,
                    },
                ));
            }
        }
    }

    // 添加路线段连接
    for (from_pos, connection) in new_connections {
        pathfinding_graph
            .connections
            .entry(from_pos)
            .or_insert_with(Vec::new)
            .push(connection);
    }

    // 建立站点与路线段的连接
    let mut station_connections = Vec::new();

    for (_station_name, &station_pos) in &pathfinding_graph.station_lookup {
        let adjacent_positions = get_adjacent_positions(station_pos);

        for adj_pos in adjacent_positions {
            if game_state.placed_segments.contains_key(&adj_pos) {
                // 站点到路线段
                station_connections.push((
                    station_pos,
                    Connection {
                        to: adj_pos,
                        cost: 0.5,
                        route_id: None,
                        connection_type: ConnectionType::Walk,
                    },
                ));

                // 路线段到站点
                station_connections.push((
                    adj_pos,
                    Connection {
                        to: station_pos,
                        cost: 0.5,
                        route_id: None,
                        connection_type: ConnectionType::Walk,
                    },
                ));
            }
        }
    }

    // 添加站点连接
    for (from_pos, connection) in station_connections {
        pathfinding_graph
            .connections
            .entry(from_pos)
            .or_insert_with(Vec::new)
            .push(connection);
    }
}
