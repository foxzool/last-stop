// src/bus_puzzle/connection_system.rs

use crate::bus_puzzle::{
    manhattan_distance, Connection, ConnectionType, GraphNode, GraphNodeType, GridPos,
    PathfindingGraph, RouteSegment, RouteSegmentType, StationEntity,
};
use bevy::prelude::*;

/// 修复后的连接系统 - 正确处理方向性
pub struct FixedConnectionSystemPlugin;

impl Plugin for FixedConnectionSystemPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                debug_connections_with_directions,
                force_rebuild_connections_fixed,
                visualize_segment_directions,
            ),
        );
    }
}

/// F8 - 调试连接和方向
fn debug_connections_with_directions(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    pathfinding_graph: Res<PathfindingGraph>,
    stations: Query<&StationEntity>,
    route_segments: Query<&RouteSegment>,
) {
    if keyboard_input.just_pressed(KeyCode::F8) {
        info!("=== 方向性连接调试 ===");

        // 显示每个路线段的详细方向信息
        for segment in route_segments.iter() {
            if !segment.is_active {
                continue;
            }

            info!(
                "🛤️  {:?} at {:?} 旋转: {}°",
                segment.segment_type, segment.grid_pos, segment.rotation
            );

            // 显示该路线段的连接端口
            let connection_ports = get_segment_connection_ports(
                segment.grid_pos,
                &segment.segment_type,
                segment.rotation,
            );

            info!("  连接端口:");
            for (direction, port_pos) in &connection_ports {
                info!("    {:?}: {:?}", direction, port_pos);
            }

            // 检查实际连接
            if let Some(connections) = pathfinding_graph.connections.get(&segment.grid_pos) {
                info!("  实际连接 {} 个:", connections.len());
                for conn in connections {
                    let direction = get_direction_between(segment.grid_pos, conn.to);
                    info!("    -> {:?} ({:?})", conn.to, direction);
                }
            } else {
                warn!("  ❌ 没有实际连接");
            }
        }

        // 显示站点连接分析
        info!("\n站点连接分析:");
        for station_entity in stations.iter() {
            let station_pos = station_entity.station_data.position;
            info!(
                "📍 {} at {:?}",
                station_entity.station_data.name, station_pos
            );

            // 分析周围的路线段
            for segment in route_segments.iter() {
                if !segment.is_active {
                    continue;
                }

                let distance = manhattan_distance(station_pos, segment.grid_pos);
                if distance <= 2 {
                    let can_connect =
                        can_station_connect_to_segment_directional(station_pos, segment);

                    let direction = get_direction_between(station_pos, segment.grid_pos);
                    info!(
                        "  距离{} {:?} at {:?} 方向{:?}: {}",
                        distance,
                        segment.segment_type,
                        segment.grid_pos,
                        direction,
                        if can_connect {
                            "✅可连接"
                        } else {
                            "❌不可连接"
                        }
                    );

                    if can_connect {
                        let connection_reason = get_connection_reason(station_pos, segment);
                        info!("    连接原因: {}", connection_reason);
                    }
                }
            }
        }
    }
}

/// F9 - 强制重建修复后的连接
fn force_rebuild_connections_fixed(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut pathfinding_graph: ResMut<PathfindingGraph>,
    stations: Query<&StationEntity>,
    route_segments: Query<&RouteSegment>,
) {
    if keyboard_input.just_pressed(KeyCode::F9) {
        info!("🔧 使用修复算法重建连接图...");

        // 清空现有图
        pathfinding_graph.connections.clear();
        pathfinding_graph.nodes.clear();
        pathfinding_graph.station_lookup.clear();

        // 使用修复后的算法重建
        rebuild_pathfinding_graph_fixed(&mut pathfinding_graph, &stations, &route_segments);

        info!("修复后的连接图重建完成！");
        info!("  节点数: {}", pathfinding_graph.nodes.len());
        info!("  连接数: {}", pathfinding_graph.connections.len());
        info!("  站点数: {}", pathfinding_graph.station_lookup.len());
    }
}

/// F10 - 可视化路线段方向
fn visualize_segment_directions(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    route_segments: Query<&RouteSegment>,
    level_manager: Res<crate::bus_puzzle::LevelManager>,
    existing_visualizations: Query<Entity, With<DirectionVisualization>>,
) {
    if keyboard_input.just_pressed(KeyCode::F10) {
        // 清除现有可视化
        for entity in existing_visualizations.iter() {
            commands.entity(entity).despawn();
        }

        info!("🧭 显示路线段方向可视化...");

        for segment in route_segments.iter() {
            if segment.is_active {
                visualize_segment_ports(&mut commands, segment, &level_manager);
            }
        }
    }
}

#[derive(Component)]
struct DirectionVisualization;

/// 修复后的寻路图重建函数
pub fn rebuild_pathfinding_graph_fixed(
    pathfinding_graph: &mut PathfindingGraph,
    stations: &Query<&StationEntity>,
    route_segments: &Query<&RouteSegment>,
) {
    // 第一步：添加所有站点节点
    info!("添加站点节点...");
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
        info!("  添加站点: {} at {:?}", station.name, pos);
    }

    // 第二步：添加所有活跃的路线段节点
    info!("添加路线段节点...");
    for segment in route_segments.iter() {
        if segment.is_active {
            let pos = segment.grid_pos;

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
            info!("  添加路线段: {:?} at {:?}", segment.segment_type, pos);
        }
    }

    // 第三步：建立路线段之间的连接（考虑方向）
    info!("建立路线段连接（考虑方向）...");
    create_segment_connections_directional(pathfinding_graph, route_segments);

    // 第四步：建立站点与路线段的连接（考虑方向）
    info!("建立站点连接（考虑方向）...");
    create_station_connections_directional(pathfinding_graph, stations, route_segments);

    info!("修复后的寻路图构建完成！");
}

/// 考虑方向的路线段连接创建
fn create_segment_connections_directional(
    pathfinding_graph: &mut PathfindingGraph,
    route_segments: &Query<&RouteSegment>,
) {
    let active_segments: Vec<_> = route_segments.iter().filter(|s| s.is_active).collect();

    for segment in &active_segments {
        let connection_ports =
            get_segment_connection_ports(segment.grid_pos, &segment.segment_type, segment.rotation);

        for (direction, port_pos) in connection_ports {
            // 检查该端口位置是否有其他路线段或站点
            let target_segment = active_segments
                .iter()
                .find(|other| other.grid_pos == port_pos);

            let has_station = pathfinding_graph
                .station_lookup
                .values()
                .any(|&station_pos| station_pos == port_pos);

            if let Some(target_segment) = target_segment {
                // 检查目标路线段是否也有朝向我们的端口
                if segment_has_port_facing(target_segment, segment.grid_pos) {
                    create_bidirectional_connection(
                        pathfinding_graph,
                        segment.grid_pos,
                        target_segment.grid_pos,
                        ConnectionType::BusRoute,
                    );

                    trace!(
                        "路线段连接: {:?} <-> {:?} (方向: {:?})",
                        segment.grid_pos,
                        target_segment.grid_pos,
                        direction
                    );
                }
            } else if has_station {
                // 与站点的连接在下一步处理
            }
        }
    }
}

/// 考虑方向的站点连接创建
fn create_station_connections_directional(
    pathfinding_graph: &mut PathfindingGraph,
    stations: &Query<&StationEntity>,
    route_segments: &Query<&RouteSegment>,
) {
    for station_entity in stations.iter() {
        let station_pos = station_entity.station_data.position;

        for segment in route_segments.iter() {
            if !segment.is_active {
                continue;
            }

            if can_station_connect_to_segment_directional(station_pos, segment) {
                create_bidirectional_connection(
                    pathfinding_graph,
                    station_pos,
                    segment.grid_pos,
                    ConnectionType::Walk,
                );

                info!(
                    "✅ 站点连接: {} <-> {:?}",
                    station_entity.station_data.name, segment.segment_type
                );
            }
        }
    }
}

/// 方向枚举
#[derive(Debug, Clone, Copy, PartialEq)]
enum Direction {
    North, // 上 (y+1)
    South, // 下 (y-1)
    East,  // 右 (x+1)
    West,  // 左 (x-1)
}

impl Direction {
    #[allow(dead_code)]
    fn opposite(self) -> Self {
        match self {
            Direction::North => Direction::South,
            Direction::South => Direction::North,
            Direction::East => Direction::West,
            Direction::West => Direction::East,
        }
    }

    fn to_offset(self) -> (i32, i32) {
        match self {
            Direction::North => (0, 1),
            Direction::South => (0, -1),
            Direction::East => (1, 0),
            Direction::West => (-1, 0),
        }
    }

    fn from_offset(dx: i32, dy: i32) -> Option<Self> {
        match (dx, dy) {
            (0, 1) => Some(Direction::North),
            (0, -1) => Some(Direction::South),
            (1, 0) => Some(Direction::East),
            (-1, 0) => Some(Direction::West),
            _ => None,
        }
    }
}

/// 获取路线段的连接端口（考虑旋转）
fn get_segment_connection_ports(
    pos: GridPos,
    segment_type: &RouteSegmentType,
    rotation: u32,
) -> Vec<(Direction, GridPos)> {
    // 定义基础方向（未旋转时）
    let base_directions = match segment_type {
        RouteSegmentType::Straight => vec![Direction::West, Direction::East], // 水平：左右连接
        RouteSegmentType::Curve => vec![Direction::West, Direction::North],   // L型：左和下
        RouteSegmentType::TSplit => vec![Direction::North, Direction::South, Direction::East],
        RouteSegmentType::Cross => vec![
            Direction::North,
            Direction::South,
            Direction::East,
            Direction::West,
        ],
        RouteSegmentType::Bridge | RouteSegmentType::Tunnel => {
            vec![Direction::West, Direction::East] // 和直线段一样
        }
    };

    // 应用旋转
    base_directions
        .into_iter()
        .map(|dir| {
            let rotated_dir = rotate_direction(dir, rotation);
            let (dx, dy) = rotated_dir.to_offset();
            let port_pos = GridPos::new(pos.x + dx, pos.y + dy);
            (rotated_dir, port_pos)
        })
        .collect()
}

/// 旋转方向
fn rotate_direction(direction: Direction, rotation: u32) -> Direction {
    let steps = (rotation / 90) % 4;
    let mut result = direction;

    for _ in 0..steps {
        result = match result {
            Direction::North => Direction::East,
            Direction::East => Direction::South,
            Direction::South => Direction::West,
            Direction::West => Direction::North,
        };
    }

    result
}

/// 检查路线段是否有朝向指定位置的端口
fn segment_has_port_facing(segment: &RouteSegment, target_pos: GridPos) -> bool {
    let direction_to_target = get_direction_between(segment.grid_pos, target_pos);

    if let Some(direction) = direction_to_target {
        let connection_ports =
            get_segment_connection_ports(segment.grid_pos, &segment.segment_type, segment.rotation);

        connection_ports
            .iter()
            .any(|(port_dir, _)| *port_dir == direction)
    } else {
        false
    }
}

/// 获取两点之间的方向
fn get_direction_between(from: GridPos, to: GridPos) -> Option<Direction> {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    Direction::from_offset(dx, dy)
}

/// 检查站点是否可以连接到路线段（考虑方向）
fn can_station_connect_to_segment_directional(
    station_pos: GridPos,
    segment: &RouteSegment,
) -> bool {
    let distance = manhattan_distance(station_pos, segment.grid_pos);

    match distance {
        1 => {
            // 直接相邻：检查路线段是否有朝向站点的端口
            segment_has_port_facing(segment, station_pos)
        }
        0 => {
            // 重叠位置：站点和路线段在同一位置，允许连接
            true
        }
        _ => {
            // 距离大于1：检查站点是否在路线段的端口位置
            let connection_ports = get_segment_connection_ports(
                segment.grid_pos,
                &segment.segment_type,
                segment.rotation,
            );

            connection_ports
                .iter()
                .any(|(_, port_pos)| *port_pos == station_pos)
        }
    }
}

/// 获取连接原因（用于调试）
fn get_connection_reason(station_pos: GridPos, segment: &RouteSegment) -> String {
    let distance = manhattan_distance(station_pos, segment.grid_pos);

    match distance {
        0 => "重叠位置".to_string(),
        1 => {
            if segment_has_port_facing(segment, station_pos) {
                "直接相邻且路线段有朝向站点的端口".to_string()
            } else {
                "直接相邻但路线段没有朝向站点的端口".to_string()
            }
        }
        _ => {
            let connection_ports = get_segment_connection_ports(
                segment.grid_pos,
                &segment.segment_type,
                segment.rotation,
            );

            if connection_ports
                .iter()
                .any(|(_, port_pos)| *port_pos == station_pos)
            {
                "站点位于路线段的端口位置".to_string()
            } else {
                "距离过远且不在端口位置".to_string()
            }
        }
    }
}

/// 创建双向连接
fn create_bidirectional_connection(
    pathfinding_graph: &mut PathfindingGraph,
    pos_a: GridPos,
    pos_b: GridPos,
    connection_type: ConnectionType,
) {
    let cost = match connection_type {
        ConnectionType::Walk => 0.5,
        ConnectionType::BusRoute => 1.0,
        ConnectionType::Transfer => 2.0,
    };

    // A -> B
    pathfinding_graph
        .connections
        .entry(pos_a)
        .or_default()
        .push(Connection {
            to: pos_b,
            cost,
            route_id: Some(format!("route_{}_{}", pos_a.x, pos_a.y)),
            connection_type: connection_type.clone(),
        });

    // B -> A
    pathfinding_graph
        .connections
        .entry(pos_b)
        .or_default()
        .push(Connection {
            to: pos_a,
            cost,
            route_id: Some(format!("route_{}_{}", pos_b.x, pos_b.y)),
            connection_type,
        });
}

/// 可视化路线段端口
fn visualize_segment_ports(
    commands: &mut Commands,
    segment: &RouteSegment,
    level_manager: &crate::bus_puzzle::LevelManager,
) {
    let tile_size = level_manager.tile_size;
    let (grid_width, grid_height) = if let Some(level_data) = &level_manager.current_level {
        level_data.grid_size
    } else {
        (10, 8)
    };

    // 获取基础连接方向（未旋转的）
    let base_directions = match segment.segment_type {
        RouteSegmentType::Straight => vec![Direction::West, Direction::East], // 水平：左右连接
        RouteSegmentType::Curve => vec![Direction::West, Direction::North],   // L型：左和上
        RouteSegmentType::TSplit => vec![Direction::North, Direction::South, Direction::East],
        RouteSegmentType::Cross => vec![
            Direction::North,
            Direction::South,
            Direction::East,
            Direction::West,
        ],
        RouteSegmentType::Bridge | RouteSegmentType::Tunnel => {
            vec![Direction::West, Direction::East] // 和直线段一样，水平连接
        }
    };

    let center_world = segment
        .grid_pos
        .to_world_pos(tile_size, grid_width, grid_height);

    // 为每个基础连接方向创建箭头指示器
    for (index, direction) in base_directions.iter().enumerate() {
        // 计算箭头位置（从路线段中心向外延伸）
        let (dx, dy) = direction.to_offset();
        let arrow_offset = Vec3::new(dx as f32 * 20.0, dy as f32 * 20.0, 0.0);
        let arrow_pos = center_world + arrow_offset;

        // 计算箭头旋转角度（基础方向 + 路线段旋转）
        let base_rotation = match direction {
            Direction::North => 0.0,                        // 向上
            Direction::East => -std::f32::consts::PI / 2.0, // 向右
            Direction::South => std::f32::consts::PI,       // 向下
            Direction::West => std::f32::consts::PI / 2.0,  // 向左
        };

        // 添加路线段的旋转
        let segment_rotation = (segment.rotation as f32) * std::f32::consts::PI / 180.0;
        let final_rotation = base_rotation + segment_rotation;

        // 为每个连接方向使用不同颜色
        let color = match index % 4 {
            0 => Color::srgb(1.0, 0.0, 0.0), // 红色 - 第一个连接
            1 => Color::srgb(0.0, 1.0, 0.0), // 绿色 - 第二个连接
            2 => Color::srgb(0.0, 0.0, 1.0), // 蓝色 - 第三个连接
            3 => Color::srgb(1.0, 1.0, 0.0), // 黄色 - 第四个连接
            _ => Color::WHITE,
        };

        // 同时旋转箭头位置
        let rotated_offset = Vec3::new(
            arrow_offset.x * segment_rotation.cos() - arrow_offset.y * segment_rotation.sin(),
            arrow_offset.x * segment_rotation.sin() + arrow_offset.y * segment_rotation.cos(),
            arrow_offset.z,
        );
        let final_arrow_pos = center_world + rotated_offset;

        // 创建箭头形状（三角形指向连接方向）
        commands.spawn((
            Sprite {
                color,
                custom_size: Some(Vec2::new(12.0, 16.0)), // 稍小一点的箭头
                ..default()
            },
            Transform::from_translation(final_arrow_pos + Vec3::Z * 15.0)
                .with_rotation(Quat::from_rotation_z(final_rotation)),
            DirectionVisualization,
            Name::new(format!(
                "Connection {:?} for {:?} (rot: {}°)",
                direction, segment.segment_type, segment.rotation
            )),
        ));
    }
}
