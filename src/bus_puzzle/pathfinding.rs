// src/bus_puzzle/pathfinding.rs - 正式版寻路系统

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    collections::{BinaryHeap, HashMap, HashSet},
};

use super::{
    AgentState, Connection, ConnectionType, GameStateEnum, GraphNode, GraphNodeType, GridPos,
    LevelManager, PASSENGER_Z, PathfindingAgent, PathfindingGraph, RouteSegment, RouteSegmentType,
    StationEntity,
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
    g_cost: f32,
    h_cost: f32,
    f_cost: f32,
    parent: Option<GridPos>,
    route_changes: u32,
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
                    update_pathfinding_graph,
                    find_paths_for_new_passengers,
                    update_passenger_movement,
                    handle_passenger_transfers,
                    cleanup_finished_passengers,
                )
                    .chain()
                    .run_if(in_state(GameStateEnum::Playing)),
            );
    }
}

// ============ 核心寻路系统 ============

fn update_pathfinding_graph(
    mut pathfinding_graph: ResMut<PathfindingGraph>,
    route_segments: Query<&RouteSegment>,
    stations: Query<&StationEntity>,
) {
    // 简化：每次都重建整个图
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
    for segment in route_segments.iter() {
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

    // 建立连接关系
    create_route_connections_improved(&mut pathfinding_graph, &route_segments_by_pos);
    create_station_connections_improved(&mut pathfinding_graph, &route_segments_by_pos);

    // 只在图结构发生变化时输出日志
    static mut LAST_NODE_COUNT: usize = 0;
    static mut LAST_CONNECTION_COUNT: usize = 0;

    unsafe {
        let current_nodes = pathfinding_graph.nodes.len();
        let current_connections = pathfinding_graph.connections.len();

        if current_nodes != LAST_NODE_COUNT || current_connections != LAST_CONNECTION_COUNT {
            info!(
                "寻路图更新: {} 个节点, {} 个连接",
                current_nodes, current_connections
            );

            for (name, pos) in &pathfinding_graph.station_lookup {
                info!("  站点: {} 位置: {:?}", name, pos);
            }

            // 显示连接详情
            for (from_pos, connections) in &pathfinding_graph.connections {
                if !connections.is_empty() {
                    info!(
                        "  {:?} 连接到: {:?}",
                        from_pos,
                        connections.iter().map(|c| c.to).collect::<Vec<_>>()
                    );
                }
            }

            LAST_NODE_COUNT = current_nodes;
            LAST_CONNECTION_COUNT = current_connections;
        }
    }
}

fn create_route_connections_improved(
    pathfinding_graph: &mut PathfindingGraph,
    route_segments_by_pos: &HashMap<GridPos, &RouteSegment>,
) {
    info!(
        "开始创建路线段连接，共 {} 个路线段",
        route_segments_by_pos.len()
    );

    for (pos, segment) in route_segments_by_pos {
        let theoretical_connections =
            get_segment_connections(*pos, &segment.segment_type, segment.rotation);
        info!(
            "路线段 {:?} at {:?} 旋转{}° 理论连接: {:?}",
            segment.segment_type, pos, segment.rotation, theoretical_connections
        );

        for connection_pos in theoretical_connections {
            if route_segments_by_pos.contains_key(&connection_pos)
                || pathfinding_graph
                    .station_lookup
                    .values()
                    .any(|&station_pos| station_pos == connection_pos)
            {
                // 创建双向连接
                add_connection_if_not_exists(
                    pathfinding_graph,
                    *pos,
                    connection_pos,
                    ConnectionType::BusRoute,
                );

                add_connection_if_not_exists(
                    pathfinding_graph,
                    connection_pos,
                    *pos,
                    ConnectionType::BusRoute,
                );

                info!("理论连接: {:?} <-> {:?}", pos, connection_pos);
            }
        }

        // 重要：强制添加所有相邻路线段的连接
        for (other_pos, other_segment) in route_segments_by_pos {
            if *other_pos != *pos {
                let distance = manhattan_distance_internal(*pos, *other_pos);
                if distance == 1 {
                    // 检查是否在同一行（水平连接）
                    if pos.y == other_pos.y {
                        add_connection_if_not_exists(
                            pathfinding_graph,
                            *pos,
                            *other_pos,
                            ConnectionType::BusRoute,
                        );
                        add_connection_if_not_exists(
                            pathfinding_graph,
                            *other_pos,
                            *pos,
                            ConnectionType::BusRoute,
                        );
                        info!("强制水平连接: {:?} <-> {:?}", pos, other_pos);
                    }
                    // 检查是否在同一列（垂直连接）
                    else if pos.x == other_pos.x {
                        add_connection_if_not_exists(
                            pathfinding_graph,
                            *pos,
                            *other_pos,
                            ConnectionType::BusRoute,
                        );
                        add_connection_if_not_exists(
                            pathfinding_graph,
                            *other_pos,
                            *pos,
                            ConnectionType::BusRoute,
                        );
                        info!("强制垂直连接: {:?} <-> {:?}", pos, other_pos);
                    }
                }
            }
        }
    }
}

// 避免重复连接的辅助函数
fn add_connection_if_not_exists(
    pathfinding_graph: &mut PathfindingGraph,
    from: GridPos,
    to: GridPos,
    connection_type: ConnectionType,
) {
    let connections = pathfinding_graph
        .connections
        .entry(from)
        .or_insert_with(Vec::new);

    // 检查是否已经存在这个连接
    if !connections.iter().any(|conn| conn.to == to) {
        connections.push(Connection {
            to,
            cost: 1.0,
            route_id: Some(format!("route_{}", from.x + from.y)),
            connection_type,
        });
    }
}

pub fn create_station_connections_improved(
    pathfinding_graph: &mut PathfindingGraph,
    route_segments_by_pos: &HashMap<GridPos, &RouteSegment>,
) {
    for (_station_name, &station_pos) in &pathfinding_graph.station_lookup {
        // 扩大搜索范围 - 检查站点周围2格内的所有路线段
        for (segment_pos, segment) in route_segments_by_pos {
            let distance = manhattan_distance_internal(station_pos, *segment_pos);

            // 允许更远的连接距离
            if distance <= 2 {
                // 检查更灵活的连接条件
                if can_connect_station_to_segment(station_pos, *segment_pos, segment) {
                    // 站点到路线段
                    pathfinding_graph
                        .connections
                        .entry(station_pos)
                        .or_insert_with(Vec::new)
                        .push(Connection {
                            to: *segment_pos,
                            cost: 0.5,
                            route_id: None,
                            connection_type: ConnectionType::Walk,
                        });

                    // 路线段到站点
                    pathfinding_graph
                        .connections
                        .entry(*segment_pos)
                        .or_insert_with(Vec::new)
                        .push(Connection {
                            to: station_pos,
                            cost: 0.5,
                            route_id: None,
                            connection_type: ConnectionType::Walk,
                        });

                    trace!(
                        "建立连接: 站点{:?} <-> 路线段{:?} (距离:{})",
                        station_pos, segment_pos, distance
                    );
                }
            }
        }
    }
}

/// 检查站点是否可以连接到路线段
fn can_connect_station_to_segment(
    station_pos: GridPos,
    segment_pos: GridPos,
    segment: &RouteSegment,
) -> bool {
    let distance = manhattan_distance_internal(station_pos, segment_pos);

    // 直接相邻
    if distance == 1 {
        return true;
    }

    // 检查是否在路线段的连接点附近
    let connection_points =
        get_segment_connections(segment_pos, &segment.segment_type, segment.rotation);
    for connection_point in connection_points {
        if manhattan_distance_internal(station_pos, connection_point) <= 1 {
            return true;
        }
    }

    // 对角连接也允许（距离为2但在对角线上）
    if distance == 2 {
        let dx = (station_pos.x - segment_pos.x).abs();
        let dy = (station_pos.y - segment_pos.y).abs();
        if dx == 1 && dy == 1 {
            return true; // 对角连接
        }
    }

    false
}

fn manhattan_distance_internal(pos1: GridPos, pos2: GridPos) -> u32 {
    ((pos1.x - pos2.x).abs() + (pos1.y - pos2.y).abs()) as u32
}

fn find_paths_for_new_passengers(
    pathfinding_graph: Res<PathfindingGraph>,
    mut passengers: Query<&mut PathfindingAgent, Added<PathfindingAgent>>,
    route_segments: Query<&RouteSegment>,
) {
    for mut agent in passengers.iter_mut() {
        // 检查寻路图是否有必要的站点
        if !pathfinding_graph.station_lookup.contains_key(&agent.origin) {
            warn!("找不到起点站: {}", agent.origin);
            agent.state = AgentState::GaveUp;
            continue;
        }

        if !pathfinding_graph
            .station_lookup
            .contains_key(&agent.destination)
        {
            warn!("找不到终点站: {}", agent.destination);
            agent.state = AgentState::GaveUp;
            continue;
        }

        if let Some(basic_path) = find_optimal_path(&pathfinding_graph, &agent.origin, &agent.destination) {
            // 检查路径是否经过路口，如果是则增强路径
            let enhanced_path = if path_needs_junction_enhancement(&basic_path, &route_segments) {
                let enhanced = enhance_path_with_junction_nodes(basic_path.clone(), &route_segments);
                info!("为乘客 {:?} 增强路径，从 {} 步增加到 {} 步",
                    agent.color, enhanced.len() - (enhanced.len() - basic_path.len()), enhanced.len());
                enhanced
            } else {
                basic_path
            };

            agent.current_path = enhanced_path;
            agent.current_step = 0;
            agent.state = AgentState::WaitingAtStation;
            agent.waiting_time = 0.0;

            info!(
                "为乘客 {:?} 找到路径，共 {} 步",
                agent.color,
                agent.current_path.len()
            );
        } else {
            info!("暂时无法为乘客 {:?} 找到路径，设置为等待状态", agent.color);
            agent.state = AgentState::WaitingAtStation;
            agent.waiting_time = 0.0;
        }
    }
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

/// 为路径添加路口内部节点
pub fn enhance_path_with_junction_nodes(
    original_path: Vec<PathNode>,
    route_segments: &Query<&RouteSegment>,
) -> Vec<PathNode> {
    let mut enhanced_path = Vec::new();

    for (i, node) in original_path.iter().enumerate() {
        // 添加原始节点
        enhanced_path.push(node.clone());

        // 检查当前节点是否是路口
        if let Some(junction) = find_junction_at_position(node.position, route_segments) {
            // 在路口前插入进入节点
            enhanced_path.push(PathNode {
                position: junction.grid_pos,
                node_type: PathNodeType::TransferPoint,
                estimated_wait_time: 0.2,
                route_id: Some(format!("junction_enter_{}", junction.grid_pos.x)),
            });

            // 插入路口中心节点
            enhanced_path.push(PathNode {
                position: junction.grid_pos,
                node_type: PathNodeType::TransferPoint,
                estimated_wait_time: 0.3,
                route_id: Some(format!("junction_center_{}", junction.grid_pos.x)),
            });

            // 插入路口退出节点
            enhanced_path.push(PathNode {
                position: junction.grid_pos,
                node_type: PathNodeType::TransferPoint,
                estimated_wait_time: 0.1,
                route_id: Some(format!("junction_exit_{}", junction.grid_pos.x)),
            });

            info!("为 {:?} 路口添加内部节点", junction.segment_type);
        }
    }

    enhanced_path
}

/// 查找指定位置的路口
fn find_junction_at_position(pos: GridPos, route_segments: &Query<&RouteSegment>) -> Option<RouteSegment> {
    for segment in route_segments.iter() {
        if segment.grid_pos == pos && segment.is_active {
            match segment.segment_type {
                RouteSegmentType::Curve | RouteSegmentType::TSplit | RouteSegmentType::Cross => {
                    return Some(segment.clone());
                }
                _ => {}
            }
        }
    }
    None
}

fn update_passenger_movement(
    time: Res<Time>,
    mut passengers: Query<(&mut PathfindingAgent, &mut Transform)>,
    level_manager: Res<LevelManager>,
    pathfinding_graph: Res<PathfindingGraph>,
) {
    let dt = time.delta_secs();
    let tile_size = level_manager.tile_size;

    let (grid_width, grid_height) = if let Some(level_data) = &level_manager.current_level {
        level_data.grid_size
    } else {
        return;
    };

    for (mut agent, mut transform) in passengers.iter_mut() {
        // 确保乘客在正确的Z层级
        transform.translation.z = PASSENGER_Z;

        match agent.state {
            AgentState::WaitingAtStation => {
                agent.waiting_time += dt;
                // 减缓耐心消耗速度，避免过快放弃
                agent.patience -= dt * 0.1; // 原来是dt，现在减慢10倍

                // 如果有路径，等待一段时间后开始移动
                if !agent.current_path.is_empty() && agent.waiting_time > 1.0 {
                    agent.state = AgentState::Traveling;
                    agent.waiting_time = 0.0;
                    info!("乘客 {:?} 开始移动", agent.color);
                } else if agent.current_path.is_empty() {
                    // 尝试重新寻路（也许玩家建设了新的路线）
                    if agent.waiting_time > 3.0 {
                        // 每3秒尝试一次重新寻路
                        if let Some(path) =
                            find_optimal_path(&pathfinding_graph, &agent.origin, &agent.destination)
                        {
                            agent.current_path = path;
                            agent.current_step = 0;
                            agent.waiting_time = 0.0;
                            info!("乘客 {:?} 找到新路径，准备出发", agent.color);
                        } else {
                            agent.waiting_time = 0.0; // 重置等待时间，继续等待
                        }
                    }
                }
            }
            AgentState::Transferring => {
                agent.waiting_time += dt;
                agent.patience -= dt * 0.2; // 换乘时耐心消耗稍快

                if agent.waiting_time > 0.8 {
                    if agent.current_step < agent.current_path.len().saturating_sub(1) {
                        agent.current_step += 1;
                        agent.state = AgentState::Traveling;
                        agent.waiting_time = 0.0;
                    }
                }
            }
            AgentState::Traveling => {
                // 移动时不消耗耐心，或者消耗很少
                agent.patience -= dt * 0.05;

                if agent.current_step < agent.current_path.len() {
                    let current_node = &agent.current_path[agent.current_step];
                    let target_pos =
                        current_node
                            .position
                            .to_world_pos(tile_size, grid_width, grid_height);

                    let direction = (target_pos - transform.translation).normalize_or_zero();
                    let speed = 120.0;

                    let distance_to_target = transform.translation.distance(target_pos);

                    if distance_to_target > 8.0 {
                        let movement = Vec3::new(direction.x, direction.y, 0.0) * speed * dt;
                        transform.translation += movement;
                        transform.translation.z = PASSENGER_Z; // 保持Z坐标
                    } else {
                        // 到达当前节点
                        transform.translation = target_pos;
                        transform.translation.z = PASSENGER_Z;

                        match &current_node.node_type {
                            PathNodeType::Station(station_name) => {
                                if station_name == &agent.destination {
                                    agent.state = AgentState::Arrived;
                                    info!("乘客 {:?} 到达目的地", agent.color);
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
                                    info!("乘客 {:?} 到达路径终点", agent.color);
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        // 检查耐心值 - 给更多的耐心时间
        if agent.patience <= 0.0 && agent.state != AgentState::Arrived {
            agent.state = AgentState::GaveUp;
            warn!(
                "乘客 {:?} 耐心耗尽，等待了 {:.1} 秒",
                agent.color,
                agent.max_patience - agent.patience
            );
        }
    }
}

fn handle_passenger_transfers(mut passengers: Query<&mut PathfindingAgent>) {
    for mut agent in passengers.iter_mut() {
        if agent.state == AgentState::Transferring && agent.waiting_time > 0.8 {
            if agent.current_step < agent.current_path.len().saturating_sub(1) {
                agent.current_step += 1;
                agent.state = AgentState::Traveling;
                agent.waiting_time = 0.0;
            }
        }
    }
}

fn cleanup_finished_passengers(
    mut commands: Commands,
    passengers: Query<(Entity, &PathfindingAgent)>,
) {
    for (entity, agent) in passengers.iter() {
        match agent.state {
            AgentState::Arrived => {
                info!("乘客 {:?} 成功到达目的地", agent.color);
                commands.entity(entity).despawn();
            }
            AgentState::GaveUp => {
                warn!("乘客 {:?} 因耐心耗尽而放弃", agent.color);
                commands.entity(entity).despawn();
            }
            _ => {}
        }
    }
}

// ============ A* 寻路算法实现 ============

fn find_optimal_path(
    graph: &PathfindingGraph,
    origin: &str,
    destination: &str,
) -> Option<Vec<PathNode>> {
    let start_pos = *graph.station_lookup.get(origin)?;
    let end_pos = *graph.station_lookup.get(destination)?;

    if start_pos == end_pos {
        return Some(vec![PathNode {
            position: start_pos,
            node_type: PathNodeType::Station(destination.to_string()),
            estimated_wait_time: 0.0,
            route_id: None,
        }]);
    }

    let mut open_set = BinaryHeap::new();
    let mut closed_set = HashSet::new();
    let mut came_from = HashMap::new();

    let start_node = AStarNode::new(start_pos, 0.0, heuristic(start_pos, end_pos), None, 0);
    open_set.push(start_node);

    while let Some(current) = open_set.pop() {
        if current.position == end_pos {
            return Some(reconstruct_path(came_from, current.position, graph));
        }

        closed_set.insert(current.position);

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
                    current.g_cost + connection.cost + (route_changes as f32 * 3.0);

                let neighbor = AStarNode::new(
                    connection.to,
                    tentative_g_cost,
                    heuristic(connection.to, end_pos),
                    Some(current.position),
                    route_changes,
                );

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

    None
}

fn heuristic(pos1: GridPos, pos2: GridPos) -> f32 {
    ((pos1.x - pos2.x).abs() + (pos1.y - pos2.y).abs()) as f32
}

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
                estimated_wait_time: 1.0,
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

// ============ 辅助函数 ============

pub fn get_segment_connections(
    pos: GridPos,
    segment_type: &RouteSegmentType,
    rotation: u32,
) -> Vec<GridPos> {
    // 直线段的特殊处理：水平放置时应该有水平连接
    let base_connections = match segment_type {
        RouteSegmentType::Straight => {
            // 根据旋转决定连接方向
            match rotation % 180 {
                0 => vec![(0, -1), (0, 1)],  // 垂直：上下连接
                90 => vec![(-1, 0), (1, 0)], // 水平：左右连接
                _ => vec![(0, -1), (0, 1)],  // 默认垂直
            }
        }
        RouteSegmentType::Curve => vec![(0, -1), (1, 0)], // L型：上和右
        RouteSegmentType::TSplit => vec![(0, -1), (0, 1), (1, 0)], // T型：上下右
        RouteSegmentType::Cross => vec![(0, -1), (0, 1), (-1, 0), (1, 0)], // 十字：四方向
        RouteSegmentType::Bridge | RouteSegmentType::Tunnel => {
            // 和直线段一样处理
            match rotation % 180 {
                0 => vec![(0, -1), (0, 1)],  // 垂直
                90 => vec![(-1, 0), (1, 0)], // 水平
                _ => vec![(0, -1), (0, 1)],  // 默认垂直
            }
        }
    };

    // 对于非直线段，应用旋转
    let final_connections = if matches!(
        segment_type,
        RouteSegmentType::Straight | RouteSegmentType::Bridge | RouteSegmentType::Tunnel
    ) {
        // 直线段已经在上面处理了旋转
        base_connections
    } else {
        // 其他类型应用旋转变换
        base_connections
            .into_iter()
            .map(|(dx, dy)| rotate_offset(dx, dy, rotation))
            .collect()
    };

    final_connections
        .into_iter()
        .map(|(dx, dy)| GridPos::new(pos.x + dx, pos.y + dy))
        .collect()
}

pub fn rebuild_pathfinding_graph(
    pathfinding_graph: &mut PathfindingGraph,
    game_state: &super::GameState,
) {
    pathfinding_graph.connections.clear();
    pathfinding_graph.nodes.clear();
    pathfinding_graph.station_lookup.clear();

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

fn rotate_offset(dx: i32, dy: i32, rotation: u32) -> (i32, i32) {
    match rotation % 360 {
        0 => (dx, dy),
        90 => (-dy, dx),
        180 => (-dx, -dy),
        270 => (dy, -dx),
        _ => (dx, dy),
    }
}

fn get_adjacent_positions(pos: GridPos) -> Vec<GridPos> {
    vec![
        GridPos::new(pos.x, pos.y - 1),
        GridPos::new(pos.x, pos.y + 1),
        GridPos::new(pos.x - 1, pos.y),
        GridPos::new(pos.x + 1, pos.y),
    ]
}

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
                station_connections.push((
                    station_pos,
                    Connection {
                        to: adj_pos,
                        cost: 0.5,
                        route_id: None,
                        connection_type: ConnectionType::Walk,
                    },
                ));

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

    for (from_pos, connection) in station_connections {
        pathfinding_graph
            .connections
            .entry(from_pos)
            .or_insert_with(Vec::new)
            .push(connection);
    }
}
