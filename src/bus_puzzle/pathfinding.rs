// src/bus_puzzle/pathfinding.rs

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    collections::{BinaryHeap, HashMap, HashSet, VecDeque},
};

// 使用相对路径引用同模块下的其他文件
use super::{
    GridPos, LevelManager, PassengerColor, RouteSegment, RouteSegmentType, StationEntity,
    StationType, TerrainType,
};

// ============ 寻路相关组件 ============

#[derive(Component)]
pub struct PathfindingAgent {
    pub color: PassengerColor,
    pub origin: String,
    pub destination: String,
    pub current_path: Vec<PathNode>,
    pub current_step: usize,
    pub state: AgentState,
    pub patience: f32,
    pub max_patience: f32,
    pub waiting_time: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AgentState {
    WaitingAtStation,
    Traveling,
    Transferring,
    Arrived,
    GaveUp,
}

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

// ============ 寻路资源 ============

#[derive(Resource)]
pub struct PathfindingGraph {
    pub nodes: HashMap<GridPos, GraphNode>,
    pub connections: HashMap<GridPos, Vec<Connection>>,
    pub station_lookup: HashMap<String, GridPos>,
    pub route_network: HashMap<String, RouteInfo>,
}

#[derive(Debug, Clone)]
pub struct GraphNode {
    pub position: GridPos,
    pub node_type: GraphNodeType,
    pub station_name: Option<String>,
    pub is_accessible: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GraphNodeType {
    Station,
    RouteSegment,
    Intersection,
}

#[derive(Debug, Clone)]
pub struct Connection {
    pub to: GridPos,
    pub cost: f32,
    pub route_id: Option<String>,
    pub connection_type: ConnectionType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionType {
    Walk,
    BusRoute,
    Transfer,
}

#[derive(Debug, Clone)]
pub struct RouteInfo {
    pub id: String,
    pub segments: Vec<GridPos>,
    pub frequency: f32,
    pub capacity: u32,
    pub is_active: bool,
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

// ============ 寻路系统 ============

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
                    .chain(),
            );
    }
}

impl Default for PathfindingGraph {
    fn default() -> Self {
        Self {
            nodes: HashMap::new(),
            connections: HashMap::new(),
            station_lookup: HashMap::new(),
            route_network: HashMap::new(),
        }
    }
}

// ============ 系统实现 ============

fn update_pathfinding_graph(
    mut pathfinding_graph: ResMut<PathfindingGraph>,
    route_segments: Query<(&RouteSegment, &Transform), Changed<RouteSegment>>,
    stations: Query<&StationEntity, Changed<StationEntity>>,
) {
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

    // 添加路线段节点并建立连接
    let mut route_segments_by_pos = HashMap::new();
    for (segment, transform) in route_segments.iter() {
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
    for (pos, segment) in &route_segments_by_pos {
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

    // 建立站点与相邻路线段的连接
    for (station_name, &station_pos) in &pathfinding_graph.station_lookup {
        use bevy::prelude::*;
        use serde::{Deserialize, Serialize};
        use std::{
            cmp::Ordering,
            collections::{BinaryHeap, HashMap, HashSet, VecDeque},
        };

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

        // ============ 寻路系统 ============

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
                            .chain(),
                    );
            }
        }

        // ============ 系统实现 ============

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

            // 添加路线段节点并建立连接
            let mut route_segments_by_pos = HashMap::new();
            for (segment, transform) in route_segments.iter() {
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
                let connections =
                    get_segment_connections(*pos, &segment.segment_type, segment.rotation);

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

            for (station_name, &station_pos) in &pathfinding_graph.station_lookup {
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

        fn find_paths_for_new_passengers(
            mut commands: Commands,
            pathfinding_graph: Res<PathfindingGraph>,
            mut passengers: Query<
                (Entity, &mut PathfindingAgent),
                (Added<PathfindingAgent>, Without<super::PassengerEntity>),
            >,
        ) {
            for (entity, mut agent) in passengers.iter_mut() {
                if let Some(path) =
                    find_optimal_path(&pathfinding_graph, &agent.origin, &agent.destination)
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

            // 获取网格尺寸信息
            let (grid_width, grid_height) = if let Some(level_data) = &level_manager.current_level {
                level_data.grid_size
            } else {
                return;
            };

            for (mut agent, mut transform) in passengers.iter_mut() {
                match agent.state {
                    AgentState::WaitingAtStation | AgentState::Transferring => {
                        agent.waiting_time += dt;
                        agent.patience -= dt;

                        if agent.waiting_time > 2.0
                            && agent.current_step < agent.current_path.len() - 1
                        {
                            agent.current_step += 1;
                            agent.state = AgentState::Traveling;
                            agent.waiting_time = 0.0;
                        }
                    }
                    AgentState::Traveling => {
                        if agent.current_step < agent.current_path.len() {
                            let current_node = &agent.current_path[agent.current_step];
                            let target_pos = current_node.position.to_world_pos(
                                tile_size,
                                grid_width,
                                grid_height,
                            );

                            let direction = (target_pos - transform.translation).normalize();
                            let speed = 100.0;

                            if transform.translation.distance(target_pos) > 5.0 {
                                transform.translation += direction * speed * dt;
                            } else {
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

                if agent.patience <= 0.0 && agent.state != AgentState::Arrived {
                    agent.state = AgentState::GaveUp;
                }
            }
        }

        fn handle_passenger_transfers(mut passengers: Query<&mut PathfindingAgent>) {
            for mut agent in passengers.iter_mut() {
                if agent.state == AgentState::Transferring && agent.waiting_time > 1.0 {
                    if agent.current_step < agent.current_path.len() - 1 {
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
                        warn!("乘客 {:?} 耐心耗尽，放弃行程", agent.color);
                        commands.entity(entity).despawn();
                    }
                    _ => {}
                }
            }
        }

        // ============ 寻路算法实现 ============

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

                        let route_changes =
                            if connection.connection_type == ConnectionType::Transfer {
                                current.route_changes + 1
                            } else {
                                current.route_changes
                            };

                        let tentative_g_cost =
                            current.g_cost + connection.cost + (route_changes as f32 * 5.0);

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
                        estimated_wait_time: 2.0,
                        route_id: None,
                    });
                }
                current = parent;
            }

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

        fn get_segment_connections(
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
    }
}

/// 为新生成的乘客寻找路径
fn find_paths_for_new_passengers(
    mut commands: Commands,
    pathfinding_graph: Res<PathfindingGraph>,
    mut passengers: Query<
        (Entity, &mut PathfindingAgent),
        (Added<PathfindingAgent>, Without<super::PassengerEntity>),
    >,
) {
    for (entity, mut agent) in passengers.iter_mut() {
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
                if agent.waiting_time > 2.0 && agent.current_step < agent.current_path.len() - 1 {
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
            if agent.current_step < agent.current_path.len() - 1 {
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
                info!("乘客 {:?} 成功到达目的地", agent.color);
                commands.entity(entity).despawn();
            }
            AgentState::GaveUp => {
                warn!("乘客 {:?} 耐心耗尽，放弃行程", agent.color);
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

// ============ 辅助函数 ============

/// 根据路线段类型和旋转角度获取连接位置
fn get_segment_connections(
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
