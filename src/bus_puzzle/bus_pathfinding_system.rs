// src/bus_puzzle/bus_pathfinding_system.rs - 公交车智能寻路系统
// 使用乘客验证过的寻路算法来驱动公交车移动

use crate::bus_puzzle::{
    find_optimal_path, BusDirection, BusState, BusVehicle, GameState, GameStateEnum, LevelManager,
    PathNode, PathNodeType, PathfindingGraph, RouteSegment, StationEntity, PASSENGER_Z, ROUTE_Z,
};
use bevy::prelude::*;
use std::collections::{HashMap, HashSet};

// ============ 公交车寻路组件 ============

#[derive(Component)]
pub struct BusPathfindingAgent {
    pub vehicle_id: String,
    pub route_id: String,
    pub current_path: Vec<PathNode>,
    pub current_step: usize,
    pub target_station: String,
    pub state: BusPathfindingState,
    pub path_progress: f32,
    pub next_station_index: usize,
    pub stations_to_visit: Vec<String>, // 路线上的所有站点
    pub direction: BusDirection,
    pub is_returning: bool, // 是否在返程
}

#[derive(Debug, Clone, PartialEq)]
pub enum BusPathfindingState {
    Planning,       // 规划路径中
    Following,      // 跟随路径中
    AtStation,      // 在站点停靠
    TurningAround,  // 调头中
    WaitingForPath, // 等待路径生成
}

#[derive(Resource, Default)]
pub struct BusPathfindingManager {
    pub bus_routes: HashMap<String, BusRouteInfo>,
    pub station_connections: HashMap<String, Vec<String>>, // 站点连接关系
    pub path_cache: HashMap<(String, String), Vec<PathNode>>, // 路径缓存
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct BusRouteInfo {
    pub route_id: String,
    pub stations: Vec<String>,
    pub is_circular: bool,
    pub max_vehicles: u32,
}

// ============ 公交车寻路系统插件 ============

pub struct BusPathfindingPlugin;

impl Plugin for BusPathfindingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BusPathfindingManager>().add_systems(
            Update,
            (
                discover_bus_routes_pathfinding,
                update_bus_pathfinding,
                move_buses_along_paths,
                handle_bus_station_stops,
                debug_bus_pathfinding,
                disable_passenger_pathfinding_system, // 新增：禁用乘客寻路
            )
                .chain()
                .run_if(in_state(GameStateEnum::Playing)),
        );
    }
}

// ============ 禁用乘客寻路系统 ============

fn disable_passenger_pathfinding_system(
    mut passengers: Query<&mut crate::bus_puzzle::PathfindingAgent>,
) {
    for mut agent in passengers.iter_mut() {
        // 将所有乘客设置为等车状态，不让他们自己寻路
        if matches!(agent.state, crate::bus_puzzle::AgentState::WaitingAtStation) {
            // 清空寻路路径，让乘客只能等车
            agent.current_path.clear();
            agent.current_step = 0;
        }
    }
}

// ============ 路线发现系统（基于寻路） ============

/// 检查站点之间是否已经有连接
fn has_station_connections(
    pathfinding_graph: &PathfindingGraph,
    stations: &Query<&StationEntity>,
) -> bool {
    let station_names: Vec<String> = stations
        .iter()
        .map(|s| s.station_data.name.clone())
        .collect();

    // 检查任意两个站点之间是否有路径
    for (i, start_station) in station_names.iter().enumerate() {
        for end_station in station_names.iter().skip(i + 1) {
            if let Some(path) = find_optimal_path(pathfinding_graph, start_station, end_station) {
                if path.len() > 1 {
                    info!(
                        "检测到站点连接: {} -> {} (路径长度: {})",
                        start_station,
                        end_station,
                        path.len()
                    );
                    return true;
                }
            }
        }
    }
    false
}

fn discover_bus_routes_pathfinding(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut bus_manager: ResMut<BusPathfindingManager>,
    pathfinding_graph: Res<PathfindingGraph>,
    _segments: Query<&RouteSegment>,
    stations: Query<&StationEntity>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    level_manager: Res<LevelManager>,
    existing_buses: Query<Entity, With<BusVehicle>>,
    game_state: Res<GameState>,
) {
    // 检查是否是教学关卡并且站点之间有连接
    let is_tutorial_level = game_state
        .current_level
        .as_ref()
        .map(|level| level.id == "tutorial_01")
        .unwrap_or(false);

    let should_auto_generate = if is_tutorial_level {
        // 教学关卡：检查是否已有连接且没有公交车
        existing_buses.is_empty() && has_station_connections(&pathfinding_graph, &stations)
    } else {
        false
    };

    // 手动触发 (F4) 或教学关卡自动触发
    if keyboard_input.just_pressed(KeyCode::F4) || should_auto_generate {
        if keyboard_input.just_pressed(KeyCode::F4) {
            info!("🚌 手动触发：使用寻路算法重新发现公交路线...");
        } else if should_auto_generate {
            info!("🚌 教学关卡：检测到站点连接，自动生成公交车...");
        }

        // 清理现有公交车
        for bus_entity in existing_buses.iter() {
            commands.entity(bus_entity).despawn();
        }

        // 清空现有数据
        bus_manager.bus_routes.clear();
        bus_manager.station_connections.clear();
        bus_manager.path_cache.clear();

        // 发现新路线
        let discovered_routes = discover_routes_using_pathfinding(&pathfinding_graph, stations);

        // 生成公交车
        for route_info in discovered_routes {
            if route_info.stations.len() >= 2 {
                spawn_pathfinding_bus(
                    &mut commands,
                    &asset_server,
                    &level_manager,
                    &route_info,
                    &pathfinding_graph,
                );

                bus_manager
                    .bus_routes
                    .insert(route_info.route_id.clone(), route_info);
            }
        }

        if keyboard_input.just_pressed(KeyCode::F4) {
            info!("手动路线发现完成: {} 条路线", bus_manager.bus_routes.len());
        } else if should_auto_generate {
            info!(
                "教学关卡公交车自动生成完成: {} 条路线",
                bus_manager.bus_routes.len()
            );
        }

        let routes: Vec<BusRouteInfo> = bus_manager.bus_routes.values().cloned().collect();
        check_passenger_coverage(&routes, &game_state)
    }
}

fn discover_routes_using_pathfinding(
    pathfinding_graph: &PathfindingGraph,
    stations: Query<&StationEntity>,
) -> Vec<BusRouteInfo> {
    let mut routes = Vec::new();
    let mut processed_pairs = HashSet::new();

    let station_list: Vec<_> = stations.iter().collect();

    // 为每对站点尝试创建路线
    for (i, start_station) in station_list.iter().enumerate() {
        for (j, end_station) in station_list.iter().enumerate() {
            if i >= j {
                continue; // 避免重复和自己到自己
            }

            let start_name = &start_station.station_data.name;
            let end_name = &end_station.station_data.name;

            // 避免重复处理相同的站点对
            let pair_key = if start_name < end_name {
                (start_name.clone(), end_name.clone())
            } else {
                (end_name.clone(), start_name.clone())
            };

            if processed_pairs.contains(&pair_key) {
                continue;
            }
            processed_pairs.insert(pair_key);

            // 使用寻路算法检查连通性
            if let Some(path) = find_optimal_path(pathfinding_graph, start_name, end_name) {
                if path.len() > 1 {
                    // 创建双向路线（往返服务）
                    let route_id = format!("智能路线_{}", routes.len() + 1);

                    // 检查路径中是否包含中转站，如果有则加入路线
                    let mut route_stations = vec![start_name.clone()];

                    // 添加路径中的中转站点
                    for path_node in &path[1..path.len() - 1] {
                        if let PathNodeType::Station(station_name) = &path_node.node_type {
                            if !route_stations.contains(station_name) {
                                route_stations.push(station_name.clone());
                            }
                        }
                    }

                    // 添加终点站
                    route_stations.push(end_name.clone());

                    let route_info = BusRouteInfo {
                        route_id: route_id.clone(),
                        stations: route_stations.clone(),
                        is_circular: false,
                        max_vehicles: 1,
                    };

                    info!(
                        "创建智能路线 {}: {:?} (路径长度: {})",
                        route_id,
                        route_stations,
                        path.len()
                    );

                    routes.push(route_info);
                }
            } else {
                info!(
                    "无法找到从 {} 到 {} 的路径，跳过创建路线",
                    start_name, end_name
                );
            }
        }
    }

    // 如果没有发现任何路线，创建一个包含所有站点的主干路线
    if routes.is_empty() {
        warn!("没有发现任何有效路线，尝试创建主干路线");

        let all_stations: Vec<String> = station_list
            .iter()
            .map(|s| s.station_data.name.clone())
            .collect();

        if all_stations.len() >= 2 {
            let route_info = BusRouteInfo {
                route_id: "主干路线".to_string(),
                stations: all_stations.clone(),
                is_circular: false,
                max_vehicles: 1,
            };

            info!("创建主干路线: {:?}", all_stations);
            routes.push(route_info);
        }
    }

    routes
}

// 同时需要添加这个函数来检查乘客需求覆盖率
fn check_passenger_coverage(routes: &[BusRouteInfo], game_state: &crate::bus_puzzle::GameState) {
    if let Some(level_data) = &game_state.current_level {
        info!("=== 乘客需求覆盖分析 ===");

        for demand in &level_data.passenger_demands {
            let mut covered = false;

            for route in routes {
                let has_origin = route.stations.contains(&demand.origin);
                let has_destination = route.stations.contains(&demand.destination);

                if has_origin && has_destination {
                    covered = true;
                    info!(
                        "✅ 乘客需求 {:?} {} -> {} 被路线 {} 覆盖",
                        demand.color, demand.origin, demand.destination, route.route_id
                    );
                    break;
                }
            }

            if !covered {
                warn!(
                    "❌ 乘客需求 {:?} {} -> {} 没有被任何路线覆盖！",
                    demand.color, demand.origin, demand.destination
                );
            }
        }
    }
}

/// 生成使用寻路算法的公交车
fn spawn_pathfinding_bus(
    commands: &mut Commands,
    asset_server: &AssetServer,
    level_manager: &LevelManager,
    route_info: &BusRouteInfo,
    pathfinding_graph: &PathfindingGraph,
) {
    if route_info.stations.is_empty() {
        return;
    }

    // 获取起始站点位置
    let start_station = &route_info.stations[0];
    if let Some(&start_pos) = pathfinding_graph.station_lookup.get(start_station) {
        let (grid_width, grid_height) = if let Some(level_data) = &level_manager.current_level {
            level_data.grid_size
        } else {
            (10, 8)
        };

        let spawn_world_pos =
            start_pos.to_world_pos(level_manager.tile_size, grid_width, grid_height)
                + Vec3::Z * (PASSENGER_Z + 0.1);

        // 生成路线颜色
        let route_colors = [
            Color::srgb(1.0, 0.2, 0.2), // 红色
            Color::srgb(0.2, 1.0, 0.2), // 绿色
            Color::srgb(0.2, 0.2, 1.0), // 蓝色
            Color::srgb(1.0, 1.0, 0.2), // 黄色
        ];
        let color_index = route_info.route_id.len() % route_colors.len();
        let route_color = route_colors[color_index];

        let vehicle_id = format!("智能公交_{}", route_info.route_id);

        // 🔧 修复：车辆生成时应该在起始站停靠，而不是立即前往下一站
        commands.spawn((
            Name::new(format!("Smart Bus {}", vehicle_id)),
            Sprite {
                image: asset_server.load("textures/bus.png"),
                color: route_color,
                custom_size: Some(Vec2::new(48.0, 48.0)),
                ..default()
            },
            Transform::from_translation(spawn_world_pos),
            BusVehicle {
                vehicle_id: vehicle_id.clone(),
                route_id: route_info.route_id.clone(),
                capacity: 30,
                current_passengers: Vec::new(),
                current_stop_index: 0,
                direction: BusDirection::Forward,
                // 🔧 关键修复：车辆生成时应该在站点停靠
                state: BusState::AtStop,
                speed: 80.0,
                dwell_time: 5.0, // 增加停靠时间，确保乘客有足够时间上车
                remaining_dwell: 5.0, // 初始停靠时间
                target_position: None,
            },
            BusPathfindingAgent {
                vehicle_id: vehicle_id.clone(),
                route_id: route_info.route_id.clone(),
                current_path: Vec::new(), // 🔧 修复：初始路径为空
                current_step: 0,
                target_station: start_station.clone(), // 🔧 修复：当前目标是起始站
                // 🔧 关键修复：车辆生成时应该在站点停靠
                state: BusPathfindingState::AtStation,
                path_progress: 0.0,
                next_station_index: 0, // 🔧 修复：从第0个站点开始
                stations_to_visit: route_info.stations.clone(),
                direction: BusDirection::Forward,
                is_returning: false,
            },
        ));

        info!(
            "🚌 生成智能公交车: {} 在起始站 {} 停靠中，准备载客",
            vehicle_id, start_station
        );
    }
}

// ============ 公交车寻路更新系统 ============

fn update_bus_pathfinding(
    mut buses: Query<(&mut BusPathfindingAgent, &mut BusVehicle)>,
    pathfinding_graph: Res<PathfindingGraph>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();

    for (mut agent, mut bus_vehicle) in buses.iter_mut() {
        match agent.state {
            BusPathfindingState::Planning => {
                // 规划新路径
                plan_next_route(&mut agent, &pathfinding_graph);
            }
            BusPathfindingState::Following => {
                // 跟随当前路径，无需额外处理，移动在另一个系统中处理
            }
            BusPathfindingState::AtStation => {
                // 🔧 修复：在站点停靠的处理逻辑
                bus_vehicle.remaining_dwell -= dt;

                // 🔧 新增：停靠期间的调试信息
                if bus_vehicle.remaining_dwell % 2.0 < dt {  // 每2秒打印一次
                    debug!(
                        "🚏 公交车 {} 在 {} 停靠中，剩余时间: {:.1}s，载客: {}/{}",
                        agent.vehicle_id,
                        agent.target_station,
                        bus_vehicle.remaining_dwell,
                        bus_vehicle.current_passengers.len(),
                        bus_vehicle.capacity
                    );
                }

                if bus_vehicle.remaining_dwell <= 0.0 {
                    // 🔧 修复：停靠结束后，更新站点索引
                    match agent.direction {
                        BusDirection::Forward => {
                            agent.next_station_index += 1;
                        }
                        BusDirection::Backward => {
                            agent.next_station_index = agent.next_station_index.saturating_sub(1);
                        }
                    }

                    agent.state = BusPathfindingState::Planning;
                    info!(
                        "🚌 公交车 {} 离开站点 {}，下一站索引: {}",
                        agent.vehicle_id, agent.target_station, agent.next_station_index
                    );
                }
            }
            BusPathfindingState::TurningAround => {
                // 调头：反转方向和站点访问顺序
                turn_around_pathfinding(&mut agent);
            }
            BusPathfindingState::WaitingForPath => {
                // 等待路径生成，可能需要重试
                if agent.current_path.is_empty() {
                    agent.state = BusPathfindingState::Planning;
                }
            }
        }
    }
}

/// 规划下一段路线
fn plan_next_route(agent: &mut BusPathfindingAgent, pathfinding_graph: &PathfindingGraph) {
    let current_station = agent.target_station.clone();

    // 🔧 修复：确定下一个目标站点
    let next_target = get_next_station_target_fixed(agent);

    if let Some(target) = next_target {
        // 使用乘客的寻路算法计算路径
        if let Some(path) = find_optimal_path(pathfinding_graph, &current_station, &target) {
            agent.current_path = path;
            agent.current_step = 0;
            agent.target_station = target.clone();
            agent.state = BusPathfindingState::Following;
            agent.path_progress = 0.0;

            info!(
                "🚌 公交车 {} 规划新路径: {} -> {} ({}步)",
                agent.vehicle_id,
                current_station,
                target,
                agent.current_path.len()
            );
        } else {
            warn!(
                "🚌 公交车 {} 无法找到从 {} 到 {} 的路径",
                agent.vehicle_id, current_station, target
            );
            agent.state = BusPathfindingState::WaitingForPath;
        }
    } else {
        // 没有下一个站点，可能需要调头
        agent.state = BusPathfindingState::TurningAround;
    }
}

/// 🔧 新增：修复后的下一站点获取逻辑
fn get_next_station_target_fixed(agent: &BusPathfindingAgent) -> Option<String> {
    let stations = &agent.stations_to_visit;

    if stations.is_empty() {
        return None;
    }

    match agent.direction {
        BusDirection::Forward => {
            // 🔧 修复：正确计算下一个站点索引
            let next_index = agent.next_station_index + 1;
            if next_index < stations.len() {
                Some(stations[next_index].clone())
            } else {
                None // 到达终点，需要调头
            }
        }
        BusDirection::Backward => {
            // 🔧 修复：反向行驶时的下一站点计算
            if agent.next_station_index > 0 {
                Some(stations[agent.next_station_index - 1].clone())
            } else {
                None // 回到起点，需要调头
            }
        }
    }
}

/// 获取下一个站点目标
fn get_next_station_target(agent: &BusPathfindingAgent) -> Option<String> {
    let stations = &agent.stations_to_visit;

    if stations.is_empty() {
        return None;
    }

    match agent.direction {
        BusDirection::Forward => {
            if agent.next_station_index < stations.len() {
                Some(stations[agent.next_station_index].clone())
            } else {
                None // 到达终点，需要调头
            }
        }
        BusDirection::Backward => {
            if agent.next_station_index > 0 {
                Some(stations[agent.next_station_index - 1].clone())
            } else {
                None // 回到起点，需要调头
            }
        }
    }
}

/// 🔧 修复：调头处理逻辑
fn turn_around_pathfinding(agent: &mut BusPathfindingAgent) {
    agent.direction = match agent.direction {
        BusDirection::Forward => {
            // 🔧 修复：正向到反向，应该从最后一个站点开始倒退
            agent.next_station_index = agent.stations_to_visit.len().saturating_sub(1);
            BusDirection::Backward
        }
        BusDirection::Backward => {
            // 🔧 修复：反向到正向，应该从第一个站点开始前进
            agent.next_station_index = 0;
            BusDirection::Forward
        }
    };

    agent.is_returning = !agent.is_returning;
    agent.state = BusPathfindingState::Planning;

    info!(
        "🔄 公交车 {} 调头完成，新方向: {:?}，当前站点索引: {}",
        agent.vehicle_id, agent.direction, agent.next_station_index
    );
}

// ============ 公交车移动系统 ============

fn move_buses_along_paths(
    mut buses: Query<(&mut BusPathfindingAgent, &mut Transform, &mut BusVehicle)>,
    level_manager: Res<LevelManager>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();

    for (mut agent, mut transform, mut bus_vehicle) in buses.iter_mut() {
        if agent.state != BusPathfindingState::Following || agent.current_path.is_empty() {
            continue;
        }

        if agent.current_step >= agent.current_path.len() {
            // 🔧 修复：到达路径终点时的处理
            agent.state = BusPathfindingState::AtStation;
            bus_vehicle.state = BusState::AtStop;
            bus_vehicle.remaining_dwell = bus_vehicle.dwell_time;

            info!(
                "🚏 公交车 {} 到达站点: {} (站点索引: {})",
                agent.vehicle_id, agent.target_station, agent.next_station_index
            );
            continue;
        }

        // 获取当前目标节点
        let current_node = &agent.current_path[agent.current_step];
        let (grid_width, grid_height) = if let Some(level_data) = &level_manager.current_level {
            level_data.grid_size
        } else {
            (10, 8)
        };

        let target_world_pos =
            current_node
                .position
                .to_world_pos(level_manager.tile_size, grid_width, grid_height);

        // 移动到目标位置
        let direction = (target_world_pos - transform.translation).normalize_or_zero();
        let distance_to_target = transform.translation.distance(target_world_pos);

        if distance_to_target > 8.0 {
            // 继续移动
            let movement = direction * bus_vehicle.speed * dt;
            transform.translation += movement;
            transform.translation.z = ROUTE_Z + 0.1;

            // 调整朝向
            if direction.length() > 0.1 {
                let angle = direction.y.atan2(direction.x);
                transform.rotation = Quat::from_rotation_z(angle);
            }
        } else {
            // 到达当前节点，移动到下一个节点
            transform.translation = target_world_pos;
            transform.translation.z = ROUTE_Z + 0.1;
            agent.current_step += 1;
            agent.path_progress = agent.current_step as f32 / agent.current_path.len() as f32;
        }

        // 更新公交车状态
        bus_vehicle.state = BusState::Traveling;
    }
}

// ============ 站点停靠处理 ============

fn handle_bus_station_stops(
    mut buses: Query<(&mut BusPathfindingAgent, &mut BusVehicle)>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();

    for (mut agent, mut bus_vehicle) in buses.iter_mut() {
        if agent.state == BusPathfindingState::AtStation {
            bus_vehicle.remaining_dwell -= dt;

            if bus_vehicle.remaining_dwell <= 0.0 {
                // 停靠结束，开始规划下一段路程
                agent.state = BusPathfindingState::Planning;
                bus_vehicle.state = BusState::Traveling;

                info!("公交车 {} 停靠结束，准备前往下一站", agent.vehicle_id);
            }
        }
    }
}

// ============ 调试系统 ============

fn debug_bus_pathfinding(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    buses: Query<(&BusPathfindingAgent, &Transform, &BusVehicle)>,
    bus_manager: Res<BusPathfindingManager>,
) {
    if keyboard_input.just_pressed(KeyCode::F5) {
        info!("=== 智能公交车寻路调试 ===");
        info!("智能路线数: {}", bus_manager.bus_routes.len());
        info!("公交车数: {}", buses.iter().count());

        for (route_id, route_info) in &bus_manager.bus_routes {
            info!(
                "智能路线 {}: {:?} ({}站)",
                route_id,
                route_info.stations,
                route_info.stations.len()
            );
        }

        for (agent, transform, bus_vehicle) in buses.iter() {
            info!("智能公交车 {} (路线: {})", agent.vehicle_id, agent.route_id);
            info!("  寻路状态: {:?}", agent.state);
            info!("  公交车状态: {:?}", bus_vehicle.state);
            info!("  方向: {:?}", agent.direction);
            info!("  当前目标: {}", agent.target_station);
            info!(
                "  路径进度: {}/{} ({:.1}%)",
                agent.current_step,
                agent.current_path.len(),
                agent.path_progress * 100.0
            );
            info!("  下一站索引: {}", agent.next_station_index);
            info!(
                "  位置: ({:.1}, {:.1})",
                transform.translation.x, transform.translation.y
            );
            info!(
                "  载客: {}/{}",
                bus_vehicle.current_passengers.len(),
                bus_vehicle.capacity
            );

            if !agent.current_path.is_empty() {
                info!("  当前路径:");
                for (i, node) in agent.current_path.iter().enumerate() {
                    let marker = if i == agent.current_step {
                        " -> "
                    } else {
                        "    "
                    };
                    let node_type = match &node.node_type {
                        PathNodeType::Station(name) => format!("站点:{}", name),
                        PathNodeType::RouteSegment => "路段".to_string(),
                        PathNodeType::TransferPoint => "换乘点".to_string(),
                    };
                    info!("{}步骤 {}: {:?} ({})", marker, i, node.position, node_type);
                }
            }
            info!(""); // 空行分隔
        }
    }
}
