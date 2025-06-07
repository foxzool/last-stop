// src/bus_puzzle/connection_debug.rs - 连接系统调试工具

use bevy::prelude::*;
use crate::bus_puzzle::{
    GridPos, RouteSegmentType, PathfindingGraph, GameStateEnum, 
    StationEntity, RouteSegment, LevelManager, ConnectionType
};

pub struct ConnectionDebugPlugin;

impl Plugin for ConnectionDebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (
            debug_connections_detailed,
            visualize_connections,
            test_specific_connection,
        ).run_if(in_state(GameStateEnum::Playing)));
    }
}

/// F8 - 详细的连接调试信息
fn debug_connections_detailed(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    pathfinding_graph: Res<PathfindingGraph>,
    stations: Query<&StationEntity>,
    route_segments: Query<&RouteSegment>,
) {
    if keyboard_input.just_pressed(KeyCode::F8) {
        info!("=== 连接系统详细调试 ===");
        
        // 检查每个站点的连接情况
        for station_entity in stations.iter() {
            let station = &station_entity.station_data;
            let station_pos = station.position;
            
            info!("📍 站点: {} 位置: {:?}", station.name, station_pos);
            
            // 显示该站点的所有连接
            if let Some(connections) = pathfinding_graph.connections.get(&station_pos) {
                info!("  ✅ 有 {} 个连接:", connections.len());
                for (i, conn) in connections.iter().enumerate() {
                    info!("    {}. -> {:?} (成本: {:.1}, 类型: {:?})", 
                        i + 1, conn.to, conn.cost, conn.connection_type);
                }
            } else {
                warn!("  ❌ 没有任何连接！");
            }
            
            // 分析周围的路线段
            info!("  周围路线段分析:");
            let mut found_nearby = false;
            for segment in route_segments.iter() {
                let distance = manhattan_distance(station_pos, segment.grid_pos);
                if distance <= 3 { // 显示3格内的所有路线段
                    found_nearby = true;
                    let should_connect = should_connect_station_segment(station_pos, segment);
                    info!("    距离{}: {:?} at {:?} {} {}", 
                        distance, 
                        segment.segment_type, 
                        segment.grid_pos,
                        if should_connect { "✅应该连接" } else { "❌不应连接" },
                        if !segment.is_active { "(未激活)" } else { "" }
                    );
                }
            }
            if !found_nearby {
                warn!("    周围3格内没有路线段");
            }
            info!(""); // 空行
        }
        
        // 检查路线段的连接
        info!("路线段连接分析:");
        for segment in route_segments.iter() {
            if !segment.is_active {
                continue;
            }
            
            info!("🛤️  {:?} at {:?} (旋转: {}°)", 
                segment.segment_type, segment.grid_pos, segment.rotation);
            
            let connection_points = get_theoretical_connections(
                segment.grid_pos, &segment.segment_type, segment.rotation
            );
            info!("  理论连接点: {:?}", connection_points);
            
            if let Some(connections) = pathfinding_graph.connections.get(&segment.grid_pos) {
                info!("  实际连接 {} 个:", connections.len());
                for conn in connections {
                    info!("    -> {:?} ({:?})", conn.to, conn.connection_type);
                }
            } else {
                warn!("  ❌ 没有实际连接");
            }
            info!("");
        }
    }
}

/// F10 - 可视化连接
fn visualize_connections(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    pathfinding_graph: Res<PathfindingGraph>,
    level_manager: Res<LevelManager>,
    existing_visualizations: Query<Entity, With<ConnectionVisualization>>,
) {
    if keyboard_input.just_pressed(KeyCode::F10) {
        // 清除现有可视化
        for entity in existing_visualizations.iter() {
            commands.entity(entity).despawn();
        }
        
        info!("🎨 显示连接可视化");
        
        let tile_size = level_manager.tile_size;
        let (grid_width, grid_height) = if let Some(level_data) = &level_manager.current_level {
            level_data.grid_size
        } else {
            (10, 8)
        };
        
        // 为每个连接创建可视化线条
        for (from_pos, connections) in &pathfinding_graph.connections {
            for connection in connections {
                spawn_connection_line(
                    &mut commands, 
                    *from_pos, 
                    connection.to, 
                    &connection.connection_type,
                    tile_size,
                    grid_width,
                    grid_height
                );
            }
        }
        
        info!("连接可视化完成，按F10再次切换");
    }
}

/// F6 - 测试特定站点的连接
fn test_specific_connection(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    stations: Query<&StationEntity>,
    route_segments: Query<&RouteSegment>,
    pathfinding_graph: Res<PathfindingGraph>,
) {
    if keyboard_input.just_pressed(KeyCode::F6) {
        info!("🔍 测试特定连接");
        
        // 找到A站和附近的路线段进行详细分析
        if let Some(a_station) = stations.iter().find(|s| s.station_data.name == "A站") {
            let station_pos = a_station.station_data.position;
            info!("测试 A站 位置: {:?}", station_pos);
            
            // 查找最近的路线段
            let mut closest_segment = None;
            let mut min_distance = u32::MAX;
            
            for segment in route_segments.iter() {
                if segment.is_active {
                    let distance = manhattan_distance(station_pos, segment.grid_pos);
                    if distance < min_distance {
                        min_distance = distance;
                        closest_segment = Some(segment);
                    }
                }
            }
            
            if let Some(segment) = closest_segment {
                info!("最近的路线段: {:?} at {:?} (距离: {})", 
                    segment.segment_type, segment.grid_pos, min_distance);
                
                // 详细分析为什么没连接
                analyze_connection_failure(station_pos, segment, &pathfinding_graph);
            } else {
                warn!("没有找到任何激活的路线段");
            }
        }
    }
}

#[derive(Component)]
pub struct ConnectionVisualization;

fn spawn_connection_line(
    commands: &mut Commands,
    from: GridPos,
    to: GridPos,
    connection_type: &ConnectionType,
    tile_size: f32,
    grid_width: u32,
    grid_height: u32,
) {
    let from_world = from.to_world_pos(tile_size, grid_width, grid_height);
    let to_world = to.to_world_pos(tile_size, grid_width, grid_height);
    
    let midpoint = (from_world + to_world) / 2.0;
    let direction = (to_world - from_world).normalize_or_zero();
    let length = from_world.distance(to_world);
    
    // 根据连接类型选择颜色和粗细
    let (color, thickness) = match connection_type {
        ConnectionType::Walk => (Color::srgb(1.0, 1.0, 0.0), 2.0), // 黄色细线 - 步行
        ConnectionType::BusRoute => (Color::srgb(0.0, 1.0, 0.0), 4.0), // 绿色粗线 - 公交
        ConnectionType::Transfer => (Color::srgb(1.0, 0.0, 1.0), 3.0), // 紫色中线 - 换乘
    };
    
    commands.spawn((
        Sprite {
            color,
            custom_size: Some(Vec2::new(length, thickness)),
            ..default()
        },
        Transform::from_translation(midpoint + Vec3::Z * 15.0) // 在很高的层级显示
            .with_rotation(if length > 0.0 { 
                Quat::from_rotation_z(direction.y.atan2(direction.x)) 
            } else { 
                Quat::IDENTITY 
            }),
        ConnectionVisualization,
        Name::new(format!("Connection {:?}->{:?}", from, to)),
    ));
}

// 辅助函数
fn manhattan_distance(pos1: GridPos, pos2: GridPos) -> u32 {
    ((pos1.x - pos2.x).abs() + (pos1.y - pos2.y).abs()) as u32
}

fn should_connect_station_segment(station_pos: GridPos, segment: &RouteSegment) -> bool {
    if !segment.is_active {
        return false;
    }
    
    let distance = manhattan_distance(station_pos, segment.grid_pos);
    
    // 基本距离检查
    if distance > 2 {
        return false;
    }
    
    // 直接相邻
    if distance == 1 {
        return true;
    }
    
    // 检查连接点
    let connection_points = get_theoretical_connections(
        segment.grid_pos, &segment.segment_type, segment.rotation
    );
    
    for connection_point in connection_points {
        if manhattan_distance(station_pos, connection_point) <= 1 {
            return true;
        }
    }
    
    false
}

fn get_theoretical_connections(
    pos: GridPos, 
    segment_type: &RouteSegmentType, 
    rotation: u32
) -> Vec<GridPos> {
    let base_offsets = match segment_type {
        RouteSegmentType::Straight => vec![(0, -1), (0, 1)],
        RouteSegmentType::Curve => vec![(0, -1), (1, 0)],
        RouteSegmentType::TSplit => vec![(0, -1), (0, 1), (1, 0)],
        RouteSegmentType::Cross => vec![(0, -1), (0, 1), (-1, 0), (1, 0)],
        RouteSegmentType::Bridge | RouteSegmentType::Tunnel => vec![(0, -1), (0, 1)],
    };
    
    base_offsets.into_iter()
        .map(|(dx, dy)| {
            let (rotated_dx, rotated_dy) = rotate_offset(dx, dy, rotation);
            GridPos::new(pos.x + rotated_dx, pos.y + rotated_dy)
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

fn analyze_connection_failure(
    station_pos: GridPos, 
    segment: &RouteSegment, 
    pathfinding_graph: &PathfindingGraph
) {
    info!("🔍 分析连接失败原因:");
    info!("  站点位置: {:?}", station_pos);
    info!("  路线段位置: {:?}", segment.grid_pos);
    info!("  距离: {}", manhattan_distance(station_pos, segment.grid_pos));
    info!("  路线段激活: {}", segment.is_active);
    
    let connection_points = get_theoretical_connections(
        segment.grid_pos, &segment.segment_type, segment.rotation
    );
    info!("  路线段连接点: {:?}", connection_points);
    
    // 检查站点是否在寻路图中
    if pathfinding_graph.nodes.contains_key(&station_pos) {
        info!("  ✅ 站点在寻路图中");
    } else {
        warn!("  ❌ 站点不在寻路图中");
    }
    
    // 检查路线段是否在寻路图中
    if pathfinding_graph.nodes.contains_key(&segment.grid_pos) {
        info!("  ✅ 路线段在寻路图中");
    } else {
        warn!("  ❌ 路线段不在寻路图中");
    }
    
    // 检查具体的连接条件
    for (i, &connection_point) in connection_points.iter().enumerate() {
        let dist_to_connection = manhattan_distance(station_pos, connection_point);
        info!("  连接点{}: {:?} 距离站点: {}", i, connection_point, dist_to_connection);
    }
}
