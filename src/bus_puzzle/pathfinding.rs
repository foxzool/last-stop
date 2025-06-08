// src/bus_puzzle/pathfinding.rs - 正式版寻路系统

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    collections::{BinaryHeap, HashMap, HashSet},
};

use super::{
    get_neighbors, AgentState, Connection, ConnectionType, GameState, GameStateEnum, GraphNode,
    GraphNodeType, GridPos, LevelManager, PathfindingAgent, PathfindingGraph, RouteSegment,
    RouteSegmentType, StationEntity, BUS_SPEED, PASSENGER_Z, WALKING_SPEED,
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
                    let connection_positions = segment
                        .segment_type
                        .get_connection_positions(adj_pos, segment.rotation);
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
                        connection_positions
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
        let connection_positions = segment
            .segment_type
            .get_connection_positions(*pos, segment.rotation);
        trace!(
            "路线段 {:?} at {:?} 旋转{}° 连接位置: {:?}",
            segment.segment_type,
            pos,
            segment.rotation,
            connection_positions
        );

        for connection_pos in connection_positions {
            if let Some(target_segment) = route_segments_by_pos.get(&connection_pos) {
                // 检查目标路线段是否也有朝向当前路线段的连接口
                if target_segment.segment_type.has_connection_to(
                    connection_pos,
                    *pos,
                    target_segment.rotation,
                ) {
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

                    trace!("双向连接建立: {:?} <-> {:?}", pos, connection_pos);
                } else {
                    trace!(
                        "单向连接拒绝: {:?} -> {:?} (目标段没有回连接口)",
                        pos,
                        connection_pos
                    );
                }
            } else if pathfinding_graph
                .station_lookup
                .values()
                .any(|&station_pos| station_pos == connection_pos)
            {
                // 连接到站点的情况不需要双向检查
                add_connection_if_not_exists(
                    pathfinding_graph,
                    *pos,
                    connection_pos,
                    ConnectionType::BusRoute,
                );

                trace!("连接到站点: {:?} -> {:?}", pos, connection_pos);
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
                        station_name,
                        station_pos,
                        segment.segment_type,
                        adj_pos
                    );
                } else {
                    trace!(
                        "跳过连接: 站点 {} {:?} -> 路线段 {:?} {:?} (路线段没有朝向站点的端口)",
                        station_name,
                        station_pos,
                        segment.segment_type,
                        adj_pos
                    );
                }
            }
        }
    }
}

/// 检查路线段是否能连接到站点（检查路线段是否有朝向站点的端口）
fn segment_can_connect_to_station(segment: &RouteSegment, station_pos: GridPos) -> bool {
    segment
        .segment_type
        .has_connection_to(segment.grid_pos, station_pos, segment.rotation)
}

fn find_paths_for_new_passengers(
    pathfinding_graph: Res<PathfindingGraph>,
    mut passengers: Query<&mut PathfindingAgent, Added<PathfindingAgent>>,
) {
    for mut agent in passengers.iter_mut() {
        // 禁用乘客寻路：让乘客只能等车，不能自己寻路
        info!(
            "乘客 {:?} 生成，设置为等车模式: {} -> {}",
            agent.color, agent.origin, agent.destination
        );

        // 清空路径，让乘客等车
        agent.current_path.clear();
        agent.current_step = 0;
        agent.state = AgentState::WaitingAtStation;
        agent.waiting_time = 0.0;

        // // 不进行寻路，让公交车来处理
        // warn!(
        //     "乘客寻路已禁用 - 乘客 {:?} 需要等车从 {} 到 {}",
        //     agent.color, agent.origin, agent.destination
        // );
    }
}

fn update_passenger_movement(
    time: Res<Time>,
    mut passengers: Query<(&mut PathfindingAgent, &mut Transform)>,
    level_manager: Res<LevelManager>,
    keyboard_input: Res<ButtonInput<KeyCode>>, // 添加键盘输入用于调试
) {
    let dt = time.delta_secs();
    let _tile_size = level_manager.tile_size;

    let (_grid_width, _grid_height) = if let Some(level_data) = &level_manager.current_level {
        level_data.grid_size
    } else {
        return;
    };

    // F7 - 调试乘客移动状态
    if keyboard_input.just_pressed(KeyCode::F7) {
        info!("=== 乘客移动调试 (寻路已禁用) ===");
        for (agent, transform) in passengers.iter() {
            info!("乘客 {:?}:", agent.color);
            info!(
                "  位置: {:.1}, {:.1}",
                transform.translation.x, transform.translation.y
            );
            info!("  状态: {:?}", agent.state);
            info!("  耐心: {:.1}/{:.1}", agent.patience, agent.max_patience);
            info!("  等待时间: {:.1}s", agent.waiting_time);
            info!("  路径: {} 步 (被禁用)", agent.current_path.len());

            if agent.current_path.is_empty() {
                info!("  ✅ 正确状态: 等车中，没有自主寻路路径");
            } else {
                warn!("  ⚠️ 异常状态: 仍有寻路路径，应该被清空");
            }
            info!(""); // 空行分隔
        }
    }

    // 禁用乘客自主移动 - 乘客只能等车，不能自己移动
    for (mut agent, mut transform) in passengers.iter_mut() {
        // 确保乘客在正确的Z层级
        transform.translation.z = crate::bus_puzzle::PASSENGER_Z;

        match agent.state {
            AgentState::WaitingAtStation => {
                agent.waiting_time += dt;
                // 减缓耐心消耗速度
                agent.patience -= dt * 0.05; // 进一步减慢耐心消耗

                // 清空任何可能存在的寻路路径
                if !agent.current_path.is_empty() {
                    agent.current_path.clear();
                    agent.current_step = 0;
                }

                // 乘客只能等车，不能自主移动
                trace!(
                    "乘客 {:?} 等车中: {} -> {} (等待 {:.1}s)",
                    agent.color,
                    agent.origin,
                    agent.destination,
                    agent.waiting_time
                );
            }
            AgentState::Traveling => {
                // 如果乘客处于移动状态，将其重置为等车状态
                // warn!("乘客 {:?} 不应该自主移动，重置为等车状态", agent.color);
                agent.state = AgentState::WaitingAtStation;
                agent.current_path.clear();
                agent.current_step = 0;
            }
            AgentState::Transferring => {
                // 换乘状态也重置为等车
                warn!("乘客 {:?} 换乘状态重置为等车状态", agent.color);
                agent.state = AgentState::WaitingAtStation;
                agent.current_path.clear();
                agent.current_step = 0;
            }
            AgentState::Arrived => {
                // 已到达状态保持不变
                info!("乘客 {:?} 已到达目的地", agent.color);
            }
            AgentState::GaveUp => {
                // 放弃状态保持不变
                warn!("乘客 {:?} 已放弃", agent.color);
            }
        }

        // 检查耐心值 - 给更多的耐心时间
        if agent.patience <= 0.0
            && agent.state != AgentState::Arrived
            && agent.state != AgentState::GaveUp
        {
            agent.state = AgentState::GaveUp;
            warn!(
                "乘客 {:?} 等车耐心耗尽，等待了 {:.1} 秒",
                agent.color, agent.waiting_time
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

pub fn find_optimal_path(
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
