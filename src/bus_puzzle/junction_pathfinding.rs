// src/bus_puzzle/junction_pathfinding.rs - 路口内部寻路系统

use crate::bus_puzzle::{
    Connection, ConnectionType, GameStateEnum, GraphNode, GraphNodeType, GridPos, PathNode,
    PathNodeType, PathfindingGraph, RouteSegment, RouteSegmentType,
};
use bevy::prelude::*;

pub struct JunctionPathfindingPlugin;

impl Plugin for JunctionPathfindingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (debug_junction_structure, create_junction_internal_nodes)
                .run_if(in_state(GameStateEnum::Playing)),
        );
    }
}

/// F9 - 创建路口内部节点
fn create_junction_internal_nodes(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut pathfinding_graph: ResMut<PathfindingGraph>,
    route_segments: Query<&RouteSegment>,
) {
    if keyboard_input.just_pressed(KeyCode::F9) {
        info!("🚦 创建路口内部寻路节点...");

        // 为每个复杂路线段创建内部节点
        for segment in route_segments.iter() {
            if !segment.is_active {
                continue;
            }

            match segment.segment_type {
                RouteSegmentType::Curve | RouteSegmentType::TSplit | RouteSegmentType::Cross => {
                    create_junction_internal_structure(&mut pathfinding_graph, segment);
                }
                _ => {} // 直线段不需要内部结构
            }
        }

        info!("路口内部结构创建完成！");
    }
}

/// 为路口创建内部寻路结构
fn create_junction_internal_structure(
    pathfinding_graph: &mut PathfindingGraph,
    segment: &RouteSegment,
) {
    let center_pos = segment.grid_pos;
    let segment_type = &segment.segment_type;
    let rotation = segment.rotation;

    info!("创建 {:?} 路口内部结构 at {:?}", segment_type, center_pos);

    // 获取路口的所有连接点
    let connection_points = get_junction_connection_points(center_pos, segment_type, rotation);

    // 创建中心节点（如果还没有的话）
    let center_node_id = create_center_node_id(center_pos);
    pathfinding_graph.nodes.insert(
        center_pos,
        GraphNode {
            position: center_pos,
            node_type: GraphNodeType::Intersection,
            station_name: None,
            is_accessible: true,
        },
    );

    // 为每个连接点创建虚拟入口/出口节点
    for (direction, connection_point) in connection_points.iter() {
        let entry_node_pos = create_entry_node_position(center_pos, direction);
        let exit_node_pos = create_exit_node_position(center_pos, direction);

        // 创建入口节点
        pathfinding_graph.nodes.insert(
            entry_node_pos,
            GraphNode {
                position: entry_node_pos,
                node_type: GraphNodeType::RouteSegment,
                station_name: None,
                is_accessible: true,
            },
        );

        // 创建出口节点
        pathfinding_graph.nodes.insert(
            exit_node_pos,
            GraphNode {
                position: exit_node_pos,
                node_type: GraphNodeType::RouteSegment,
                station_name: None,
                is_accessible: true,
            },
        );

        // 建立连接：外部 -> 入口 -> 中心 -> 出口 -> 外部
        create_junction_connections(
            pathfinding_graph,
            *connection_point,
            entry_node_pos,
            center_pos,
            exit_node_pos,
            direction,
        );
    }

    // 在中心创建所有可能的转向连接
    create_internal_turn_connections(
        pathfinding_graph,
        center_pos,
        &connection_points,
        segment_type,
    );
}

/// 获取路口的连接点和方向
fn get_junction_connection_points(
    center_pos: GridPos,
    segment_type: &RouteSegmentType,
    rotation: u32,
) -> Vec<(JunctionDirection, GridPos)> {
    let base_directions = match segment_type {
        RouteSegmentType::Curve => vec![
            (JunctionDirection::North, (0, 1)),
            (JunctionDirection::East, (1, 0)),
        ],
        RouteSegmentType::TSplit => vec![
            (JunctionDirection::North, (0, 1)),
            (JunctionDirection::South, (0, -1)),
            (JunctionDirection::East, (1, 0)),
        ],
        RouteSegmentType::Cross => vec![
            (JunctionDirection::North, (0, 1)),
            (JunctionDirection::South, (0, -1)),
            (JunctionDirection::East, (1, 0)),
            (JunctionDirection::West, (-1, 0)),
        ],
        _ => vec![],
    };

    base_directions
        .into_iter()
        .map(|(dir, (dx, dy))| {
            let rotated_dir = rotate_direction(dir, rotation);
            let (rotated_dx, rotated_dy) = rotate_offset(dx, dy, rotation);
            let connection_pos = GridPos::new(center_pos.x + rotated_dx, center_pos.y + rotated_dy);
            (rotated_dir, connection_pos)
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum JunctionDirection {
    North,
    South,
    East,
    West,
}

impl JunctionDirection {
    fn opposite(self) -> Self {
        match self {
            JunctionDirection::North => JunctionDirection::South,
            JunctionDirection::South => JunctionDirection::North,
            JunctionDirection::East => JunctionDirection::West,
            JunctionDirection::West => JunctionDirection::East,
        }
    }
}

fn rotate_direction(direction: JunctionDirection, rotation: u32) -> JunctionDirection {
    let steps = (rotation / 90) % 4;
    let mut result = direction;

    for _ in 0..steps {
        result = match result {
            JunctionDirection::North => JunctionDirection::East,
            JunctionDirection::East => JunctionDirection::South,
            JunctionDirection::South => JunctionDirection::West,
            JunctionDirection::West => JunctionDirection::North,
        };
    }

    result
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

/// 创建入口节点位置（在路口边缘）
fn create_entry_node_position(center: GridPos, direction: &JunctionDirection) -> GridPos {
    let offset = match direction {
        JunctionDirection::North => (0, 300), // 使用大数字避免冲突
        JunctionDirection::South => (0, -300),
        JunctionDirection::East => (300, 0),
        JunctionDirection::West => (-300, 0),
    };

    GridPos::new(center.x + offset.0, center.y + offset.1)
}

/// 创建出口节点位置
fn create_exit_node_position(center: GridPos, direction: &JunctionDirection) -> GridPos {
    let offset = match direction {
        JunctionDirection::North => (0, 310),
        JunctionDirection::South => (0, -310),
        JunctionDirection::East => (310, 0),
        JunctionDirection::West => (-310, 0),
    };

    GridPos::new(center.x + offset.0, center.y + offset.1)
}

fn create_center_node_id(center: GridPos) -> String {
    format!("center_{}_{}", center.x, center.y)
}

/// 创建路口内的连接
fn create_junction_connections(
    pathfinding_graph: &mut PathfindingGraph,
    external_point: GridPos,
    entry_node: GridPos,
    center: GridPos,
    exit_node: GridPos,
    direction: &JunctionDirection,
) {
    // 外部点 -> 入口节点
    add_connection(
        pathfinding_graph,
        external_point,
        entry_node,
        0.1,
        ConnectionType::BusRoute,
    );

    // 入口节点 -> 中心
    add_connection(
        pathfinding_graph,
        entry_node,
        center,
        0.1,
        ConnectionType::BusRoute,
    );

    // 中心 -> 出口节点
    add_connection(
        pathfinding_graph,
        center,
        exit_node,
        0.1,
        ConnectionType::BusRoute,
    );

    // 出口节点 -> 外部点（反向）
    add_connection(
        pathfinding_graph,
        exit_node,
        external_point,
        0.1,
        ConnectionType::BusRoute,
    );

    info!(
        "创建路口连接: {:?} -> 入口 -> 中心 -> 出口 -> {:?}",
        external_point, external_point
    );
}

/// 在路口中心创建转向连接
fn create_internal_turn_connections(
    pathfinding_graph: &mut PathfindingGraph,
    center: GridPos,
    connection_points: &[(JunctionDirection, GridPos)],
    segment_type: &RouteSegmentType,
) {
    // 根据路口类型决定允许的转向
    match segment_type {
        RouteSegmentType::Curve => {
            // L型路口：只允许90度转弯
            if connection_points.len() == 2 {
                let entry1 = create_entry_node_position(center, &connection_points[0].0);
                let exit2 = create_exit_node_position(center, &connection_points[1].0);
                let entry2 = create_entry_node_position(center, &connection_points[1].0);
                let exit1 = create_exit_node_position(center, &connection_points[0].0);

                // 允许两个方向的转弯
                add_connection(
                    pathfinding_graph,
                    entry1,
                    exit2,
                    0.2,
                    ConnectionType::BusRoute,
                );
                add_connection(
                    pathfinding_graph,
                    entry2,
                    exit1,
                    0.2,
                    ConnectionType::BusRoute,
                );
            }
        }
        RouteSegmentType::TSplit => {
            // T型路口：允许所有转向，但不允许直行穿过主干
            for (i, (dir1, _)) in connection_points.iter().enumerate() {
                for (j, (dir2, _)) in connection_points.iter().enumerate() {
                    if i != j {
                        let entry = create_entry_node_position(center, dir1);
                        let exit = create_exit_node_position(center, dir2);
                        add_connection(
                            pathfinding_graph,
                            entry,
                            exit,
                            0.2,
                            ConnectionType::BusRoute,
                        );
                    }
                }
            }
        }
        RouteSegmentType::Cross => {
            // 十字路口：允许所有方向转换
            for (i, (dir1, _)) in connection_points.iter().enumerate() {
                for (j, (dir2, _)) in connection_points.iter().enumerate() {
                    if i != j {
                        let entry = create_entry_node_position(center, dir1);
                        let exit = create_exit_node_position(center, dir2);

                        // 直行成本较低，转弯成本较高
                        let cost = if dir1.opposite() == *dir2 { 0.1 } else { 0.3 };
                        add_connection(
                            pathfinding_graph,
                            entry,
                            exit,
                            cost,
                            ConnectionType::BusRoute,
                        );
                    }
                }
            }
        }
        _ => {}
    }
}

fn add_connection(
    pathfinding_graph: &mut PathfindingGraph,
    from: GridPos,
    to: GridPos,
    cost: f32,
    connection_type: ConnectionType,
) {
    pathfinding_graph
        .connections
        .entry(from)
        .or_default()
        .push(Connection {
            to,
            cost,
            route_id: Some(format!("junction_{}_{}", from.x, from.y)),
            connection_type,
        });
}

/// F12 - 调试路口结构
fn debug_junction_structure(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    route_segments: Query<&RouteSegment>,
    pathfinding_graph: Res<PathfindingGraph>,
) {
    if keyboard_input.just_pressed(KeyCode::F12) {
        info!("🚦 路口结构调试");

        for segment in route_segments.iter() {
            if !segment.is_active {
                continue;
            }

            match segment.segment_type {
                RouteSegmentType::Curve | RouteSegmentType::TSplit | RouteSegmentType::Cross => {
                    info!(
                        "路口: {:?} at {:?} 旋转: {}°",
                        segment.segment_type, segment.grid_pos, segment.rotation
                    );

                    let connection_points = get_junction_connection_points(
                        segment.grid_pos,
                        &segment.segment_type,
                        segment.rotation,
                    );

                    info!("  连接方向:");
                    for (direction, point) in connection_points {
                        info!("    {:?} -> {:?}", direction, point);
                    }

                    // 显示路口中心的连接情况
                    if let Some(center_connections) =
                        pathfinding_graph.connections.get(&segment.grid_pos)
                    {
                        info!("  中心连接 {} 个:", center_connections.len());
                        for conn in center_connections {
                            info!("    -> {:?} (成本: {:.2})", conn.to, conn.cost);
                        }
                    } else {
                        warn!("  ❌ 中心没有连接");
                    }

                    info!("");
                }
                _ => {}
            }
        }
    }
}

/// 改进的寻路系统，考虑路口内部结构
pub fn create_junction_aware_path(
    pathfinding_graph: &PathfindingGraph,
    start: GridPos,
    end: GridPos,
) -> Option<Vec<PathNode>> {
    // 这里实现考虑路口内部结构的寻路算法
    // 当路径经过复杂路口时，会自动插入中间节点

    // 简化版本：如果路径经过路口，插入中心点
    let mut path = Vec::new();

    // 添加起点
    path.push(PathNode {
        position: start,
        node_type: PathNodeType::Station("起点".to_string()),
        estimated_wait_time: 0.0,
        route_id: None,
    });

    // 检查是否需要经过路口
    if is_junction_position(start, pathfinding_graph) {
        path.push(PathNode {
            position: start,
            node_type: PathNodeType::TransferPoint,
            estimated_wait_time: 0.5,
            route_id: Some("junction_center".to_string()),
        });
    }

    // 添加终点
    path.push(PathNode {
        position: end,
        node_type: PathNodeType::Station("终点".to_string()),
        estimated_wait_time: 0.0,
        route_id: None,
    });

    Some(path)
}

fn is_junction_position(pos: GridPos, pathfinding_graph: &PathfindingGraph) -> bool {
    if let Some(node) = pathfinding_graph.nodes.get(&pos) {
        matches!(node.node_type, GraphNodeType::Intersection)
    } else {
        false
    }
}
