use crate::game::{
    grid::{Direction, GridPosition, GridState, RouteSegment, RouteSegmentComponent},
    interaction::PlaceSegmentEvent,
};
use bevy::prelude::*;
use std::collections::{HashMap, HashSet};

// Connection point represents where routes can connect
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConnectionPoint {
    pub position: GridPosition,
    pub direction: Direction,
}

impl ConnectionPoint {
    pub fn new(position: GridPosition, direction: Direction) -> Self {
        Self {
            position,
            direction,
        }
    }

    // Get the connection point this one should connect to
    pub fn get_target(&self) -> ConnectionPoint {
        let (dx, dy) = self.direction.to_offset();
        let target_pos = GridPosition::new(self.position.x + dx, self.position.y + dy);
        ConnectionPoint::new(target_pos, self.direction.opposite())
    }
}

// Component to mark invalid connections with visual feedback
#[derive(Component)]
pub struct InvalidConnection;

// Component to mark valid connections
#[derive(Component)]
pub struct ValidConnection;

// Resource to track all connections in the grid
#[derive(Resource, Default)]
pub struct ConnectionMap {
    pub connections: HashMap<ConnectionPoint, ConnectionPoint>,
    pub invalid_segments: HashSet<GridPosition>,
    pub isolated_segments: HashSet<GridPosition>,
}

impl ConnectionMap {
    // Add a bidirectional connection between two points
    pub fn add_connection(&mut self, point1: ConnectionPoint, point2: ConnectionPoint) {
        self.connections.insert(point1, point2);
        self.connections.insert(point2, point1);
    }

    // Remove all connections involving a specific position
    pub fn remove_connections_at(&mut self, position: GridPosition) {
        self.connections
            .retain(|point, _| point.position != position);
    }

    // Check if a connection point has a valid connection
    pub fn has_connection(&self, point: &ConnectionPoint) -> bool {
        self.connections.contains_key(point)
    }

    // Get all connection points from a grid position
    pub fn get_connection_points(
        &self,
        position: GridPosition,
        segment: &RouteSegmentComponent,
    ) -> Vec<ConnectionPoint> {
        match segment.segment_type {
            RouteSegment::Straight => {
                // Straight segments connect in two opposite directions
                vec![
                    ConnectionPoint::new(position, segment.direction),
                    ConnectionPoint::new(position, segment.direction.opposite()),
                ]
            }
            RouteSegment::Corner => {
                // Corner segments connect in two perpendicular directions
                vec![
                    ConnectionPoint::new(position, segment.direction),
                    ConnectionPoint::new(position, segment.direction.rotate_cw()),
                ]
            }
            RouteSegment::TJunction => {
                // T-junction connects in three directions
                vec![
                    ConnectionPoint::new(position, segment.direction),
                    ConnectionPoint::new(position, segment.direction.rotate_cw()),
                    ConnectionPoint::new(
                        position,
                        segment.direction.rotate_cw().rotate_cw().rotate_cw(),
                    ),
                ]
            }
            RouteSegment::Cross => {
                // Cross connects in all four directions
                vec![
                    ConnectionPoint::new(position, Direction::North),
                    ConnectionPoint::new(position, Direction::East),
                    ConnectionPoint::new(position, Direction::South),
                    ConnectionPoint::new(position, Direction::West),
                ]
            }
            RouteSegment::Station => {
                // Station connects in two opposite directions (like straight)
                vec![
                    ConnectionPoint::new(position, segment.direction),
                    ConnectionPoint::new(position, segment.direction.opposite()),
                ]
            }
            RouteSegment::Grass => {
                // Grass doesn't connect to anything
                vec![]
            }
        }
    }
}

// System to validate all route connections
pub fn validate_connections_system(
    mut connection_map: ResMut<ConnectionMap>,
    mut commands: Commands,
    grid_state: Res<GridState>,
    query: Query<(Entity, &GridPosition), With<RouteSegmentComponent>>,
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

    // Build connection map
    for (entity, grid_pos) in query.iter() {
        if let Some(segment) = grid_state.get_route_segment(*grid_pos) {
            let connection_points = connection_map.get_connection_points(*grid_pos, segment);

            for point in connection_points {
                let target = point.get_target();

                // Check if target position has a segment that can connect
                if let Some(target_segment) = grid_state.get_route_segment(target.position) {
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

    // Validate each segment
    for (entity, grid_pos) in query.iter() {
        if let Some(segment) = grid_state.get_route_segment(*grid_pos) {
            let connection_points = connection_map.get_connection_points(*grid_pos, segment);
            let mut has_valid_connection = false;
            let mut has_invalid_connection = false;

            for point in connection_points {
                if connection_map.has_connection(&point) {
                    has_valid_connection = true;
                } else {
                    // Check if there's a segment at target position but no valid connection
                    let target = point.get_target();
                    if grid_state.get_route_segment(target.position).is_some() {
                        has_invalid_connection = true;
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

// System to provide visual feedback for connection validation
pub fn connection_visual_feedback_system(
    mut query: Query<(
        &mut Sprite,
        Option<&InvalidConnection>,
        Option<&ValidConnection>,
    )>,
) {
    for (mut sprite, invalid, valid) in query.iter_mut() {
        if invalid.is_some() {
            // Red tint for invalid connections
            sprite.color = Color::srgb(1.0, 0.3, 0.3);
        } else if valid.is_some() {
            // Green tint for valid connections
            sprite.color = Color::srgb(0.3, 1.0, 0.3);
        } else {
            // White for isolated segments (not connected to anything)
            sprite.color = Color::WHITE;
        }
    }
}

// System to find connected route networks
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
        network_events.send(RouteNetworkEvent {
            network_id: index,
            positions: network.clone(),
        });
    }
}

// Event for route network discovery
#[derive(Event)]
pub struct RouteNetworkEvent {
    pub network_id: usize,
    pub positions: HashSet<GridPosition>,
}

// System to check if placement would create valid connections
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

// Event for placement validation results
#[derive(Event)]
pub struct PlacementValidationEvent {
    pub position: GridPosition,
    pub segment_type: RouteSegment,
    pub direction: Direction,
    pub valid_connections: usize,
    pub invalid_connections: usize,
    pub is_valid: bool,
}

// System to prevent invalid placements (optional - can be disabled for puzzle flexibility)
pub fn prevent_invalid_placement_system(
    mut placement_events: EventReader<PlacementValidationEvent>,
    mut commands: Commands,
) {
    for event in placement_events.read() {
        if !event.is_valid {
            info!(
                "Invalid placement at {:?}: {} invalid connections",
                event.position, event.invalid_connections
            );
            // Could prevent placement here if desired
            // For puzzle games, might want to allow invalid placements initially
        }
    }
}

// Helper function to check if two segments can connect
pub fn can_segments_connect(
    pos1: GridPosition,
    segment1: &RouteSegmentComponent,
    pos2: GridPosition,
    segment2: &RouteSegmentComponent,
    connection_map: &ConnectionMap,
) -> bool {
    let points1 = connection_map.get_connection_points(pos1, segment1);
    let points2 = connection_map.get_connection_points(pos2, segment2);

    for point1 in points1 {
        let target = point1.get_target();
        if target.position == pos2 && points2.contains(&target) {
            return true;
        }
    }
    false
}

// Plugin to register all connection validation systems
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
                    .chain(),
            );
    }
}
