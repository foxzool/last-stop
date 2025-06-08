// src/bus_puzzle/pathfinding.rs - 正式版寻路系统

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    collections::{BinaryHeap, HashMap, HashSet},
};

use super::{
    get_neighbors, manhattan_distance, rotate_offset, AgentState, Connection, ConnectionType,
    GameState, GameStateEnum, GraphNode, GraphNodeType, GridPos, LevelManager, PathfindingAgent,
    PathfindingGraph, RouteSegment, RouteSegmentType, StationEntity, PASSENGER_Z,
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
    keyboard_input: Res<ButtonInput<KeyCode>>, // 添加键盘输入以便调试
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

    // 按F8显示详细的连接调试信息
    if keyboard_input.just_pressed(KeyCode::F8) {
        info!("=== 详细连接调试 ===");

        // 显示每个站点的连接信息
        for (station_name, &station_pos) in &pathfinding_graph.station_lookup {
            info!("站点: {} at {:?}", station_name, station_pos);

            if let Some(connections) = pathfinding_graph.connections.get(&station_pos) {
                for conn in connections {
                    info!(
                        "  -> {:?} (类型: {:?}, 成本: {:.1})",
                        conn.to, conn.connection_type, conn.cost
                    );
                }
            } else {
                warn!("  没有连接！");
            }

            // 显示相邻的路线段分析
            let adjacent_positions = get_neighbors(station_pos);

            info!("  相邻位置分析:");
            for adj_pos in adjacent_positions {
                if let Some(segment) = route_segments_by_pos.get(&adj_pos) {
                    let can_connect = segment_can_connect_to_station(segment, station_pos);
                    let connection_points =
                        get_segment_connections(adj_pos, &segment.segment_type, segment.rotation);
                    info!(
                        "    {:?}: {:?} 旋转{}° - {} (连接点: {:?})",
                        adj_pos,
                        segment.segment_type,
                        segment.rotation,
                        if can_connect {
                            "✅可连接"
                        } else {
                            "❌不可连接"
                        },
                        connection_points
                    );
                } else {
                    info!("    {:?}: 空位置", adj_pos);
                }
            }
        }
    }

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
    trace!(
        "开始创建路线段连接，共 {} 个路线段",
        route_segments_by_pos.len()
    );

    for (pos, segment) in route_segments_by_pos {
        let theoretical_connections =
            get_segment_connections(*pos, &segment.segment_type, segment.rotation);
        trace!(
            "路线段 {:?} at {:?} 旋转{}° 理论连接: {:?}",
            segment.segment_type,
            pos,
            segment.rotation,
            theoretical_connections
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

                trace!("理论连接: {:?} <-> {:?}", pos, connection_pos);
            }
        }

        // 重要：强制添加所有相邻路线段的连接
        for other_pos in route_segments_by_pos.keys() {
            if *other_pos != *pos {
                let distance = manhattan_distance(*pos, *other_pos);
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
                        trace!("强制水平连接: {:?} <-> {:?}", pos, other_pos);
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
                        trace!("强制垂直连接: {:?} <-> {:?}", pos, other_pos);
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
    let connections = pathfinding_graph.connections.entry(from).or_default();

    // 检查是否已经存在这个连接
    if !connections.iter().any(|conn| conn.to == to) {
        let cost = match connection_type {
            ConnectionType::Walk => 0.5,
            ConnectionType::BusRoute => 1.0,
            ConnectionType::Transfer => 2.0,
        };

        connections.push(Connection {
            to,
            cost,
            route_id: Some(format!("route_{}", from.x + from.y)),
            connection_type,
        });
    }
}

pub fn create_station_connections_improved(
    pathfinding_graph: &mut PathfindingGraph,
    route_segments_by_pos: &HashMap<GridPos, &RouteSegment>,
) {
    let station_lookup: Vec<_> = pathfinding_graph
        .station_lookup
        .iter()
        .map(|(name, pos)| (name.clone(), *pos))
        .collect();
    for (station_name, station_pos) in station_lookup {
        trace!("检查站点 {} at {:?} 的连接", station_name, station_pos);

        // 只检查站点直接相邻的位置（距离为1）
        let adjacent_positions = get_neighbors(station_pos);

        for adj_pos in adjacent_positions {
            if let Some(segment) = route_segments_by_pos.get(&adj_pos) {
                // 详细调试特定情况
                if station_name == "B站" && matches!(segment.segment_type, RouteSegmentType::Curve)
                {
                    info!("=== 详细调试 B站 -> Curve ===");
                    info!("  站点位置: {:?}", station_pos);
                    info!("  Curve位置: {:?}", segment.grid_pos);
                    info!("  Curve旋转: {}°", segment.rotation);

                    let connection_points = get_segment_connections(
                        segment.grid_pos,
                        &segment.segment_type,
                        segment.rotation,
                    );
                    info!("  Curve连接点: {:?}", connection_points);
                    info!(
                        "  站点是否在连接点中: {}",
                        connection_points.contains(&station_pos)
                    );

                    // 显示方向分析
                    let dx = station_pos.x - segment.grid_pos.x;
                    let dy = station_pos.y - segment.grid_pos.y;
                    info!("  相对方向: dx={}, dy={} (站点相对于Curve的位置)", dx, dy);
                }

                // 检查路线段是否有朝向站点的连接点
                if segment_can_connect_to_station(segment, station_pos) {
                    // 站点到路线段
                    add_connection_if_not_exists(
                        pathfinding_graph,
                        station_pos,
                        adj_pos,
                        ConnectionType::Walk,
                    );

                    // 路线段到站点
                    add_connection_if_not_exists(
                        pathfinding_graph,
                        adj_pos,
                        station_pos,
                        ConnectionType::Walk,
                    );

                    trace!(
                        "建立连接: 站点 {} {:?} <-> 路线段 {:?} {:?}",
                        station_name, station_pos, segment.segment_type, adj_pos
                    );
                } else {
                    trace!(
                        "跳过连接: 站点 {} {:?} -> 路线段 {:?} {:?} (路线段没有朝向站点的端口)",
                        station_name, station_pos, segment.segment_type, adj_pos
                    );
                }
            }
        }
    }
}

/// 检查路线段是否能连接到站点（检查路线段是否有朝向站点的端口）
fn segment_can_connect_to_station(segment: &RouteSegment, station_pos: GridPos) -> bool {
    let segment_pos = segment.grid_pos;
    let connection_points =
        get_segment_connections(segment_pos, &segment.segment_type, segment.rotation);

    // 检查站点是否在路线段的连接点列表中
    connection_points.contains(&station_pos)
}

/// 检查站点是否可以连接到路线段
fn can_connect_station_to_segment(
    station_pos: GridPos,
    segment_pos: GridPos,
    segment: &RouteSegment,
) -> bool {
    let distance = manhattan_distance(station_pos, segment_pos);

    // 直接相邻
    if distance == 1 {
        return true;
    }

    // 检查是否在路线段的连接点附近
    let connection_points =
        get_segment_connections(segment_pos, &segment.segment_type, segment.rotation);
    for connection_point in connection_points {
        if manhattan_distance(station_pos, connection_point) <= 1 {
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

fn find_paths_for_new_passengers(
    pathfinding_graph: Res<PathfindingGraph>,
    mut passengers: Query<&mut PathfindingAgent, Added<PathfindingAgent>>,
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

        if let Some(basic_path) =
            find_optimal_path(&pathfinding_graph, &agent.origin, &agent.destination)
        {
            // 暂时禁用路口增强功能，使用基础路径
            agent.current_path = basic_path;
            agent.current_step = 0;
            agent.state = AgentState::WaitingAtStation;
            agent.waiting_time = 0.0;

            info!(
                "为乘客 {:?} 找到路径，共 {} 步",
                agent.color,
                agent.current_path.len()
            );

            // 显示路径详情
            for (i, node) in agent.current_path.iter().enumerate() {
                info!("  步骤 {}: {:?} ({:?})", i, node.position, node.node_type);
            }
        } else {
            info!("暂时无法为乘客 {:?} 找到路径，设置为等待状态", agent.color);
            agent.state = AgentState::WaitingAtStation;
            agent.waiting_time = 0.0;
        }
    }
}

/// 检查路径是否需要路口增强
pub fn path_needs_junction_enhancement(
    path: &[PathNode],
    route_segments: &Query<&RouteSegment>,
) -> bool {
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

    for node in original_path.iter() {
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
fn find_junction_at_position(
    pos: GridPos,
    route_segments: &Query<&RouteSegment>,
) -> Option<RouteSegment> {
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

fn update_passenger_movement(
    time: Res<Time>,
    mut passengers: Query<(&mut PathfindingAgent, &mut Transform)>,
    level_manager: Res<LevelManager>,
    pathfinding_graph: Res<PathfindingGraph>,
    route_segments: Query<&RouteSegment>,      // 添加路线段查询
    keyboard_input: Res<ButtonInput<KeyCode>>, // 添加键盘输入用于调试
) {
    let dt = time.delta_secs();
    let tile_size = level_manager.tile_size;

    let (grid_width, grid_height) = if let Some(level_data) = &level_manager.current_level {
        level_data.grid_size
    } else {
        return;
    };

    // F7 - 调试乘客移动状态
    if keyboard_input.just_pressed(KeyCode::F7) {
        info!("=== 乘客移动调试 ===");
        for (agent, transform) in passengers.iter() {
            if matches!(agent.state, AgentState::Traveling) {
                info!("乘客 {:?}:", agent.color);
                info!("  位置: {:?}", transform.translation);
                info!(
                    "  路径步骤: {}/{}",
                    agent.current_step,
                    agent.current_path.len()
                );

                if agent.current_step < agent.current_path.len() {
                    let current_node = &agent.current_path[agent.current_step];
                    let target_pos =
                        current_node
                            .position
                            .to_world_pos(tile_size, grid_width, grid_height);
                    let distance = transform.translation.distance(target_pos);

                    info!(
                        "  当前目标: {:?} (距离: {:.1})",
                        current_node.position, distance
                    );
                    info!("  节点类型: {:?}", current_node.node_type);

                    // 检查是否是路口
                    let is_junction = route_segments.iter().any(|segment| {
                        segment.grid_pos == current_node.position
                            && segment.is_active
                            && matches!(
                                segment.segment_type,
                                RouteSegmentType::Curve
                                    | RouteSegmentType::TSplit
                                    | RouteSegmentType::Cross
                            )
                    });

                    if is_junction {
                        info!("  这是一个路口！");
                    }
                }
            }
        }
    }

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

                if agent.waiting_time > 0.8
                    && agent.current_step < agent.current_path.len().saturating_sub(1)
                {
                    agent.current_step += 1;
                    agent.state = AgentState::Traveling;
                    agent.waiting_time = 0.0;
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

                    // 检查当前节点是否是路口类型（Curve, TSplit, Cross）
                    let is_junction = route_segments.iter().any(|segment| {
                        segment.grid_pos == current_node.position
                            && segment.is_active
                            && matches!(
                                segment.segment_type,
                                RouteSegmentType::Curve
                                    | RouteSegmentType::TSplit
                                    | RouteSegmentType::Cross
                            )
                    });

                    if is_junction {
                        // 对于路口，使用更宽松的到达判定
                        let distance_to_target = transform.translation.distance(target_pos);
                        if distance_to_target > 15.0 {
                            // 距离较远，正常移动
                            let direction =
                                (target_pos - transform.translation).normalize_or_zero();
                            let speed = 100.0; // 在路口附近稍慢一些
                            let movement = Vec3::new(direction.x, direction.y, 0.0) * speed * dt;
                            transform.translation += movement;
                            transform.translation.z = PASSENGER_Z;
                        } else {
                            // 接近路口，直接移动到下一个节点
                            let next_step = agent.current_step + 1;
                            if next_step >= agent.current_path.len() {
                                agent.state = AgentState::Arrived;
                                info!("乘客 {:?} 到达路径终点", agent.color);
                            } else {
                                info!(
                                    "乘客 {:?} 通过路口 {:?}",
                                    agent.color, agent.current_path[agent.current_step].position
                                );
                            }
                            agent.current_step = next_step;
                        }
                    } else {
                        // 普通路段的移动逻辑
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
        if agent.state == AgentState::Transferring
            && agent.waiting_time > 0.8
            && agent.current_step < agent.current_path.len().saturating_sub(1)
        {
            agent.current_step += 1;
            agent.state = AgentState::Traveling;
            agent.waiting_time = 0.0;
        }
    }
}

fn cleanup_finished_passengers(
    mut commands: Commands,
    passengers: Query<(Entity, &PathfindingAgent)>,
    mut game_state: ResMut<GameState>,
) {
    for (entity, agent) in passengers.iter() {
        match agent.state {
            AgentState::Arrived => {
                info!("乘客 {:?} 成功到达目的地", agent.color);
                game_state.passenger_stats.total_arrived += 1;
                commands.entity(entity).despawn();
            }
            AgentState::GaveUp => {
                warn!("乘客 {:?} 因耐心耗尽而放弃", agent.color);
                commands.entity(entity).despawn();
                game_state.passenger_stats.total_gave_up += 1;
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
                    .is_none_or(|existing| neighbor.f_cost < existing.f_cost);

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
    // 直线段的特殊处理：假设纹理默认是水平方向
    let base_connections = match segment_type {
        RouteSegmentType::Straight => {
            // 假设直线段纹理默认是水平方向，所以基础连接是左右
            match rotation % 180 {
                0 => vec![(-1, 0), (1, 0)],  // 水平：左右连接 (纹理默认方向)
                90 => vec![(0, -1), (0, 1)], // 垂直：上下连接 (旋转90度后)
                _ => vec![(-1, 0), (1, 0)],  // 默认水平
            }
        }
        RouteSegmentType::Curve => vec![(-1, 0), (0, 1)],
        RouteSegmentType::TSplit => vec![(0, -1), (0, 1), (1, 0)], // T型：上下右
        RouteSegmentType::Cross => vec![(0, -1), (0, 1), (-1, 0), (1, 0)], // 十字：四方向
        RouteSegmentType::Bridge | RouteSegmentType::Tunnel => {
            // 和直线段一样处理
            match rotation % 180 {
                0 => vec![(-1, 0), (1, 0)],  // 水平：左右连接
                90 => vec![(0, -1), (0, 1)], // 垂直：上下连接
                _ => vec![(-1, 0), (1, 0)],  // 默认水平
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
            .or_default()
            .push(connection);
    }

    // 建立站点与路线段的连接
    let mut station_connections = Vec::new();

    for (_station_name, &station_pos) in &pathfinding_graph.station_lookup {
        let adjacent_positions = get_neighbors(station_pos);

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
            .or_default()
            .push(connection);
    }
}
