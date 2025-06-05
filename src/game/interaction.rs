use crate::{
    game::{
        grid::{
            Direction, GridConfig, GridPosition, GridState, RouteSegment, RouteSegmentComponent,
            spawn_route_segment,
        },
        passenger::{Passenger, RequestPathReplanEvent},
    },
    screens::Screen,
};
use bevy::{input::mouse::MouseButtonInput, prelude::*, window::PrimaryWindow}; // Added for passenger replanning

// 注册所有鼠标交互系统的插件
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
                    mouse_button_system,  // Handles press/release, initiates drag
                    drag_place_system,    // Handles continuous placement during drag
                    place_segment_system, // Consumes PlaceSegmentEvent
                    select_entity_system,
                    rotate_segment_system,
                    preview_system,
                    tool_selection_system,
                    remove_segment_system,  // Handles right-click
                    clear_selection_system, // Handles left-click on empty to clear selection
                )
                    .chain()
                    .run_if(in_state(Screen::Gameplay)),
            );
    }
}

// 用于跟踪当前鼠标状态和选定工具的资源
#[derive(Resource, Default)]
pub struct MouseState {
    pub world_position: Vec2,
    pub grid_position: GridPosition,
    pub is_dragging: bool,
    pub drag_start_pos: Option<GridPosition>,
    pub selected_entity: Option<Entity>,
    pub last_drag_grid_position: Option<GridPosition>, // Added for drag placement
}

// 当前选定工具/路段类型的资源
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

// 标记实体为可选择/可拖动的组件
#[derive(Component)]
pub struct Selectable;

// 标记实体为可悬停的组件（显示预览）
#[derive(Component)]
pub struct Hoverable;

// 预览/幽灵实体的组件
#[derive(Component)]
pub struct Preview;

// 鼠标交互事件
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

// 更新鼠标世界位置的系统
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

// 处理鼠标按钮事件的系统
pub fn mouse_button_system(
    mut mouse_button_events: EventReader<MouseButtonInput>,
    mut mouse_state: ResMut<MouseState>,
    mut place_segment_events: EventWriter<PlaceSegmentEvent>,
    mut select_entity_events: EventWriter<SelectEntityEvent>,
    mut rotate_segment_events: EventWriter<RotateSegmentEvent>,
    grid_state: Res<GridState>,
    selected_tool: Res<SelectedTool>,
    grid_config: Res<GridConfig>,
    keys: Res<ButtonInput<KeyCode>>, // For Shift+Click
) {
    for event in mouse_button_events.read() {
        let grid_pos = mouse_state.grid_position;

        if event.button == MouseButton::Left {
            if event.state.is_pressed() {
                // Left Mouse Button PRESSED
                if !grid_config.is_valid_position(grid_pos) {
                    // Clicked outside valid grid area, do nothing.
                    // clear_selection_system might handle clicks on truly empty (non-grid) areas.
                    continue;
                }

                if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
                    // Handle rotation if Shift is pressed and cell is occupied
                    if grid_state.is_occupied(grid_pos) {
                        if let Some(entity_to_rotate) = grid_state.get_entity(grid_pos) {
                            rotate_segment_events.write(RotateSegmentEvent {
                                entity: entity_to_rotate,
                            });
                        }
                    }
                    // If shift-clicking on empty, do nothing (don't start drag, don't select)
                } else if grid_state.is_occupied(grid_pos) {
                    // Handle selection if cell is occupied and not shift-clicking
                    if let Some(entity_to_select) = grid_state.get_entity(grid_pos) {
                        select_entity_events.write(SelectEntityEvent {
                            entity: entity_to_select,
                            position: grid_pos,
                        });
                        mouse_state.selected_entity = Some(entity_to_select);
                    }
                } else {
                    // Cell is not occupied, and not shift-clicking: Start placing/dragging
                    // This implies it's a valid, empty grid cell.

                    // Clear conceptual selection state, as we are starting a new build action.
                    if mouse_state.selected_entity.is_some() {
                        mouse_state.selected_entity = None;
                        // Note: This does not visually deselect the previously selected entity.
                        // Visual deselection happens in clear_selection_system if clicking on an *already* empty cell,
                        // or in select_entity_system when a new entity is selected.
                    }

                    mouse_state.is_dragging = true;
                    mouse_state.drag_start_pos = Some(grid_pos);
                    // Mark this cell as "placed" for the current drag session to avoid double placement by drag_place_system immediately.
                    mouse_state.last_drag_grid_position = Some(grid_pos);

                    // Place the first segment for the drag operation
                    place_segment_events.write(PlaceSegmentEvent {
                        position: grid_pos,
                        segment_type: selected_tool.segment_type,
                        direction: selected_tool.direction,
                    });
                }
            } else {
                // Left Mouse Button RELEASED
                if mouse_state.is_dragging {
                    // Only act if a drag was in progress
                    mouse_state.is_dragging = false;
                    mouse_state.drag_start_pos = None;
                    // Reset last_drag_grid_position to ensure the next click/drag starts fresh.
                    mouse_state.last_drag_grid_position = None;
                }
            }
        }
        // Right click for removal is handled by remove_segment_system.
        // Middle click is not currently handled in this system.
    }
}

// System to handle continuous segment placement during mouse drag
pub fn drag_place_system(
    mut mouse_state: ResMut<MouseState>,
    mut place_segment_events: EventWriter<PlaceSegmentEvent>,
    selected_tool: Res<SelectedTool>,
    grid_config: Res<GridConfig>,
    grid_state: Res<GridState>,
    mouse_buttons: Res<ButtonInput<MouseButton>>, // To check if the button is *still* pressed
) {
    // Only run if dragging is active and the left mouse button is currently pressed.
    if mouse_state.is_dragging && mouse_buttons.pressed(MouseButton::Left) {
        let current_grid_pos = mouse_state.grid_position;

        // Check if the current grid position is valid, not already occupied,
        // and different from the last segment placed during this drag.
        if grid_config.is_valid_position(current_grid_pos)
            && !grid_state.is_occupied(current_grid_pos)
            && mouse_state.last_drag_grid_position != Some(current_grid_pos)
        {
            place_segment_events.write(PlaceSegmentEvent {
                position: current_grid_pos,
                segment_type: selected_tool.segment_type,
                direction: selected_tool.direction,
            });
            // Update the last placed position for this drag session.
            mouse_state.last_drag_grid_position = Some(current_grid_pos);
        }
    } else if !mouse_buttons.pressed(MouseButton::Left) && mouse_state.is_dragging {
        // Safety net: If the mouse button is released but is_dragging is somehow still true
        // (e.g., release event was missed or not yet processed by mouse_button_system).
        // This ensures the dragging state is reset.
        mouse_state.is_dragging = false;
        mouse_state.drag_start_pos = None;
        mouse_state.last_drag_grid_position = None;
    }
}

// 处理路段放置的系统
pub fn place_segment_system(
    mut commands: Commands,
    mut place_segment_events: EventReader<PlaceSegmentEvent>,
    mut grid_state: ResMut<GridState>,
    asset_server: Res<AssetServer>,
    grid_config: Res<GridConfig>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    passenger_query: Query<Entity, With<Passenger>>, // Query for passenger entities
) {
    for event in place_segment_events.read() {
        let entity = spawn_route_segment(
            &mut commands,
            event.position,
            event.segment_type,
            event.direction,
            &asset_server,
            &grid_config, // Pass GridConfig
            &mut texture_atlas_layouts,
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

        let passengers = passenger_query.iter().collect::<Vec<_>>();
        commands.trigger_targets(RequestPathReplanEvent, passengers);

        // Add selectable component
        commands.entity(entity).insert(Selectable);
    }
}

// 处理实体选择的系统
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

// 处理路段旋转的系统
pub fn rotate_segment_system(
    mut commands: Commands,
    mut rotate_segment_events: EventReader<RotateSegmentEvent>,
    mut query: Query<(&mut Transform, &mut RouteSegmentComponent)>,
    mut grid_state: ResMut<GridState>,
    grid_positions: Query<&GridPosition, With<RouteSegmentComponent>>, /* Query GridPosition of RouteSegments */
    passenger_query: Query<Entity, With<Passenger>>, // Query for passenger entities
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
                let passengers = passenger_query.iter().collect::<Vec<_>>();
                commands.trigger_targets(RequestPathReplanEvent, passengers);
            }
        }
    }
}

// 在鼠标位置显示预览/幽灵路段的系统
pub fn preview_system(
    mut commands: Commands,
    mouse_state: Res<MouseState>,
    selected_tool: Res<SelectedTool>,
    grid_config: Res<GridConfig>,
    grid_state: Res<GridState>,
    asset_server: Res<AssetServer>,
    preview_query: Query<Entity, With<Preview>>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    // Remove existing preview
    for entity in preview_query.iter() {
        commands.entity(entity).despawn();
    }

    let grid_pos = mouse_state.grid_position;

    // Only show preview if position is valid and empty
    if grid_config.is_valid_position(grid_pos) && !grid_state.is_occupied(grid_pos) {
        let texture = asset_server.load("textures/roads2W.png");
        let layout = TextureAtlasLayout::from_grid(UVec2::splat(64), 8, 3, None, None);
        let texture_atlas_layout = texture_atlas_layouts.add(layout);
        let texture_index = selected_tool.segment_type as usize;

        let final_rotation_angle = crate::game::grid::segment_type_rotation(
            selected_tool.segment_type,
            selected_tool.direction,
        );

        let world_pos = grid_config.grid_to_world(grid_pos);

        commands.spawn((
            Sprite {
                image: texture,
                texture_atlas: Some(TextureAtlas {
                    layout: texture_atlas_layout,
                    index: texture_index,
                }),
                color: Color::srgba(1.0, 1.0, 1.0, 0.5),
                ..default()
            },
            Transform {
                translation: Vec3::new(world_pos.x, world_pos.y, -1.0), // Behind other sprites
                rotation: Quat::from_rotation_z(final_rotation_angle),
                ..default()
            },
            Preview,
        ));
    }
}

// 处理工具选择的键盘输入的系统
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

// 右键点击移除路段的系统
pub fn remove_segment_system(
    mut commands: Commands,
    mut mouse_button_events: EventReader<MouseButtonInput>,
    mouse_state: ResMut<MouseState>,
    mut grid_state: ResMut<GridState>,
    grid_config: Res<GridConfig>,
    passenger_query: Query<Entity, With<Passenger>>, // Query for passenger entities
) {
    for event in mouse_button_events.read() {
        if event.button == MouseButton::Right && event.state.is_pressed() {
            let grid_pos = mouse_state.grid_position;

            if grid_config.is_valid_position(grid_pos) {
                if let Some(entity) = grid_state.remove_entity(grid_pos) {
                    commands.entity(entity).despawn();
                    grid_state.route_segments.remove(&grid_pos);

                    // Trigger replan for all passengers
                    info!(
                        "Segment removed from {:?}. Triggering replan for all passengers.",
                        grid_pos
                    );
                    let passengers = passenger_query.iter().collect::<Vec<_>>();

                    commands.trigger_targets(RequestPathReplanEvent, passengers);
                }
            }
        }
    }
}

// 点击空白区域清除选择的系统
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
