// src/bus_puzzle/bus_pathfinding_system.rs - 公交车智能寻路系统
// 使用乘客验证过的寻路算法来驱动公交车移动

use crate::bus_puzzle::{
    find_optimal_path, BusDirection, BusState, BusVehicle, GameStateEnum, LevelManager, PathNode,
    PathNodeType, PathfindingGraph, RouteSegment, StationEntity, PASSENGER_Z, ROUTE_Z,
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
) {
    if keyboard_input.just_pressed(KeyCode::F4) {
        info!("🚌 使用寻路算法重新发现公交路线...");

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

        info!("智能路线发现完成: {} 条路线", bus_manager.bus_routes.len());
    }
}

/// 使用寻路图发现公交路线
fn discover_routes_using_pathfinding(
    pathfinding_graph: &PathfindingGraph,
    stations: Query<&StationEntity>,
) -> Vec<BusRouteInfo> {
    let mut routes = Vec::new();
    let mut processed_stations = HashSet::new();

    let station_list: Vec<_> = stations.iter().collect();

    for (i, start_station) in station_list.iter().enumerate() {
        let start_name = &start_station.station_data.name;

        if processed_stations.contains(start_name) {
            continue;
        }

        // 尝试找到一条连接多个站点的路线
        let mut route_stations = vec![start_name.clone()];
        let mut current_station = start_name;

        // 寻找可达的其他站点
        for end_station in station_list.iter().skip(i + 1) {
            let end_name = &end_station.station_data.name;

            if processed_stations.contains(end_name) || current_station == end_name {
                continue;
            }

            // 使用寻路算法检查连通性
            if let Some(path) = find_optimal_path(pathfinding_graph, current_station, end_name) {
                if path.len() > 1 {
                    // 找到有效路径
                    route_stations.push(end_name.clone());
                    current_station = end_name;

                    info!(
                        "找到连接: {} -> {} (路径长度: {})",
                        route_stations[route_stations.len() - 2],
                        end_name,
                        path.len()
                    );

                    // 如果已经有足够的站点，可以创建路线
                    if route_stations.len() >= 2 {
                        break;
                    }
                }
            }
        }

        // 如果找到了有效路线
        if route_stations.len() >= 2 {
            let route_id = format!("智能路线_{}", routes.len() + 1);
            let route_info = BusRouteInfo {
                route_id: route_id.clone(),
                stations: route_stations.clone(),
                is_circular: false,
                max_vehicles: 1,
            };

            info!("创建智能路线 {}: {:?}", route_id, route_stations);

            // 标记这些站点为已处理
            for station_name in &route_stations {
                processed_stations.insert(station_name.clone());
            }

            routes.push(route_info);
        }
    }

    routes
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

        // 生成到第二个站点的初始路径
        let initial_target = if route_info.stations.len() > 1 {
            route_info.stations[1].clone()
        } else {
            start_station.clone()
        };

        let initial_path = find_optimal_path(pathfinding_graph, start_station, &initial_target)
            .unwrap_or_default();

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
                state: BusState::Traveling,
                speed: 80.0,
                dwell_time: 3.0,
                remaining_dwell: 0.0,
                target_position: None,
            },
            BusPathfindingAgent {
                vehicle_id: vehicle_id.clone(),
                route_id: route_info.route_id.clone(),
                current_path: initial_path,
                current_step: 0,
                target_station: initial_target,
                state: BusPathfindingState::Following,
                path_progress: 0.0,
                next_station_index: 1,
                stations_to_visit: route_info.stations.clone(),
                direction: BusDirection::Forward,
                is_returning: false,
            },
        ));

        info!(
            "生成智能公交车: {} 路线: {} -> {}",
            vehicle_id,
            start_station,
            route_info.stations.get(1).unwrap_or(&"终点".to_string())
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
                // 在站点停靠，等待一段时间后继续
                bus_vehicle.remaining_dwell -= dt;
                if bus_vehicle.remaining_dwell <= 0.0 {
                    agent.state = BusPathfindingState::Planning;
                    info!(
                        "公交车 {} 离开站点 {}",
                        agent.vehicle_id, agent.target_station
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

    // 确定下一个目标站点
    let next_target = get_next_station_target(agent);

    if let Some(target) = next_target {
        // 使用乘客的寻路算法计算路径
        if let Some(path) = find_optimal_path(pathfinding_graph, &current_station, &target) {
            agent.current_path = path;
            agent.current_step = 0;
            agent.target_station = target.clone();
            agent.state = BusPathfindingState::Following;
            agent.path_progress = 0.0;

            info!(
                "公交车 {} 规划新路径: {} -> {} ({}步)",
                agent.vehicle_id,
                current_station,
                target,
                agent.current_path.len()
            );
        } else {
            warn!(
                "公交车 {} 无法找到从 {} 到 {} 的路径",
                agent.vehicle_id, current_station, target
            );
            agent.state = BusPathfindingState::WaitingForPath;
        }
    } else {
        // 没有下一个站点，可能需要调头
        agent.state = BusPathfindingState::TurningAround;
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

/// 调头处理
fn turn_around_pathfinding(agent: &mut BusPathfindingAgent) {
    agent.direction = match agent.direction {
        BusDirection::Forward => {
            agent.next_station_index = agent.stations_to_visit.len().saturating_sub(1);
            BusDirection::Backward
        }
        BusDirection::Backward => {
            agent.next_station_index = 1;
            BusDirection::Forward
        }
    };

    agent.is_returning = !agent.is_returning;
    agent.state = BusPathfindingState::Planning;

    info!(
        "公交车 {} 调头，新方向: {:?}，下一站索引: {}",
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
            // 到达路径终点
            agent.state = BusPathfindingState::AtStation;
            bus_vehicle.state = BusState::AtStop;
            bus_vehicle.remaining_dwell = bus_vehicle.dwell_time;

            // 更新下一站索引
            match agent.direction {
                BusDirection::Forward => {
                    agent.next_station_index += 1;
                }
                BusDirection::Backward => {
                    agent.next_station_index = agent.next_station_index.saturating_sub(1);
                }
            }

            info!(
                "公交车 {} 到达站点: {}",
                agent.vehicle_id, agent.target_station
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
