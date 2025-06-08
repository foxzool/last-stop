// src/bus_puzzle/bus_system.rs - 公交车系统核心实现

use crate::bus_puzzle::{GridPos, LevelManager, RouteSegment, StationEntity, PASSENGER_Z, ROUTE_Z};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============ 公交车实体组件 ============

#[derive(Component, Debug, Clone)]
#[allow(dead_code)]
pub struct BusVehicle {
    pub vehicle_id: String,
    pub route_id: String,
    pub capacity: u32,
    pub current_passengers: Vec<Entity>,
    pub current_stop_index: usize,
    pub direction: BusDirection,
    pub state: BusState,
    pub speed: f32,
    pub dwell_time: f32,
    pub remaining_dwell: f32,
    pub target_position: Option<Vec3>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BusDirection {
    Forward,  // 正向运行
    Backward, // 反向运行
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BusState {
    Traveling,     // 行驶中
    AtStop,        // 停站中
    Loading,       // 上下客
    TurningAround, // 终点调头
    Idle,          // 空闲状态
}

impl Default for BusVehicle {
    fn default() -> Self {
        Self {
            vehicle_id: "bus_001".to_string(),
            route_id: "route_001".to_string(),
            capacity: 30,
            current_passengers: Vec::new(),
            current_stop_index: 0,
            direction: BusDirection::Forward,
            state: BusState::Idle,
            speed: 80.0,     // 像素/秒
            dwell_time: 3.0, // 停站3秒
            remaining_dwell: 0.0,
            target_position: None,
        }
    }
}

// ============ 公交路线定义 ============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusRoute {
    pub route_id: String,
    pub route_name: String,
    pub stops: Vec<BusStop>,
    pub segments: Vec<GridPos>,
    pub frequency: f32, // 发车间隔(秒)
    pub is_circular: bool,
    pub vehicles: Vec<Entity>,
    pub max_vehicles: u32,
    pub color: Color,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusStop {
    pub position: GridPos,
    pub name: String,
    pub waiting_passengers: Vec<Entity>,
    pub platform_capacity: u32,
}

// ============ 路线发现和管理 ============

#[derive(Resource, Default)]
#[allow(dead_code)]
pub struct BusRoutesManager {
    pub routes: HashMap<String, BusRoute>,
    pub route_counter: u32,
    pub vehicle_counter: u32,
}

impl BusRoutesManager {
    #[allow(dead_code)]
    pub fn generate_route_id(&mut self) -> String {
        self.route_counter += 1;
        format!("route_{:03}", self.route_counter)
    }

    #[allow(dead_code)]
    pub fn generate_vehicle_id(&mut self) -> String {
        self.vehicle_counter += 1;
        format!("bus_{:03}", self.vehicle_counter)
    }

    pub fn add_route(&mut self, route: BusRoute) {
        self.routes.insert(route.route_id.clone(), route);
    }

    pub fn get_route(&self, route_id: &str) -> Option<&BusRoute> {
        self.routes.get(route_id)
    }

    #[allow(dead_code)]
    pub fn get_route_mut(&mut self, route_id: &str) -> Option<&mut BusRoute> {
        self.routes.get_mut(route_id)
    }
}

// ============ 路线自动发现系统 ============

pub struct RouteDiscoverySystem;

impl RouteDiscoverySystem {
    /// 从放置的路线段和站点自动识别公交路线
    pub fn discover_routes(
        segments: &Query<&RouteSegment>,
        stations: &Query<&StationEntity>,
    ) -> Vec<BusRoute> {
        let mut discovered_routes = Vec::new();
        let mut processed_segments = std::collections::HashSet::new();

        // 收集所有活跃的路线段
        let active_segments: Vec<_> = segments
            .iter()
            .filter(|segment| segment.is_active)
            .collect();

        // 收集所有站点
        let station_positions: HashMap<GridPos, String> = stations
            .iter()
            .map(|station| {
                (
                    station.station_data.position,
                    station.station_data.name.clone(),
                )
            })
            .collect();

        info!(
            "开始路线发现: {} 个活跃路线段, {} 个站点",
            active_segments.len(),
            station_positions.len()
        );

        // 从每个站点开始尝试构建路线
        for station_entity in stations.iter() {
            let start_pos = station_entity.station_data.position;

            if let Some(route) = Self::build_route_from_station(
                start_pos,
                &active_segments,
                &station_positions,
                &mut processed_segments,
            ) {
                info!(
                    "发现路线: {} ({}个站点)",
                    route.route_name,
                    route.stops.len()
                );
                discovered_routes.push(route);
            }
        }

        info!("路线发现完成: 共发现 {} 条路线", discovered_routes.len());
        discovered_routes
    }

    /// 从指定站点开始构建路线
    fn build_route_from_station(
        start_pos: GridPos,
        segments: &[&RouteSegment],
        stations: &HashMap<GridPos, String>,
        processed: &mut std::collections::HashSet<GridPos>,
    ) -> Option<BusRoute> {
        // 检查起点是否已经被处理过
        if processed.contains(&start_pos) {
            return None;
        }

        let mut route_segments = Vec::new();
        let mut route_stops = Vec::new();
        let mut visited = std::collections::HashSet::new();

        // 添加起始站点
        if let Some(station_name) = stations.get(&start_pos) {
            route_stops.push(BusStop {
                position: start_pos,
                name: station_name.clone(),
                waiting_passengers: Vec::new(),
                platform_capacity: 20,
            });
            info!("开始构建从 {} ({:?}) 的路线", station_name, start_pos);
        } else {
            return None;
        }

        // 查找从起始站点连接的路线段
        let mut current_pos = start_pos;
        let mut found_segments = Vec::new();

        // 首先找到与起始站点相邻的路线段
        for segment in segments {
            if Self::is_adjacent(current_pos, segment.grid_pos) {
                info!("找到与起始站点相邻的路线段: {:?}", segment.grid_pos);
                found_segments.push(segment.grid_pos);
            }
        }

        if found_segments.is_empty() {
            info!("起始站点 {:?} 没有连接的路线段", start_pos);
            return None;
        }

        // 选择一个起始路线段
        current_pos = found_segments[0];
        route_segments.push(current_pos);
        visited.insert(start_pos);
        visited.insert(current_pos);

        info!("开始路线段: {:?}", current_pos);

        // 沿着连通的路线段前进，寻找更多站点
        loop {
            let mut found_next = false;

            // 查找与当前位置相邻且未访问的路线段
            for segment in segments {
                if !visited.contains(&segment.grid_pos)
                    && Self::is_adjacent(current_pos, segment.grid_pos)
                {
                    route_segments.push(segment.grid_pos);
                    visited.insert(segment.grid_pos);
                    current_pos = segment.grid_pos;
                    found_next = true;

                    info!("添加路线段: {:?}", segment.grid_pos);
                    break;
                }
            }

            // 检查当前位置是否有站点
            if let Some(station_name) = stations.get(&current_pos) {
                if !route_stops.iter().any(|stop| stop.position == current_pos) {
                    route_stops.push(BusStop {
                        position: current_pos,
                        name: station_name.clone(),
                        waiting_passengers: Vec::new(),
                        platform_capacity: 20,
                    });
                    info!("添加站点: {} ({:?})", station_name, current_pos);
                }
            }

            if !found_next {
                break;
            }
        }

        // 检查路线段末端是否有相邻的站点
        for (station_pos, station_name) in stations {
            if Self::is_adjacent(current_pos, *station_pos)
                && !route_stops.iter().any(|stop| stop.position == *station_pos)
            {
                route_stops.push(BusStop {
                    position: *station_pos,
                    name: station_name.clone(),
                    waiting_passengers: Vec::new(),
                    platform_capacity: 20,
                });
                info!("添加终点站点: {} ({:?})", station_name, station_pos);
                break;
            }
        }

        // 标记所有访问过的位置为已处理
        for &pos in &visited {
            processed.insert(pos);
        }

        info!(
            "路线构建完成: {} 个站点, {} 个路线段",
            route_stops.len(),
            route_segments.len()
        );

        // 输出路线详情
        for (i, stop) in route_stops.iter().enumerate() {
            info!("  站点 {}: {} at {:?}", i, stop.name, stop.position);
        }

        // 只有包含至少2个站点的路线才有效
        if route_stops.len() < 2 {
            info!("路线站点不足 ({}个)，需要至少2个", route_stops.len());
            return None;
        }

        // 生成路线颜色
        let route_colors = [
            Color::srgb(1.0, 0.2, 0.2), // 红色
            Color::srgb(0.2, 1.0, 0.2), // 绿色
            Color::srgb(0.2, 0.2, 1.0), // 蓝色
            Color::srgb(1.0, 1.0, 0.2), // 黄色
            Color::srgb(1.0, 0.2, 1.0), // 紫色
            Color::srgb(0.2, 1.0, 1.0), // 青色
        ];
        let route_color = route_colors[route_stops.len() % route_colors.len()];

        let route_id = format!("route_{}", route_stops[0].name);
        let route_name = format!("{}路", route_stops[0].name);

        info!(
            "成功构建路线: {} ({} 个站点)",
            route_name,
            route_stops.len()
        );

        Some(BusRoute {
            route_id: route_id.clone(),
            route_name,
            stops: route_stops,
            segments: route_segments,
            frequency: 20.0,    // 默认20秒一班
            is_circular: false, // 目前都是往返线路
            vehicles: Vec::new(),
            max_vehicles: 2,
            color: route_color,
        })
    }

    /// 检查两个位置是否相邻
    fn is_adjacent(pos1: GridPos, pos2: GridPos) -> bool {
        let dx = (pos1.x - pos2.x).abs();
        let dy = (pos1.y - pos2.y).abs();
        (dx + dy) == 1 // 曼哈顿距离为1
    }

    /// 检查从pos1是否可以连接到segment
    fn can_connect(pos1: GridPos, segment: &RouteSegment) -> bool {
        // 简化版本：检查路线段是否有朝向pos1的连接端口
        segment
            .segment_type
            .has_connection_to(segment.grid_pos, pos1, segment.rotation)
    }
}

// ============ 公交车移动系统 ============

pub fn update_bus_movement(
    mut buses: Query<(&mut BusVehicle, &mut Transform)>,
    routes_manager: Res<BusRoutesManager>,
    time: Res<Time>,
    level_manager: Res<LevelManager>,
) {
    let dt = time.delta_secs();

    for (mut bus, mut transform) in buses.iter_mut() {
        // 获取公交车所属路线
        let route = match routes_manager.get_route(&bus.route_id) {
            Some(route) => route,
            None => {
                warn!("找不到路线: {}", bus.route_id);
                continue;
            }
        };

        match bus.state {
            BusState::Traveling => {
                handle_bus_traveling(&mut bus, &mut transform, route, &level_manager, dt);
            }
            BusState::AtStop => {
                handle_bus_at_stop(&mut bus, route, dt);
            }
            BusState::TurningAround => {
                handle_bus_turning_around(&mut bus, route);
            }
            BusState::Idle => {
                // 空闲状态，等待调度
            }
            BusState::Loading => {
                // 上下客状态，后续实现
                bus.state = BusState::AtStop;
            }
        }
    }
}

/// 处理公交车行驶状态
fn handle_bus_traveling(
    bus: &mut BusVehicle,
    transform: &mut Transform,
    route: &BusRoute,
    level_manager: &LevelManager,
    dt: f32,
) {
    if bus.current_stop_index >= route.stops.len() {
        warn!("公交车 {} 的站点索引超出范围", bus.vehicle_id);
        return;
    }

    let target_stop = &route.stops[bus.current_stop_index];
    let (grid_width, grid_height) = if let Some(level_data) = &level_manager.current_level {
        level_data.grid_size
    } else {
        (10, 8)
    };

    let target_world_pos =
        target_stop
            .position
            .to_world_pos(level_manager.tile_size, grid_width, grid_height);

    // 计算移动方向
    let direction = (target_world_pos - transform.translation).normalize_or_zero();
    let distance_to_target = transform.translation.distance(target_world_pos);

    if distance_to_target > 8.0 {
        // 继续移动
        let movement = direction * bus.speed * dt;
        transform.translation += movement;
        transform.translation.z = ROUTE_Z + 0.1; // 确保公交车在路线段之上

        // 调整公交车朝向
        if direction.length() > 0.1 {
            let angle = direction.y.atan2(direction.x);
            transform.rotation = Quat::from_rotation_z(angle);
        }
    } else {
        // 到达站点
        transform.translation = target_world_pos + Vec3::Z * (ROUTE_Z + 0.1);
        bus.state = BusState::AtStop;
        bus.remaining_dwell = bus.dwell_time;

        info!("公交车 {} 到达站点: {}", bus.vehicle_id, target_stop.name);
    }
}

/// 处理公交车停站状态
fn handle_bus_at_stop(bus: &mut BusVehicle, route: &BusRoute, dt: f32) {
    bus.remaining_dwell -= dt;

    if bus.remaining_dwell <= 0.0 {
        // 停站结束，前往下一站
        advance_to_next_stop(bus, route);
        info!("公交车 {} 离开站点，前往下一站", bus.vehicle_id);
        bus.state = BusState::Traveling;
    }
}

/// 处理公交车调头状态
fn handle_bus_turning_around(bus: &mut BusVehicle, route: &BusRoute) {
    // 改变行驶方向
    bus.direction = match bus.direction {
        BusDirection::Forward => {
            bus.current_stop_index = route.stops.len().saturating_sub(1);
            BusDirection::Backward
        }
        BusDirection::Backward => {
            bus.current_stop_index = 0;
            BusDirection::Forward
        }
    };

    // 移动到下一站
    advance_to_next_stop(bus, route);
    bus.state = BusState::Traveling;

    info!(
        "公交车 {} 调头，新方向: {:?}",
        bus.vehicle_id, bus.direction
    );
}

/// 前进到下一站
fn advance_to_next_stop(bus: &mut BusVehicle, route: &BusRoute) {
    match bus.direction {
        BusDirection::Forward => {
            if bus.current_stop_index + 1 >= route.stops.len() {
                if route.is_circular {
                    bus.current_stop_index = 0;
                } else {
                    bus.state = BusState::TurningAround;
                    return;
                }
            } else {
                bus.current_stop_index += 1;
            }
        }
        BusDirection::Backward => {
            if bus.current_stop_index == 0 {
                if route.is_circular {
                    bus.current_stop_index = route.stops.len() - 1;
                } else {
                    bus.state = BusState::TurningAround;
                    return;
                }
            } else {
                bus.current_stop_index -= 1;
            }
        }
    }
}

// ============ 公交车生成系统 ============

pub fn spawn_bus_vehicle(
    commands: &mut Commands,
    asset_server: &AssetServer,
    route_id: String,
    spawn_position: Vec3,
    route_color: Color,
) -> Entity {
    // 使用时间戳生成唯一ID
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let vehicle_id = format!("bus_{}", timestamp % 10000);

    commands
        .spawn((
            Name::new(format!("Bus {}", vehicle_id)),
            Sprite {
                image: asset_server.load("textures/bus.png"),
                color: route_color,
                custom_size: Some(Vec2::new(48.0, 48.0)),
                ..default()
            },
            Transform::from_translation(spawn_position),
            BusVehicle {
                vehicle_id: vehicle_id.clone(),
                route_id,
                capacity: 30,
                current_passengers: Vec::new(),
                current_stop_index: 0,
                direction: BusDirection::Forward,
                state: BusState::Idle,
                speed: 80.0,
                dwell_time: 3.0,
                remaining_dwell: 0.0,
                target_position: None,
            },
        ))
        .id()
}

// ============ 路线更新系统 ============

pub fn update_bus_routes(
    mut routes_manager: ResMut<BusRoutesManager>,
    segments: Query<&RouteSegment>,
    stations: Query<&StationEntity>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    level_manager: Res<LevelManager>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    // F4 - 手动重新发现路线并生成公交车
    if keyboard_input.just_pressed(KeyCode::F4) {
        info!("🚌 手动重新发现路线...");

        // 清空现有路线
        routes_manager.routes.clear();

        // 重新发现路线
        let discovered_routes = RouteDiscoverySystem::discover_routes(&segments, &stations);

        // 添加到管理器
        for mut route in discovered_routes {
            let route_id = route.route_id.clone();
            let route_color = route.color;

            // 为每条路线生成一辆公交车
            if !route.stops.is_empty() {
                let first_stop = &route.stops[0];
                let (grid_width, grid_height) =
                    if let Some(level_data) = &level_manager.current_level {
                        level_data.grid_size
                    } else {
                        (10, 8)
                    };

                let spawn_pos = first_stop.position.to_world_pos(
                    level_manager.tile_size,
                    grid_width,
                    grid_height,
                ) + Vec3::Z * (PASSENGER_Z + 0.1);

                let bus_entity = spawn_bus_vehicle(
                    &mut commands,
                    &asset_server,
                    route_id.clone(),
                    spawn_pos,
                    route_color,
                );

                route.vehicles.push(bus_entity);
                info!("为路线 {} 生成公交车: {:?}", route.route_name, bus_entity);
            }

            routes_manager.add_route(route);
        }

        info!("路线发现完成: {} 条路线", routes_manager.routes.len());

        // 显示路线详情
        for (route_id, route) in &routes_manager.routes {
            info!("路线 {}: {}", route_id, route.route_name);
            for (i, stop) in route.stops.iter().enumerate() {
                info!("  站点 {}: {} {:?}", i, stop.name, stop.position);
            }
        }
    }
}

// ============ 调试系统 ============

pub fn debug_bus_system(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    buses: Query<(&BusVehicle, &Transform)>,
    routes_manager: Res<BusRoutesManager>,
) {
    if keyboard_input.just_pressed(KeyCode::F5) {
        info!("=== 公交车系统调试 ===");
        info!("当前路线数: {}", routes_manager.routes.len());
        info!("当前公交车数: {}", buses.iter().count());

        for (route_id, route) in &routes_manager.routes {
            info!("路线 {}: {}", route_id, route.route_name);
            info!("  站点数: {}", route.stops.len());
            info!("  车辆数: {}", route.vehicles.len());
            info!("  发车间隔: {:.1}秒", route.frequency);

            for stop in &route.stops {
                info!("    站点: {} {:?}", stop.name, stop.position);
            }
        }

        for (bus, transform) in buses.iter() {
            info!("公交车 {} (路线: {})", bus.vehicle_id, bus.route_id);
            info!("  状态: {:?}", bus.state);
            info!("  方向: {:?}", bus.direction);
            info!("  当前站点索引: {}", bus.current_stop_index);
            info!(
                "  位置: ({:.1}, {:.1})",
                transform.translation.x, transform.translation.y
            );
            info!("  载客: {}/{}", bus.current_passengers.len(), bus.capacity);
        }
    }
}
