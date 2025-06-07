// src/bus_puzzle/pathfinding.rs - 正式版寻路系统

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    collections::{BinaryHeap, HashMap, HashSet},
};

use super::{
    AgentState, Connection, ConnectionType, GameStateEnum, GraphNode, GraphNodeType, GridPos,
    LevelManager, PASSENGER_Z, PathfindingAgent, PathfindingGraph, RouteSegment,
    RouteSegmentType, StationEntity,
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
    create_route_connections(&mut pathfinding_graph, &route_segments_by_pos);
    create_station_connections(&mut pathfinding_graph, &route_segments_by_pos);
}



fn create_route_connections(
    pathfinding_graph: &mut PathfindingGraph,
    route_segments_by_pos: &HashMap<GridPos, &RouteSegment>,
) {
    for (pos, segment) in route_segments_by_pos {
        let connections = get_segment_connections(*pos, &segment.segment_type, segment.rotation);

        for connection_pos in connections {
            if route_segments_by_pos.contains_key(&connection_pos)
                || pathfinding_graph
                    .station_lookup
                    .values()
                    .any(|&station_pos| station_pos == connection_pos)
            {
                pathfinding_graph
                    .connections
                    .entry(*pos)
                    .or_insert_with(Vec::new)
                    .push(Connection {
                        to: connection_pos,
                        cost: 1.0,
                        route_id: Some(format!("route_{}", pos.x + pos.y)),
                        connection_type: ConnectionType::BusRoute,
                    });
            }
        }
    }
}

fn create_station_connections(
    pathfinding_graph: &mut PathfindingGraph,
    route_segments_by_pos: &HashMap<GridPos, &RouteSegment>,
) {
    for (_station_name, &station_pos) in &pathfinding_graph.station_lookup {
        let adjacent_positions = get_adjacent_positions(station_pos);

        for adj_pos in adjacent_positions {
            if route_segments_by_pos.contains_key(&adj_pos) {
                // 站点到路线段
                pathfinding_graph
                    .connections
                    .entry(station_pos)
                    .or_insert_with(Vec::new)
                    .push(Connection {
                        to: adj_pos,
                        cost: 0.5,
                        route_id: None,
                        connection_type: ConnectionType::Walk,
                    });

                // 路线段到站点
                pathfinding_graph
                    .connections
                    .entry(adj_pos)
                    .or_insert_with(Vec::new)
                    .push(Connection {
                        to: station_pos,
                        cost: 0.5,
                        route_id: None,
                        connection_type: ConnectionType::Walk,
                    });
            }
        }
    }
}

fn find_paths_for_new_passengers(
    pathfinding_graph: Res<PathfindingGraph>,
    mut passengers: Query<&mut PathfindingAgent, Added<PathfindingAgent>>,
) {
    for mut agent in passengers.iter_mut() {
        if let Some(path) = find_optimal_path(&pathfinding_graph, &agent.origin, &agent.destination)
        {
            agent.current_path = path;
            agent.current_step = 0;
            agent.state = AgentState::WaitingAtStation;

            info!(
                "为乘客 {:?} 找到路径，共 {} 步",
                agent.color,
                agent.current_path.len()
            );
        } else {
            warn!(
                "无法为乘客 {:?} 找到从 {} 到 {} 的路径",
                agent.color, agent.origin, agent.destination
            );
            agent.state = AgentState::GaveUp;
        }
    }
}

fn update_passenger_movement(
    time: Res<Time>,
    mut passengers: Query<(&mut PathfindingAgent, &mut Transform)>,
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
        // 确保乘客在正确的Z层级
        transform.translation.z = PASSENGER_Z;

        match agent.state {
            AgentState::WaitingAtStation | AgentState::Transferring => {
                agent.waiting_time += dt;
                agent.patience -= dt;

                // 等待一定时间后开始移动
                if agent.waiting_time > 1.0
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
            _ => {}
        }

        // 检查耐心值
        if agent.patience <= 0.0 && agent.state != AgentState::Arrived {
            agent.state = AgentState::GaveUp;
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
    let base_connections = match segment_type {
        RouteSegmentType::Straight => vec![(0, -1), (0, 1)],
        RouteSegmentType::Curve => vec![(0, -1), (1, 0)],
        RouteSegmentType::TSplit => vec![(0, -1), (0, 1), (1, 0)],
        RouteSegmentType::Cross => vec![(0, -1), (0, 1), (-1, 0), (1, 0)],
        RouteSegmentType::Bridge | RouteSegmentType::Tunnel => vec![(0, -1), (0, 1)],
    };

    base_connections
        .into_iter()
        .map(|(dx, dy)| {
            let (new_dx, new_dy) = rotate_offset(dx, dy, rotation);
            GridPos::new(pos.x + new_dx, pos.y + new_dy)
        })
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
