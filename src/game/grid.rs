// Plugin to register all grid-related systems
use crate::screens::Screen;
use bevy::{
    prelude::*,
    window::{PrimaryWindow, WindowResized},
};

pub struct GridPlugin;

impl Plugin for GridPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GridConfig>()
            .init_resource::<GridState>()
            .add_systems(OnEnter(Screen::Gameplay), setup_grid_from_window_size)
            .add_systems(
                Update,
                (
                    grid_snap_system,
                    update_grid_state_system,
                    setup_grid_from_window_size
                        .run_if(|ev: EventReader<WindowResized>| !ev.is_empty()),
                ),
            );
    }
}

// Grid position component - represents logical grid coordinates
#[derive(Component, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GridPosition {
    pub x: i32,
    pub y: i32,
}

impl GridPosition {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    // Get adjacent positions (up, down, left, right)
    pub fn adjacent(&self) -> [GridPosition; 4] {
        [
            GridPosition::new(self.x, self.y + 1), // Up
            GridPosition::new(self.x, self.y - 1), // Down
            GridPosition::new(self.x - 1, self.y), // Left
            GridPosition::new(self.x + 1, self.y), // Right
        ]
    }

    // Calculate Manhattan distance to another grid position
    pub fn distance_to(&self, other: &GridPosition) -> i32 {
        (self.x - other.x).abs() + (self.y - other.y).abs()
    }
}

// Resource to manage grid configuration
#[derive(Resource, Debug)] // Added Debug for logging
pub struct GridConfig {
    pub tile_size: f32,      // Size of each grid tile in world units
    pub grid_width: i32,     // Number of tiles horizontally
    pub grid_height: i32,    // Number of tiles vertically
    pub origin_offset: Vec2, // Offset from world origin to grid center
}

impl Default for GridConfig {
    fn default() -> Self {
        Self {
            tile_size: 64.0,
            grid_width: 12,
            grid_height: 8,
            origin_offset: Vec2::ZERO,
        }
    }
}

impl GridConfig {
    // Convert grid position to world coordinates
    pub fn grid_to_world(&self, grid_pos: GridPosition) -> Vec2 {
        Vec2::new(
            grid_pos.x as f32 * self.tile_size + self.origin_offset.x,
            grid_pos.y as f32 * self.tile_size + self.origin_offset.y,
        )
    }

    // Convert world coordinates to grid position
    pub fn world_to_grid(&self, world_pos: Vec2) -> GridPosition {
        let adjusted_pos = world_pos - self.origin_offset;
        GridPosition::new(
            (adjusted_pos.x / self.tile_size).round() as i32,
            (adjusted_pos.y / self.tile_size).round() as i32,
        )
    }

    // Check if grid position is within bounds
    pub fn is_valid_position(&self, grid_pos: GridPosition) -> bool {
        grid_pos.x >= 0
            && grid_pos.x < self.grid_width
            && grid_pos.y >= 0
            && grid_pos.y < self.grid_height
    }
}

// Component to mark entities that should snap to grid
#[derive(Component)]
pub struct GridSnap;

// Route segment types
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub enum RouteSegment {
    Straight,  // ─ or │
    Corner,    // └ ┘ ┐ ┌
    TJunction, // ┬ ┴ ├ ┤
    Cross,     // ┼
    Station,   // Bus station/stop
    Grass,     // Grass terrain/background
}

// Direction enum for route segments
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    North = 0,
    East = 1,
    South = 2,
    West = 3,
}

impl Direction {
    // Rotate direction clockwise
    pub fn rotate_cw(&self) -> Direction {
        match self {
            Direction::North => Direction::East,
            Direction::East => Direction::South,
            Direction::South => Direction::West,
            Direction::West => Direction::North,
        }
    }

    // Get opposite direction
    pub fn opposite(&self) -> Direction {
        match self {
            Direction::North => Direction::South,
            Direction::East => Direction::West,
            Direction::South => Direction::North,
            Direction::West => Direction::East,
        }
    }

    // Convert to grid offset
    pub fn to_offset(&self) -> (i32, i32) {
        match self {
            Direction::North => (0, 1),
            Direction::East => (1, 0),
            Direction::South => (0, -1),
            Direction::West => (-1, 0),
        }
    }
}

// Component for route segment with direction
#[derive(Component, Clone)]
pub struct RouteSegmentComponent {
    pub segment_type: RouteSegment,
    pub direction: Direction,
}

// Component to mark terrain/background elements (like grass)
#[derive(Component)]
pub struct TerrainElement;

// Component to mark actual route elements (roads, stations)
#[derive(Component)]
pub struct RouteElement;

// Grid state resource to track what's placed where
#[derive(Resource, Default)]
pub struct GridState {
    pub occupied: std::collections::HashMap<GridPosition, Entity>,
    pub route_segments: std::collections::HashMap<GridPosition, RouteSegmentComponent>,
}

impl GridState {
    // Check if a grid position is occupied
    pub fn is_occupied(&self, pos: GridPosition) -> bool {
        self.occupied.contains_key(&pos)
    }

    // Place an entity at a grid position
    pub fn place_entity(&mut self, pos: GridPosition, entity: Entity) {
        self.occupied.insert(pos, entity);
    }

    // Remove entity from grid position
    pub fn remove_entity(&mut self, pos: GridPosition) -> Option<Entity> {
        self.occupied.remove(&pos)
    }

    // Get entity at grid position
    pub fn get_entity(&self, pos: GridPosition) -> Option<Entity> {
        self.occupied.get(&pos).copied()
    }

    // Place a route segment
    pub fn place_route_segment(&mut self, pos: GridPosition, segment: RouteSegmentComponent) {
        self.route_segments.insert(pos, segment);
    }

    // Get route segment at position
    pub fn get_route_segment(&self, pos: GridPosition) -> Option<&RouteSegmentComponent> {
        self.route_segments.get(&pos)
    }
}

// System to setup GridConfig based on window size at startup
fn setup_grid_from_window_size(
    mut grid_config: ResMut<GridConfig>,
    window: Single<&Window, With<PrimaryWindow>>,
) {
    let window_width = window.width();
    let window_height = window.height();

    // Assuming (0,0) in world coordinates is the center of the window.
    // The world coordinates of the window's bottom-left corner.
    grid_config.origin_offset = Vec2::new(-window_width / 2.0, -window_height / 2.0);

    grid_config.grid_width = (window_width / grid_config.tile_size).ceil() as i32;
    grid_config.grid_height = (window_height / grid_config.tile_size).ceil() as i32;

    info!("GridConfig adapted to window size: {:?}", *grid_config);
}

// System to snap entities with GridSnap component to grid positions
pub fn grid_snap_system(
    mut query: Query<(&mut Transform, &GridPosition), (With<GridSnap>, Changed<GridPosition>)>,
    grid_config: Res<GridConfig>,
) {
    for (mut transform, grid_pos) in query.iter_mut() {
        let world_pos = grid_config.grid_to_world(*grid_pos);
        transform.translation.x = world_pos.x;
        transform.translation.y = world_pos.y;
    }
}

// System to update grid state when entities with GridPosition move
pub fn update_grid_state_system(
    mut grid_state: ResMut<GridState>,
    query: Query<(Entity, &GridPosition), Changed<GridPosition>>,
) {
    for (entity, grid_pos) in query.iter() {
        // Remove from old position if exists
        grid_state.occupied.retain(|_, &mut v| v != entity);
        // Add to new position
        grid_state.place_entity(*grid_pos, entity);
    }
}

// Helper function to spawn a route segment at grid position
pub fn spawn_route_segment(
    commands: &mut Commands,
    grid_pos: GridPosition,
    segment_type: RouteSegment,
    direction: Direction,
    asset_server: &Res<AssetServer>,
    grid_config: &Res<GridConfig>, // Added GridConfig resource
    texture_atlas_layouts: &mut ResMut<Assets<TextureAtlasLayout>>,
) -> Entity {
    let texture = asset_server.load("textures/roads2W.png");
    let layout = TextureAtlasLayout::from_grid(UVec2::splat(64), 8, 3, None, None);
    let texture_atlas_layout = texture_atlas_layouts.add(layout);
    let texture_index = match segment_type {
        RouteSegment::Straight => 3,
        RouteSegment::Corner => 11,
        RouteSegment::TJunction => 13,
        RouteSegment::Cross => 16,
        RouteSegment::Station => 17,
        RouteSegment::Grass => 5,
    };

    commands
        .spawn((
            Sprite::from_atlas_image(
                texture,
                TextureAtlas {
                    layout: texture_atlas_layout,
                    index: texture_index,
                },
            ),
            Transform {
                translation: grid_config.grid_to_world(grid_pos).extend(0.0), // Set initial world position
                rotation: Quat::from_rotation_z(
                    direction as u8 as f32 * std::f32::consts::PI / 2.0,
                ),
                ..default()
            },
            grid_pos, // Keep GridPosition for state tracking and other systems
            GridSnap,
            RouteSegmentComponent {
                segment_type,
                direction,
            },
        ))
        .id()
}
