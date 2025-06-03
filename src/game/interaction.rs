use crate::{
    game::grid::{
        Direction, GridConfig, GridPosition, GridState, RouteSegment, RouteSegmentComponent,
        spawn_route_segment,
    },
    screens::Screen,
};
use bevy::{input::mouse::MouseButtonInput, prelude::*, window::PrimaryWindow};

// Resource to track current mouse state and selected tool
#[derive(Resource, Default)]
pub struct MouseState {
    pub world_position: Vec2,
    pub grid_position: GridPosition,
    pub is_dragging: bool,
    pub drag_start_pos: Option<GridPosition>,
    pub selected_entity: Option<Entity>,
}

// Resource for current selected tool/segment type
#[derive(Resource)]
pub struct SelectedTool {
    pub segment_type: RouteSegment,
    pub direction: Direction,
}

impl Default for SelectedTool {
    fn default() -> Self {
        Self {
            segment_type: RouteSegment::Straight,
            direction: Direction::North,
        }
    }
}

// Component to mark entities as selectable/draggable
#[derive(Component)]
pub struct Selectable;

// Component to mark entities as hoverable (shows preview)
#[derive(Component)]
pub struct Hoverable;

// Component for preview/ghost entities
#[derive(Component)]
pub struct Preview;

// Mouse interaction events
#[derive(Event)]
pub struct PlaceSegmentEvent {
    pub position: GridPosition,
    pub segment_type: RouteSegment,
    pub direction: Direction,
}

#[derive(Event)]
pub struct SelectEntityEvent {
    pub entity: Entity,
    pub position: GridPosition,
}

#[derive(Event)]
pub struct RotateSegmentEvent {
    pub entity: Entity,
}

// System to update mouse world position
pub fn update_mouse_position_system(
    mut mouse_state: ResMut<MouseState>,
    window: Single<&Window, With<PrimaryWindow>>,
    q_camera: Single<(&Camera, &GlobalTransform)>,
    grid_config: Res<GridConfig>,
) {
    let (camera, camera_transform) = q_camera.into_inner();

    if let Some(cursor_position) = window.cursor_position() {
        // Convert screen coordinates to world coordinates
        if let Ok(world_position) = camera.viewport_to_world_2d(camera_transform, cursor_position) {
            mouse_state.world_position = world_position;
            mouse_state.grid_position = grid_config.world_to_grid(world_position);
        }
    }
}

// System to handle mouse button events
pub fn mouse_button_system(
    mut mouse_button_events: EventReader<MouseButtonInput>,
    mut mouse_state: ResMut<MouseState>,
    mut place_segment_events: EventWriter<PlaceSegmentEvent>,
    mut select_entity_events: EventWriter<SelectEntityEvent>,
    mut rotate_segment_events: EventWriter<RotateSegmentEvent>,
    grid_state: Res<GridState>,
    selected_tool: Res<SelectedTool>,
    grid_config: Res<GridConfig>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    for event in mouse_button_events.read() {
        if !event.state.is_pressed() {
            continue;
        }

        let grid_pos = mouse_state.grid_position;

        // Check if position is valid
        if !grid_config.is_valid_position(grid_pos) {
            continue;
        }

        match event.button {
            MouseButton::Left => {
                // Check if holding Shift for rotation
                if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
                    if let Some(entity) = grid_state.get_entity(grid_pos) {
                        rotate_segment_events.write(RotateSegmentEvent { entity });
                    }
                }
                // Check if position is occupied for selection
                else if let Some(entity) = grid_state.get_entity(grid_pos) {
                    select_entity_events.write(SelectEntityEvent {
                        entity,
                        position: grid_pos,
                    });
                    mouse_state.selected_entity = Some(entity);
                }
                // Place new segment if position is empty
                else {
                    place_segment_events.write(PlaceSegmentEvent {
                        position: grid_pos,
                        segment_type: selected_tool.segment_type,
                        direction: selected_tool.direction,
                    });
                }
            }
            MouseButton::Right => {
                // Right click to remove segment
                if let Some(entity) = grid_state.get_entity(grid_pos) {
                    // Will be handled by remove_segment_system
                }
            }
            _ => {}
        }
    }
}

// System to handle segment placement
pub fn place_segment_system(
    mut commands: Commands,
    mut place_segment_events: EventReader<PlaceSegmentEvent>,
    mut grid_state: ResMut<GridState>,
    asset_server: Res<AssetServer>,
) {
    for event in place_segment_events.read() {
        let entity = spawn_route_segment(
            &mut commands,
            event.position,
            event.segment_type,
            event.direction,
            &asset_server,
        );

        // Add to grid state
        grid_state.place_entity(event.position, entity);
        grid_state.place_route_segment(
            event.position,
            RouteSegmentComponent {
                segment_type: event.segment_type,
                direction: event.direction,
            },
        );

        // Add selectable component
        commands.entity(entity).insert(Selectable);
    }
}

// System to handle entity selection
pub fn select_entity_system(
    mut select_entity_events: EventReader<SelectEntityEvent>,
    mut query: Query<&mut Sprite>,
) {
    for event in select_entity_events.read() {
        if let Ok(mut sprite) = query.get_mut(event.entity) {
            // Highlight selected entity
            sprite.color = Color::srgb(1.0, 1.0, 0.5); // Yellow tint
        }
    }
}

// System to handle segment rotation
pub fn rotate_segment_system(
    mut rotate_segment_events: EventReader<RotateSegmentEvent>,
    mut query: Query<(&mut Transform, &mut RouteSegmentComponent)>,
    mut grid_state: ResMut<GridState>,
    grid_positions: Query<&GridPosition>,
) {
    for event in rotate_segment_events.read() {
        if let Ok((mut transform, mut route_segment)) = query.get_mut(event.entity) {
            // Rotate direction
            route_segment.direction = route_segment.direction.rotate_cw();

            // Update visual rotation
            let rotation_angle = route_segment.direction as u8 as f32 * std::f32::consts::PI / 2.0;
            transform.rotation = Quat::from_rotation_z(rotation_angle);

            // Update grid state
            if let Ok(grid_pos) = grid_positions.get(event.entity) {
                grid_state.place_route_segment(*grid_pos, route_segment.clone());
            }
        }
    }
}

// System to show preview/ghost segment at mouse position
pub fn preview_system(
    mut commands: Commands,
    mouse_state: Res<MouseState>,
    selected_tool: Res<SelectedTool>,
    grid_config: Res<GridConfig>,
    grid_state: Res<GridState>,
    asset_server: Res<AssetServer>,
    preview_query: Query<Entity, With<Preview>>,
) {
    // Remove existing preview
    for entity in preview_query.iter() {
        commands.entity(entity).despawn();
    }

    let grid_pos = mouse_state.grid_position;

    // Only show preview if position is valid and empty
    if grid_config.is_valid_position(grid_pos) && !grid_state.is_occupied(grid_pos) {
        let texture_path = match selected_tool.segment_type {
            RouteSegment::Straight => "sprites/road_straight.png",
            RouteSegment::Corner => "sprites/road_corner.png",
            RouteSegment::TJunction => "sprites/road_t_junction.png",
            RouteSegment::Cross => "sprites/road_cross.png",
            RouteSegment::Station => "sprites/bus_station.png",
            RouteSegment::Grass => "sprites/grass.png",
        };

        let world_pos = grid_config.grid_to_world(grid_pos);

        commands.spawn((
            Sprite {
                image: asset_server.load(texture_path),
                color: Color::srgba(1.0, 1.0, 1.0, 0.5),
                ..default()
            },
            Transform {
                translation: Vec3::new(world_pos.x, world_pos.y, -1.0), // Behind other sprites
                rotation: Quat::from_rotation_z(
                    selected_tool.direction as u8 as f32 * std::f32::consts::PI / 2.0,
                ),
                ..default()
            },
            Preview,
        ));
    }
}

// System to handle keyboard input for tool selection
pub fn tool_selection_system(
    mut selected_tool: ResMut<SelectedTool>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    // Number keys to select segment type
    if keys.just_pressed(KeyCode::Digit1) {
        selected_tool.segment_type = RouteSegment::Straight;
    }
    if keys.just_pressed(KeyCode::Digit2) {
        selected_tool.segment_type = RouteSegment::Corner;
    }
    if keys.just_pressed(KeyCode::Digit3) {
        selected_tool.segment_type = RouteSegment::TJunction;
    }
    if keys.just_pressed(KeyCode::Digit4) {
        selected_tool.segment_type = RouteSegment::Cross;
    }
    if keys.just_pressed(KeyCode::Digit5) {
        selected_tool.segment_type = RouteSegment::Station;
    }
    if keys.just_pressed(KeyCode::Digit6) {
        selected_tool.segment_type = RouteSegment::Grass;
    }

    // R key to rotate current tool direction
    if keys.just_pressed(KeyCode::KeyR) {
        selected_tool.direction = selected_tool.direction.rotate_cw();
    }
}

// System to remove segments on right click
pub fn remove_segment_system(
    mut commands: Commands,
    mut mouse_button_events: EventReader<MouseButtonInput>,
    mouse_state: ResMut<MouseState>,
    mut grid_state: ResMut<GridState>,
    grid_config: Res<GridConfig>,
) {
    for event in mouse_button_events.read() {
        if event.button == MouseButton::Right && event.state.is_pressed() {
            let grid_pos = mouse_state.grid_position;

            if grid_config.is_valid_position(grid_pos) {
                if let Some(entity) = grid_state.remove_entity(grid_pos) {
                    commands.entity(entity).despawn();
                    grid_state.route_segments.remove(&grid_pos);
                }
            }
        }
    }
}

// System to clear selection when clicking empty space
pub fn clear_selection_system(
    mut mouse_button_events: EventReader<MouseButtonInput>,
    mut mouse_state: ResMut<MouseState>,
    grid_state: Res<GridState>,
    mut query: Query<&mut Sprite, With<Selectable>>,
) {
    for event in mouse_button_events.read() {
        if event.button == MouseButton::Left && event.state.is_pressed() {
            let grid_pos = mouse_state.grid_position;

            // If clicking on empty space, clear selection
            if !grid_state.is_occupied(grid_pos) {
                if let Some(selected_entity) = mouse_state.selected_entity {
                    // Remove highlight from previously selected entity
                    if let Ok(mut sprite) = query.get_mut(selected_entity) {
                        sprite.color = Color::WHITE;
                    }
                }
                mouse_state.selected_entity = None;
            }
        }
    }
}

// Plugin to register all mouse interaction systems
pub struct MouseInteractionPlugin;

impl Plugin for MouseInteractionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MouseState>()
            .init_resource::<SelectedTool>()
            .add_event::<PlaceSegmentEvent>()
            .add_event::<SelectEntityEvent>()
            .add_event::<RotateSegmentEvent>()
            .add_systems(
                Update,
                (
                    update_mouse_position_system,
                    mouse_button_system,
                    place_segment_system,
                    select_entity_system,
                    rotate_segment_system,
                    preview_system,
                    tool_selection_system,
                    remove_segment_system,
                    clear_selection_system,
                )
                    .chain()
                    .run_if(in_state(Screen::Gameplay)), // Ensure proper execution order
            );
    }
}
