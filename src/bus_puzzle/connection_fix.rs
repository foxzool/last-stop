// src/bus_puzzle/connection_fix.rs - 连接系统修复

use crate::bus_puzzle::{
    manhattan_distance, Connection, ConnectionType, GameStateEnum, GridPos, PathfindingGraph,
    RouteSegment, RouteSegmentType,
};
use bevy::prelude::*;

pub struct ConnectionFixPlugin;

impl Plugin for ConnectionFixPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (fix_missing_connections, debug_route_directions)
                .run_if(in_state(GameStateEnum::Playing)),
        );
    }
}

/// F11 - 强制修复缺失的连接
fn fix_missing_connections(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut pathfinding_graph: ResMut<PathfindingGraph>,
    route_segments: Query<&RouteSegment>,
) {
    if keyboard_input.just_pressed(KeyCode::F11) {
        info!("🔧 修复缺失的路线段连接...");

        let segments: Vec<_> = route_segments.iter().filter(|s| s.is_active).collect();

        // 为每个路线段添加直接相邻连接
        for (i, segment_a) in segments.iter().enumerate() {
            for segment_b in segments.iter().skip(i + 1) {
                let distance = manhattan_distance(segment_a.grid_pos, segment_b.grid_pos);

                if distance == 1 {
                    // 检查是否应该连接
                    if should_segments_connect(segment_a, segment_b) {
                        add_bidirectional_connection(
                            &mut pathfinding_graph,
                            segment_a.grid_pos,
                            segment_b.grid_pos,
                            ConnectionType::BusRoute,
                        );

                        info!(
                            "修复连接: {:?} <-> {:?}",
                            segment_a.grid_pos, segment_b.grid_pos
                        );
                    }
                }
            }
        }

        info!("连接修复完成！");
    }
}

/// F12 - 调试路线方向和连接
fn debug_route_directions(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    route_segments: Query<&RouteSegment>,
) {
    if keyboard_input.just_pressed(KeyCode::F12) {
        info!("🧭 路线方向调试");

        for segment in route_segments.iter() {
            if !segment.is_active {
                continue;
            }

            info!(
                "路线段: {:?} at {:?} 旋转: {}°",
                segment.segment_type, segment.grid_pos, segment.rotation
            );

            let theoretical_connections = get_theoretical_connections_fixed(
                segment.grid_pos,
                &segment.segment_type,
                segment.rotation,
            );
            info!("  理论连接: {:?}", theoretical_connections);

            // 检查实际应该连接的方向
            let actual_directions = get_actual_connection_directions(segment);
            info!("  实际方向: {:?}", actual_directions);
        }
    }
}

/// 检查两个路线段是否应该连接
fn should_segments_connect(segment_a: &RouteSegment, segment_b: &RouteSegment) -> bool {
    let pos_a = segment_a.grid_pos;
    let pos_b = segment_b.grid_pos;

    // 计算相对位置
    let dx = pos_b.x - pos_a.x;
    let dy = pos_b.y - pos_a.y;

    // 检查segment_a是否有连接到segment_b方向的端口
    let directions_a = get_segment_connection_directions(segment_a);
    let directions_b = get_segment_connection_directions(segment_b);

    // 检查方向匹配
    let direction_to_b = match (dx, dy) {
        (1, 0) => Direction::Right,
        (-1, 0) => Direction::Left,
        (0, 1) => Direction::Up,
        (0, -1) => Direction::Down,
        _ => return false,
    };

    let direction_to_a = direction_to_b.opposite();

    directions_a.contains(&direction_to_b) && directions_b.contains(&direction_to_a)
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    fn opposite(self) -> Self {
        match self {
            Direction::Up => Direction::Down,
            Direction::Down => Direction::Up,
            Direction::Left => Direction::Right,
            Direction::Right => Direction::Left,
        }
    }
}

/// 获取路线段的连接方向
fn get_segment_connection_directions(segment: &RouteSegment) -> Vec<Direction> {
    let base_directions = match segment.segment_type {
        RouteSegmentType::Straight => vec![Direction::Up, Direction::Down],
        RouteSegmentType::Curve => vec![Direction::Up, Direction::Right],
        RouteSegmentType::TSplit => vec![Direction::Up, Direction::Down, Direction::Right],
        RouteSegmentType::Cross => vec![
            Direction::Up,
            Direction::Down,
            Direction::Left,
            Direction::Right,
        ],
        RouteSegmentType::Bridge | RouteSegmentType::Tunnel => vec![Direction::Up, Direction::Down],
    };

    // 应用旋转
    base_directions
        .into_iter()
        .map(|dir| rotate_direction(dir, segment.rotation))
        .collect()
}

fn rotate_direction(direction: Direction, rotation: u32) -> Direction {
    let steps = (rotation / 90) % 4;
    let mut result = direction;

    for _ in 0..steps {
        result = match result {
            Direction::Up => Direction::Right,
            Direction::Right => Direction::Down,
            Direction::Down => Direction::Left,
            Direction::Left => Direction::Up,
        };
    }

    result
}

fn get_theoretical_connections_fixed(
    pos: GridPos,
    segment_type: &RouteSegmentType,
    rotation: u32,
) -> Vec<GridPos> {
    let directions = match segment_type {
        RouteSegmentType::Straight => vec![Direction::Up, Direction::Down],
        RouteSegmentType::Curve => vec![Direction::Up, Direction::Right],
        RouteSegmentType::TSplit => vec![Direction::Up, Direction::Down, Direction::Right],
        RouteSegmentType::Cross => vec![
            Direction::Up,
            Direction::Down,
            Direction::Left,
            Direction::Right,
        ],
        RouteSegmentType::Bridge | RouteSegmentType::Tunnel => vec![Direction::Up, Direction::Down],
    };

    directions
        .into_iter()
        .map(|dir| rotate_direction(dir, rotation))
        .map(|dir| {
            let (dx, dy) = match dir {
                Direction::Up => (0, 1),
                Direction::Down => (0, -1),
                Direction::Left => (-1, 0),
                Direction::Right => (1, 0),
            };
            GridPos::new(pos.x + dx, pos.y + dy)
        })
        .collect()
}

fn get_actual_connection_directions(segment: &RouteSegment) -> Vec<Direction> {
    get_segment_connection_directions(segment)
}

fn add_bidirectional_connection(
    pathfinding_graph: &mut PathfindingGraph,
    pos_a: GridPos,
    pos_b: GridPos,
    connection_type: ConnectionType,
) {
    // A -> B
    pathfinding_graph
        .connections
        .entry(pos_a)
        .or_insert_with(Vec::new)
        .push(Connection {
            to: pos_b,
            cost: 1.0,
            route_id: Some(format!("fixed_{}", pos_a.x + pos_a.y)),
            connection_type: connection_type.clone(),
        });

    // B -> A
    pathfinding_graph
        .connections
        .entry(pos_b)
        .or_insert_with(Vec::new)
        .push(Connection {
            to: pos_a,
            cost: 1.0,
            route_id: Some(format!("fixed_{}", pos_b.x + pos_b.y)),
            connection_type: connection_type.clone(),
        });
}
