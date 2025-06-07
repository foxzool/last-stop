// src/bus_puzzle/connection_system.rs

use bevy::prelude::*;
use crate::bus_puzzle::{
    GridPos, RouteSegmentType, PathfindingGraph, GraphNode, GraphNodeType,
    Connection, ConnectionType, GameState, StationEntity, RouteSegment
};

/// 改进的连接系统 - 解决站点和路线段连接问题
pub struct ConnectionSystemPlugin;

impl Plugin for ConnectionSystemPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (
            debug_connections,
            visualize_connections,
            force_rebuild_connections,
        ));
    }
}

/// 调试连接状态 - 按F8查看详细连接信息
fn debug_connections(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    pathfinding_graph: Res<PathfindingGraph>,
    stations: Query<&StationEntity>,
    route_segments: Query<&RouteSegment>,
) {
    if keyboard_input.just_pressed(KeyCode::F8) {
        info!("=== 连接系统调试 ===");

        // 显示所有站点
        info!("站点列表:");
        for station_entity in stations.iter() {
            let station = &station_entity.station_data;
            info!("  {} 位置: {:?}", station.name, station.position);

            // 检查该站点的连接
            if let Some(connections) = pathfinding_graph.connections.get(&station.position) {
                info!("    连接到 {} 个位置:", connections.len());
                for conn in connections {
                    info!("      -> {:?} (成本: {:.1}, 类型: {:?})",
                        conn.to, conn.cost, conn.connection_type);
                }
            } else {
                warn!("    ❌ 没有任何连接！");
            }
        }

        // 显示所有路线段
        info!("路线段列表:");
        for segment in route_segments.iter() {
            info!("  {:?} 位置: {:?} 旋转: {}°",
                segment.segment_type, segment.grid_pos, segment.rotation);

            // 显示该路线段的理论连接点
            let connection_points = get_segment_connection_points(
                segment.grid_pos,
                &segment.segment_type,
                segment.rotation
            );
            info!("    理论连接点: {:?}", connection_points);

            // 检查实际连接
            if let Some(connections) = pathfinding_graph.connections.get(&segment.grid_pos) {
                info!("    实际连接 {} 个位置:", connections.len());
                for conn in connections {
                    info!("      -> {:?}", conn.to);
                }
            } else {
                warn!("    ❌ 没有实际连接！");
            }
        }

        // 检查站点到路线段的连接可能性
        info!("站点-路线段连接分析:");
        for station_entity in stations.iter() {
            let station_pos = station_entity.station_data.position;
            info!("  检查 {} ({:?}) 周围的路线段:",
                station_entity.station_data.name, station_pos);

            for segment in route_segments.iter() {
                let distance = manhattan_distance(station_pos, segment.grid_pos);
                if distance <= 2 { // 检查距离2格内的路线段
                    let connection_points = get_segment_connection_points(
                        segment.grid_pos,
                        &segment.segment_type,
                        segment.rotation
                    );

                    let can_connect = can_station_connect_to_segment(
                        station_pos,
                        segment.grid_pos,
                        &connection_points
                    );

                    info!("    距离 {} 的 {:?}: {} {}",
                        distance,
                        segment.segment_type,
                        if can_connect { "✅ 可连接" } else { "❌ 不可连接" },
                        format!("连接点: {:?}", connection_points)
                    );
                }
            }
        }
    }
}

/// F9 - 强制重建连接图
fn force_rebuild_connections(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut pathfinding_graph: ResMut<PathfindingGraph>,
    game_state: Res<GameState>,
    stations: Query<&StationEntity>,
    route_segments: Query<&RouteSegment>,
) {
    if keyboard_input.just_pressed(KeyCode::F9) {
        info!("强制重建连接图...");

        // 清空现有图
        pathfinding_graph.connections.clear();
        pathfinding_graph.nodes.clear();
        pathfinding_graph.station_lookup.clear();

        // 重新构建
        rebuild_pathfinding_graph_improved(&mut pathfinding_graph, &game_state, &stations, &route_segments);

        info!("连接图重建完成！");
        info!("  节点数: {}", pathfinding_graph.nodes.len());
        info!("  连接数: {}", pathfinding_graph.connections.len());
        info!("  站点数: {}", pathfinding_graph.station_lookup.len());
    }
}

/// 改进的寻路图重建函数
pub fn rebuild_pathfinding_graph_improved(
    pathfinding_graph: &mut PathfindingGraph,
    game_state: &GameState,
    stations: &Query<&StationEntity>,
    route_segments: &Query<&RouteSegment>,
) {
    // 第一步：添加所有站点节点
    info!("添加站点节点...");
    for station_entity in stations.iter() {
        let station = &station_entity.station_data;
        let pos = station.position;

        pathfinding_graph.nodes.insert(pos, GraphNode {
            position: pos,
            node_type: GraphNodeType::Station,
            station_name: Some(station.name.clone()),
            is_accessible: true,
        });

        pathfinding_graph.station_lookup.insert(station.name.clone(), pos);
        info!("  添加站点: {} at {:?}", station.name, pos);
    }

    // 第二步：添加所有活跃的路线段节点
    info!("添加路线段节点...");
    for segment in route_segments.iter() {
        if segment.is_active {
            let pos = segment.grid_pos;

            pathfinding_graph.nodes.insert(pos, GraphNode {
                position: pos,
                node_type: match segment.segment_type {
                    RouteSegmentType::Cross | RouteSegmentType::TSplit => GraphNodeType::Intersection,
                    _ => GraphNodeType::RouteSegment,
                },
                station_name: None,
                is_accessible: true,
            });
            info!("  添加路线段: {:?} at {:?}", segment.segment_type, pos);
        }
    }

    // 第三步：建立路线段之间的连接
    info!("建立路线段连接...");
    for segment in route_segments.iter() {
        if segment.is_active {
            create_segment_connections(pathfinding_graph, segment, route_segments);
        }
    }

    // 第四步：建立站点与路线段的连接
    info!("建立站点连接...");
    for station_entity in stations.iter() {
        create_station_connections(pathfinding_graph, station_entity, route_segments);
    }

    info!("寻路图构建完成！");
}

/// 改进的路线段连接创建
fn create_segment_connections(
    pathfinding_graph: &mut PathfindingGraph,
    segment: &RouteSegment,
    all_segments: &Query<&RouteSegment>,
) {
    let connection_points = get_segment_connection_points(
        segment.grid_pos,
        &segment.segment_type,
        segment.rotation
    );

    for connection_point in connection_points {
        // 检查连接点是否有其他路线段或站点
        let has_segment = all_segments.iter().any(|other_segment|
            other_segment.is_active && other_segment.grid_pos == connection_point
        );

        let has_station = pathfinding_graph.station_lookup.values()
            .any(|&station_pos| station_pos == connection_point);

        if has_segment || has_station {
            // 创建双向连接
            pathfinding_graph.connections
                .entry(segment.grid_pos)
                .or_insert_with(Vec::new)
                .push(Connection {
                    to: connection_point,
                    cost: 1.0,
                    route_id: Some(format!("route_{}", segment.grid_pos.x + segment.grid_pos.y)),
                    connection_type: ConnectionType::BusRoute,
                });

            pathfinding_graph.connections
                .entry(connection_point)
                .or_insert_with(Vec::new)
                .push(Connection {
                    to: segment.grid_pos,
                    cost: 1.0,
                    route_id: Some(format!("route_{}", segment.grid_pos.x + segment.grid_pos.y)),
                    connection_type: ConnectionType::BusRoute,
                });
        }
    }
}

/// 改进的站点连接创建
fn create_station_connections(
    pathfinding_graph: &mut PathfindingGraph,
    station_entity: &StationEntity,
    route_segments: &Query<&RouteSegment>,
) {
    let station_pos = station_entity.station_data.position;

    // 检查站点周围更大范围内的路线段
    for segment in route_segments.iter() {
        if !segment.is_active {
            continue;
        }

        let segment_pos = segment.grid_pos;
        let distance = manhattan_distance(station_pos, segment_pos);

        // 扩大连接检测范围到2格
        if distance <= 2 {
            let connection_points = get_segment_connection_points(
                segment_pos,
                &segment.segment_type,
                segment.rotation
            );

            if can_station_connect_to_segment(station_pos, segment_pos, &connection_points) {
                // 创建站点到路线段的连接
                pathfinding_graph.connections
                    .entry(station_pos)
                    .or_insert_with(Vec::new)
                    .push(Connection {
                        to: segment_pos,
                        cost: 0.5, // 步行到路线段的成本
                        route_id: None,
                        connection_type: ConnectionType::Walk,
                    });

                // 创建路线段到站点的连接
                pathfinding_graph.connections
                    .entry(segment_pos)
                    .or_insert_with(Vec::new)
                    .push(Connection {
                        to: station_pos,
                        cost: 0.5,
                        route_id: None,
                        connection_type: ConnectionType::Walk,
                    });

                info!("✅ 连接建立: {} <-> {:?} (距离: {})",
                    station_entity.station_data.name, segment.segment_type, distance);
            }
        }
    }
}

/// 获取路线段的连接点（考虑旋转）
pub fn get_segment_connection_points(
    pos: GridPos,
    segment_type: &RouteSegmentType,
    rotation: u32
) -> Vec<GridPos> {
    let base_offsets = match segment_type {
        RouteSegmentType::Straight => vec![(0, -1), (0, 1)], // 上下
        RouteSegmentType::Curve => vec![(0, -1), (1, 0)],    // 上右
        RouteSegmentType::TSplit => vec![(0, -1), (0, 1), (1, 0)], // 上下右
        RouteSegmentType::Cross => vec![(0, -1), (0, 1), (-1, 0), (1, 0)], // 四方向
        RouteSegmentType::Bridge | RouteSegmentType::Tunnel => vec![(0, -1), (0, 1)], // 上下
    };

    base_offsets.into_iter()
        .map(|(dx, dy)| {
            let (rotated_dx, rotated_dy) = rotate_offset(dx, dy, rotation);
            GridPos::new(pos.x + rotated_dx, pos.y + rotated_dy)
        })
        .collect()
}

/// 旋转偏移量
fn rotate_offset(dx: i32, dy: i32, rotation: u32) -> (i32, i32) {
    match rotation % 360 {
        0 => (dx, dy),
        90 => (-dy, dx),
        180 => (-dx, -dy),
        270 => (dy, -dx),
        _ => (dx, dy), // 默认不旋转
    }
}

/// 检查站点是否可以连接到路线段
fn can_station_connect_to_segment(
    station_pos: GridPos,
    segment_pos: GridPos,
    connection_points: &[GridPos]
) -> bool {
    // 方式1：站点直接与路线段相邻
    let distance = manhattan_distance(station_pos, segment_pos);
    if distance == 1 {
        return true;
    }

    // 方式2：站点位于路线段的连接点上
    if connection_points.contains(&station_pos) {
        return true;
    }

    // 方式3：站点与路线段的连接点相邻
    for &connection_point in connection_points {
        if manhattan_distance(station_pos, connection_point) == 1 {
            return true;
        }
    }

    false
}

/// 计算曼哈顿距离
fn manhattan_distance(pos1: GridPos, pos2: GridPos) -> u32 {
    ((pos1.x - pos2.x).abs() + (pos1.y - pos2.y).abs()) as u32
}

/// F10 - 可视化连接（在游戏中显示连接线）
fn visualize_connections(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    pathfinding_graph: Res<PathfindingGraph>,
    level_manager: Res<crate::bus_puzzle::LevelManager>,
    existing_visualizations: Query<Entity, With<ConnectionVisualization>>,
) {
    if keyboard_input.just_pressed(KeyCode::F10) {
        // 清除现有的可视化
        for entity in existing_visualizations.iter() {
            commands.entity(entity).despawn();
        }

        info!("显示连接可视化...");

        // 为每个连接创建可视化线条
        for (from_pos, connections) in &pathfinding_graph.connections {
            for connection in connections {
                spawn_connection_line(
                    &mut commands,
                    *from_pos,
                    connection.to,
                    &connection.connection_type,
                    &level_manager
                );
            }
        }
    }
}

#[derive(Component)]
struct ConnectionVisualization;

fn spawn_connection_line(
    commands: &mut Commands,
    from: GridPos,
    to: GridPos,
    connection_type: &ConnectionType,
    level_manager: &crate::bus_puzzle::LevelManager,
) {
    let tile_size = level_manager.tile_size;
    let (grid_width, grid_height) = if let Some(level_data) = &level_manager.current_level {
        level_data.grid_size
    } else {
        (10, 8)
    };

    let from_world = from.to_world_pos(tile_size, grid_width, grid_height);
    let to_world = to.to_world_pos(tile_size, grid_width, grid_height);

    // 计算线条的中点和方向
    let midpoint = (from_world + to_world) / 2.0;
    let direction = (to_world - from_world).normalize();
    let length = from_world.distance(to_world);

    // 根据连接类型选择颜色
    let color = match connection_type {
        ConnectionType::Walk => Color::srgb(1.0, 1.0, 0.0), // 黄色 - 步行
        ConnectionType::BusRoute => Color::srgb(0.0, 1.0, 0.0), // 绿色 - 公交
        ConnectionType::Transfer => Color::srgb(1.0, 0.0, 1.0), // 紫色 - 换乘
    };

    // 创建线条实体
    commands.spawn((
        Sprite {
            color,
            custom_size: Some(Vec2::new(length, 3.0)),
            ..default()
        },
        Transform::from_translation(midpoint + Vec3::Z * 10.0) // 在最顶层显示
            .with_rotation(Quat::from_rotation_z(direction.y.atan2(direction.x))),
        ConnectionVisualization,
    ));
}
