// src/bus_puzzle/smart_bus_generation.rs
// 智能公交车生成系统

use crate::bus_puzzle::{
    find_optimal_path, BusPathfindingAgent, BusPathfindingManager, BusVehicle,
    GameStateEnum, LevelManager, PathfindingGraph, RouteSegment,
    SegmentPlacedEvent, SegmentRemovedEvent, StationEntity,
    PASSENGER_Z,
};
use bevy::prelude::*;
use std::collections::HashSet;

pub struct SmartBusGenerationPlugin;

impl Plugin for SmartBusGenerationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                auto_generate_buses_on_connection,
                manual_bus_generation_for_tutorial,
                cleanup_invalid_buses,
            )
                .chain()
                .run_if(in_state(GameStateEnum::Playing)),
        );
    }
}

/// 当路线连接发生变化时自动生成公交车
fn auto_generate_buses_on_connection(
    mut segment_placed_events: EventReader<SegmentPlacedEvent>,
    mut segment_removed_events: EventReader<SegmentRemovedEvent>,
    mut bus_manager: ResMut<BusPathfindingManager>,
    pathfinding_graph: Res<PathfindingGraph>,
    stations: Query<&StationEntity>,
    route_segments: Query<&RouteSegment>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    level_manager: Res<LevelManager>,
    existing_buses: Query<Entity, With<BusVehicle>>,
    mut last_trigger_time: Local<f32>,
    time: Res<Time>,
) {
    let has_route_changes = !segment_placed_events.is_empty() || !segment_removed_events.is_empty();

    // 清空事件读取器
    segment_placed_events.clear();
    segment_removed_events.clear();

    // 防止频繁重生成：最少间隔2秒
    if has_route_changes && (time.elapsed_secs() - *last_trigger_time) > 2.0 {
        info!("🔄 检测到路线变化，重新生成公交车系统...");

        // 检查是否有有效的站点连接
        let connected_stations = analyze_station_connectivity(&pathfinding_graph, &stations);

        if connected_stations.len() >= 2 {
            // 清理现有公交车
            for bus_entity in existing_buses.iter() {
                commands.entity(bus_entity).despawn();
            }

            // 重新生成公交车
            generate_smart_bus_routes(
                &mut commands,
                &asset_server,
                &level_manager,
                &mut bus_manager,
                &pathfinding_graph,
                &stations,
                &route_segments,
            );

            *last_trigger_time = time.elapsed_secs();
            info!("✅ 公交车系统重新生成完成");
        } else {
            info!("❌ 连接的站点不足，暂不生成公交车");
        }
    }
}

/// 教学关卡的手动公交车生成
fn manual_bus_generation_for_tutorial(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    level_manager: Res<LevelManager>,
    mut bus_manager: ResMut<BusPathfindingManager>,
    pathfinding_graph: Res<PathfindingGraph>,
    stations: Query<&StationEntity>,
    route_segments: Query<&RouteSegment>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    existing_buses: Query<Entity, With<BusVehicle>>,
) {
    // 在教学关卡中，按空格键手动生成公交车
    if keyboard_input.just_pressed(KeyCode::Space) {
        if let Some(level_data) = &level_manager.current_level {
            if level_data.id == "tutorial_01" {
                info!("🎓 教学关卡：手动启动公交系统");

                // 清理现有公交车
                for bus_entity in existing_buses.iter() {
                    commands.entity(bus_entity).despawn();
                }

                // 生成公交车
                generate_smart_bus_routes(
                    &mut commands,
                    &asset_server,
                    &level_manager,
                    &mut bus_manager,
                    &pathfinding_graph,
                    &stations,
                    &route_segments,
                );

                info!("✅ 教学关卡公交系统启动完成");
            }
        }
    }
}

/// 清理无效的公交车（路线不再可达）
fn cleanup_invalid_buses(
    mut commands: Commands,
    buses: Query<(Entity, &BusPathfindingAgent)>,
    pathfinding_graph: Res<PathfindingGraph>,
    mut cleanup_timer: Local<f32>,
    time: Res<Time>,
) {
    *cleanup_timer += time.delta_secs();

    // 每10秒检查一次
    if *cleanup_timer > 10.0 {
        *cleanup_timer = 0.0;

        for (entity, bus_agent) in buses.iter() {
            // 检查公交车的路线是否仍然有效
            let mut valid_route = true;

            for i in 0..bus_agent.stations_to_visit.len().saturating_sub(1) {
                let from = &bus_agent.stations_to_visit[i];
                let to = &bus_agent.stations_to_visit[i + 1];

                if find_optimal_path(&pathfinding_graph, from, to).is_none() {
                    valid_route = false;
                    break;
                }
            }

            if !valid_route {
                info!("🗑️ 清理无效公交车: {}", bus_agent.vehicle_id);
                commands.entity(entity).despawn();
            }
        }
    }
}

/// 分析站点连通性
fn analyze_station_connectivity(
    pathfinding_graph: &PathfindingGraph,
    stations: &Query<&StationEntity>,
) -> Vec<String> {
    let mut connected_stations = Vec::new();
    let station_names: Vec<_> = stations.iter().map(|s| s.station_data.name.clone()).collect();

    for station_name in &station_names {
        if let Some(&station_pos) = pathfinding_graph.station_lookup.get(station_name) {
            // 检查是否有连接到其他节点
            if pathfinding_graph.connections.contains_key(&station_pos) {
                connected_stations.push(station_name.clone());
            }
        }
    }

    connected_stations
}

/// 智能生成公交路线
fn generate_smart_bus_routes(
    commands: &mut Commands,
    asset_server: &AssetServer,
    level_manager: &LevelManager,
    bus_manager: &mut BusPathfindingManager,
    pathfinding_graph: &PathfindingGraph,
    stations: &Query<&StationEntity>,
    _route_segments: &Query<&RouteSegment>,
) {
    bus_manager.bus_routes.clear();

    let station_list: Vec<_> = stations.iter().collect();
    let mut processed_stations = HashSet::new();

    info!("🧠 开始智能路线分析...");

    for (i, start_station) in station_list.iter().enumerate() {
        let start_name = &start_station.station_data.name;

        if processed_stations.contains(start_name) {
            continue;
        }

        // 寻找从当前站点可达的其他站点
        let mut route_stations = vec![start_name.clone()];
        let mut current_station = start_name;

        for end_station in station_list.iter().skip(i + 1) {
            let end_name = &end_station.station_data.name;

            if processed_stations.contains(end_name) {
                continue;
            }

            // 使用寻路算法检查连通性
            if let Some(path) = find_optimal_path(pathfinding_graph, current_station, end_name) {
                if path.len() > 1 {
                    route_stations.push(end_name.clone());
                    current_station = end_name;

                    info!("📍 发现连接: {} -> {}",
                          route_stations[route_stations.len() - 2], end_name);

                    // 如果路线足够长，可以创建公交车
                    if route_stations.len() >= 2 {
                        break;
                    }
                }
            }
        }

        // 创建公交路线和车辆
        if route_stations.len() >= 2 {
            let route_id = format!("智能路线_{}", bus_manager.bus_routes.len() + 1);

            spawn_smart_bus(
                commands,
                asset_server,
                level_manager,
                pathfinding_graph,
                &route_id,
                &route_stations,
            );

            // 标记这些站点为已处理
            for station_name in &route_stations {
                processed_stations.insert(station_name.clone());
            }

            info!("🚌 创建公交路线 {}: {:?}", route_id, route_stations);
        }
    }

    info!("✅ 智能公交系统生成完成");
}

/// 生成智能公交车
fn spawn_smart_bus(
    commands: &mut Commands,
    asset_server: &AssetServer,
    level_manager: &LevelManager,
    pathfinding_graph: &PathfindingGraph,
    route_id: &str,
    stations: &[String],
) {
    if stations.is_empty() {
        return;
    }

    let start_station = &stations[0];
    if let Some(&start_pos) = pathfinding_graph.station_lookup.get(start_station) {
        let (grid_width, grid_height) = if let Some(level_data) = &level_manager.current_level {
            level_data.grid_size
        } else {
            (10, 8)
        };

        let spawn_world_pos = start_pos.to_world_pos(
            level_manager.tile_size,
            grid_width,
            grid_height,
        ) + Vec3::Z * (PASSENGER_Z + 0.1);

        // 生成路线颜色
        let route_colors = [
            Color::srgb(1.0, 0.2, 0.2), // 红色
            Color::srgb(0.2, 1.0, 0.2), // 绿色
            Color::srgb(0.2, 0.2, 1.0), // 蓝色
            Color::srgb(1.0, 1.0, 0.2), // 黄色
        ];
        let color_index = route_id.len() % route_colors.len();
        let route_color = route_colors[color_index];

        let vehicle_id = format!("智能公交_{}", route_id);

        // 生成初始路径
        let initial_target = if stations.len() > 1 {
            stations[1].clone()
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
                route_id: route_id.to_string(),
                capacity: 30,
                current_passengers: Vec::new(),
                current_stop_index: 0,
                direction: crate::bus_puzzle::BusDirection::Forward,
                state: crate::bus_puzzle::BusState::Traveling,
                speed: 80.0,
                dwell_time: 3.0,
                remaining_dwell: 0.0,
                target_position: None,
            },
            BusPathfindingAgent {
                vehicle_id: vehicle_id.clone(),
                route_id: route_id.to_string(),
                current_path: initial_path,
                current_step: 0,
                target_station: initial_target,
                state: crate::bus_puzzle::BusPathfindingState::Following,
                path_progress: 0.0,
                next_station_index: 1,
                stations_to_visit: stations.to_vec(),
                direction: crate::bus_puzzle::BusDirection::Forward,
                is_returning: false,
            },
        ));

        info!("🚌 生成智能公交车: {} 路线: {}", vehicle_id, route_id);
    }
}
