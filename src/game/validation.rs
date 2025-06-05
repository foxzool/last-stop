use crate::{
    game::{
        grid::{Direction, GridPos, GridState, RouteSegmentComponent, RouteSegmentType},
        interaction::PlaceSegmentEvent,
    },
    screens::Screen,
};
use bevy::prelude::*;
use std::collections::{HashMap, HashSet};

// 连接点表示路线可以连接的位置
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConnectionPoint {
    pub position: GridPos,
    pub direction: Direction,
}

impl ConnectionPoint {
    pub fn new(position: GridPos, direction: Direction) -> Self {
        Self {
            position,
            direction,
        }
    }

    // 获取此连接点应该连接到的目标连接点
    pub fn get_target(&self) -> ConnectionPoint {
        let (dx, dy) = self.direction.to_offset();
        let target_pos = GridPos::new(self.position.x + dx, self.position.y + dy);
        ConnectionPoint::new(target_pos, self.direction.opposite())
    }
}

// 用于标记无效连接并提供视觉反馈的组件
#[derive(Component)]
pub struct InvalidConnection;

// 用于标记有效连接的组件
#[derive(Component)]
pub struct ValidConnection;

// 用于跟踪网格中所有连接的资源
#[derive(Resource, Default)]
pub struct ConnectionMap {
    pub connections: HashMap<ConnectionPoint, ConnectionPoint>,
    pub invalid_segments: HashSet<GridPos>,
    pub isolated_segments: HashSet<GridPos>,
}

impl ConnectionMap {
    // 在两个点之间添加双向连接
    pub fn add_connection(&mut self, point1: ConnectionPoint, point2: ConnectionPoint) {
        self.connections.insert(point1, point2);
        self.connections.insert(point2, point1);
    }

    // 移除涉及特定位置的所有连接
    pub fn remove_connections_at(&mut self, position: GridPos) {
        self.connections
            .retain(|point, _| point.position != position);
    }

    // 检查连接点是否有有效连接
    pub fn has_connection(&self, point: &ConnectionPoint) -> bool {
        self.connections.contains_key(point)
    }

    // 从网格位置获取所有连接点
    pub fn get_connection_points(
        &self,
        position: GridPos,
        segment: &RouteSegmentComponent,
    ) -> Vec<ConnectionPoint> {
        match segment.segment_type {
            RouteSegmentType::Straight => {
                // 直线段在两个相反方向上连接
                vec![
                    ConnectionPoint::new(position, segment.direction),
                    ConnectionPoint::new(position, segment.direction.opposite()),
                ]
            }
            RouteSegmentType::Turn => {
                // 转角段在两个垂直方向上连接
                vec![
                    ConnectionPoint::new(position, segment.direction),
                    ConnectionPoint::new(position, segment.direction.rotate_cw()),
                ]
            }
            RouteSegmentType::TSplit => {
                // T型路口在三个方向上连接
                vec![
                    ConnectionPoint::new(position, segment.direction),
                    ConnectionPoint::new(position, segment.direction.rotate_cw()),
                    ConnectionPoint::new(
                        position,
                        segment.direction.rotate_cw().rotate_cw().rotate_cw(),
                    ),
                ]
            }
            RouteSegmentType::Cross => {
                // 十字路口在所有四个方向上连接
                vec![
                    ConnectionPoint::new(position, Direction::North),
                    ConnectionPoint::new(position, Direction::East),
                    ConnectionPoint::new(position, Direction::South),
                    ConnectionPoint::new(position, Direction::West),
                ]
            }
            RouteSegmentType::DeadEnd => {
                // 车站路口在所有四个方向上连接
                vec![
                    ConnectionPoint::new(position, Direction::North),
                    ConnectionPoint::new(position, Direction::East),
                    ConnectionPoint::new(position, Direction::South),
                    ConnectionPoint::new(position, Direction::West),
                ]
            }
        }
    }
}

// 系统：验证网格中所有路线分段的连接。
// 该系统会执行以下操作：
// 1. 清除先前所有连接的验证状态（包括连接映射和无效/孤立路段的标记）。
// 2. 移除所有路段实体上的 `InvalidConnection` 和 `ValidConnection` 组件。
// 3. 遍历所有带有 `RouteSegmentComponent` 的实体（即路线分段）：
//    a. 获取每个路段的连接点。
//    b. 对于每个连接点，计算其目标连接点和目标位置。
//    c. 检查目标位置是否存在另一个路段（非草地等地形）。
//    d. 如果目标路段存在且其连接点与当前路段的目标连接点匹配，则在 `ConnectionMap` 中添加此双向连接。
// 4. 再次遍历所有路段实体，以确定其最终的连接状态：
//    a. 检查其每个连接点是否在 `ConnectionMap` 中有记录（即有效连接）。
//    b. 如果任何连接点指向一个存在路段但无法连接的位置（例如，方向不匹配或目标路段是草地但被错误地视为可连接），则标记为无效连接。
//    c. 根据是否有有效连接或无效连接，为实体添加 `ValidConnection` 或 `InvalidConnection` 组件。
//    d. 如果一个路段没有任何连接（既没有有效连接也没有无效连接，例如一个孤立的直路），则将其记录到 `isolated_segments` 中。
pub fn validate_connections_system(
    mut connection_map: ResMut<ConnectionMap>,
    mut commands: Commands,
    grid_state: Res<GridState>,
    query: Query<(Entity, &GridPos), With<RouteSegmentComponent>>,
) {
    // Clear previous validation state
    connection_map.connections.clear();
    connection_map.invalid_segments.clear();
    connection_map.isolated_segments.clear();

    // Remove all validation components
    for (entity, _) in query.iter() {
        commands.entity(entity).remove::<InvalidConnection>();
        commands.entity(entity).remove::<ValidConnection>();
    }

    // Build connection map (only for route elements, not terrain)
    for (_entity, grid_pos) in query.iter() {
        if let Some(segment) = grid_state.get_route_segment(*grid_pos) {
            let connection_points = connection_map.get_connection_points(*grid_pos, segment);

            for point in connection_points {
                let target = point.get_target();

                // Check if target position has a route segment (not terrain)
                if let Some(target_segment) = grid_state.get_route_segment(target.position) {
                    // // Skip if target is grass (terrain element)
                    // if target_segment.segment_type == RouteSegmentType::Grass {
                    //     continue;
                    // } fixme

                    let target_points =
                        connection_map.get_connection_points(target.position, target_segment);

                    // Check if target segment has a matching connection point
                    if target_points.contains(&target) {
                        connection_map.add_connection(point, target);
                    }
                }
            }
        }
    }

    // Validate each segment (only route elements)
    for (entity, grid_pos) in query.iter() {
        if let Some(segment) = grid_state.get_route_segment(*grid_pos) {
            let connection_points = connection_map.get_connection_points(*grid_pos, segment);
            let mut has_valid_connection = false;
            let mut has_invalid_connection = false;

            for point in connection_points {
                if connection_map.has_connection(&point) {
                    has_valid_connection = true;
                } else {
                    // Check if there's a route segment (not grass) at target position but no valid connection
                    let target = point.get_target();
                    if let Some(target_segment) = grid_state.get_route_segment(target.position) {
                        // if target_segment.segment_type != RouteSegmentType::Grass {
                        //     has_invalid_connection = true;
                        // } fixme
                    }
                }
            }

            // Apply validation components
            if has_invalid_connection {
                commands.entity(entity).insert(InvalidConnection);
                connection_map.invalid_segments.insert(*grid_pos);
            } else if has_valid_connection {
                commands.entity(entity).insert(ValidConnection);
            } else {
                // Isolated segment (no connections at all)
                connection_map.isolated_segments.insert(*grid_pos);
            }
        }
    }
}

// 为连接验证提供视觉反馈的系统
pub fn connection_visual_feedback_system(
    mut query: Query<(
        &mut Sprite,
        &RouteSegmentComponent,
        Option<&InvalidConnection>,
        Option<&ValidConnection>,
    )>,
) {
    // for (mut sprite, segment, invalid, valid) in query.iter_mut() {
    //     match segment.segment_type {
    //         RouteSegmentType::Grass => {
    //             // Grass always stays green
    //             sprite.color = Color::srgb(0.4, 0.8, 0.4);
    //         }
    //         _ => {
    //             // Route elements get validation colors
    //             if invalid.is_some() {
    //                 // Red tint for invalid connections
    //                 sprite.color = Color::srgb(1.0, 0.3, 0.3);
    //             } else if valid.is_some() {
    //                 // Green tint for valid connections
    //                 sprite.color = Color::srgb(0.3, 1.0, 0.3);
    //             } else {
    //                 // White for isolated segments (not connected to anything)
    //                 sprite.color = Color::WHITE;
    //             }
    //         }
    //     }
    // }
}

// 查找连接的路线网络的系统
pub fn find_route_networks_system(
    connection_map: Res<ConnectionMap>,
    grid_state: Res<GridState>,
    mut network_events: EventWriter<RouteNetworkEvent>,
) {
    let mut visited = HashSet::new();
    let mut networks = Vec::new();

    // Find all segments with route components
    for (pos, _) in grid_state.route_segments.iter() {
        if visited.contains(pos) {
            continue;
        }

        // Start a new network from this position
        let mut network = HashSet::new();
        let mut stack = vec![*pos];

        while let Some(current_pos) = stack.pop() {
            if visited.contains(&current_pos) {
                continue;
            }

            visited.insert(current_pos);
            network.insert(current_pos);

            // Find all connected segments
            if let Some(segment) = grid_state.get_route_segment(current_pos) {
                let connection_points = connection_map.get_connection_points(current_pos, segment);

                for point in connection_points {
                    if let Some(target) = connection_map.connections.get(&point) {
                        if !visited.contains(&target.position) {
                            stack.push(target.position);
                        }
                    }
                }
            }
        }

        if !network.is_empty() {
            networks.push(network);
        }
    }

    // Send network events
    for (index, network) in networks.iter().enumerate() {
        network_events.write(RouteNetworkEvent {
            network_id: index,
            positions: network.clone(),
        });
    }
}

// 路线网络发现的事件
#[derive(Event)]
pub struct RouteNetworkEvent {
    pub network_id: usize,
    pub positions: HashSet<GridPos>,
}

// 检查放置是否会创建有效连接的系统
pub fn validate_placement_system(
    grid_state: Res<GridState>,
    connection_map: Res<ConnectionMap>,
    mut placement_events: EventReader<PlaceSegmentEvent>,
    mut validation_events: EventWriter<PlacementValidationEvent>,
) {
    for event in placement_events.read() {
        let temp_segment = RouteSegmentComponent {
            segment_type: event.segment_type,
            direction: event.direction,
        };

        let connection_points = connection_map.get_connection_points(event.position, &temp_segment);
        let mut valid_connections = 0;
        let mut invalid_connections = 0;

        for point in connection_points {
            let target = point.get_target();

            if let Some(target_segment) = grid_state.get_route_segment(target.position) {
                let target_points =
                    connection_map.get_connection_points(target.position, target_segment);

                if target_points.contains(&target) {
                    valid_connections += 1;
                } else {
                    invalid_connections += 1;
                }
            }
        }

        validation_events.write(PlacementValidationEvent {
            position: event.position,
            segment_type: event.segment_type,
            direction: event.direction,
            valid_connections,
            invalid_connections,
            is_valid: invalid_connections == 0,
        });
    }
}

// 放置验证结果的事件
#[derive(Event)]
pub struct PlacementValidationEvent {
    pub position: GridPos,
    pub segment_type: RouteSegmentType,
    pub direction: Direction,
    pub valid_connections: usize,
    pub invalid_connections: usize,
    pub is_valid: bool,
}

// 防止无效放置的系统（可选 - 可以为了谜题的灵活性而禁用）
pub fn prevent_invalid_placement_system(
    mut placement_events: EventReader<PlacementValidationEvent>,
    mut commands: Commands,
) {
    for event in placement_events.read() {
        if !event.is_valid {
            info!(
                "在 {:?} 处的无效放置: {} 个无效连接",
                event.position, event.invalid_connections
            );
            // Could prevent placement here if desired
            // For puzzle games, might want to allow invalid placements initially
        }
    }
}

// 检查两个路段是否可以连接的辅助函数
pub fn can_segments_connect(
    pos1: GridPos,
    segment1: &RouteSegmentComponent,
    pos2: GridPos,
    segment2: &RouteSegmentComponent,
    connection_map: &ConnectionMap, /* This parameter's state is not directly used by get_connection_points */
) -> bool {
    // get_connection_points could be a static/helper method or a method on RouteSegmentComponent
    // as it doesn't rely on the internal state of 'connection_map' instance here.
    let points1 = ConnectionMap::default().get_connection_points(pos1, segment1);
    let points2 = ConnectionMap::default().get_connection_points(pos2, segment2);

    trace!(
        "can_segments_connect: pos1={:?}, seg1={:?}/{:?}, points1={:?}",
        pos1, segment1.segment_type, segment1.direction, points1
    );
    trace!(
        "can_segments_connect: pos2={:?}, seg2={:?}/{:?}, points2={:?}",
        pos2, segment2.segment_type, segment2.direction, points2
    );

    for point1 in points1 {
        let target = point1.get_target();
        let contains_target = points2.contains(&target);
        trace!(
            "can_segments_connect: point1={:?}, target={:?}, points2_contains_target={}",
            point1, target, contains_target
        );
        if target.position == pos2 && contains_target {
            trace!("can_segments_connect: Found connection!");
            return true;
        }
    }
    trace!("can_segments_connect: No connection found.");
    false
}

// 注册所有连接验证系统的插件
pub struct ConnectionValidationPlugin;

impl Plugin for ConnectionValidationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ConnectionMap>()
            .add_event::<RouteNetworkEvent>()
            .add_event::<PlacementValidationEvent>()
            .add_systems(
                Update,
                (
                    validate_connections_system,
                    connection_visual_feedback_system,
                    find_route_networks_system,
                    validate_placement_system,
                    prevent_invalid_placement_system,
                )
                    .chain()
                    .run_if(in_state(Screen::Gameplay)),
            );
    }
}
