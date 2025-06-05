// 注册所有网格相关系统的插件
use crate::{
    game::{interaction::Selectable, validation::ConnectionMap},
    screens::Screen,
};
use bevy::{
    prelude::*,
    window::{PrimaryWindow, WindowResized},
};
use serde::{Deserialize, Serialize};

pub struct GridPlugin;

impl Plugin for GridPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GridConfig>()
            .init_resource::<GridState>()
            .init_resource::<ConnectionMap>() // Initialize ConnectionMap
            .add_systems(OnEnter(Screen::Gameplay), setup_grid_from_window_size) // 注册新的事件
            .add_systems(
                Update,
                (
                    grid_snap_system,
                    update_grid_state_system,
                    update_route_segments_system,
                    setup_grid_from_window_size
                        .run_if(|ev: EventReader<WindowResized>| !ev.is_empty()),
                )
                    .run_if(in_state(Screen::Gameplay)),
            );
        // .add_observer(spawn_route_segment_system);
    }
}

// 事件：用于请求生成一个新的路线片段
#[derive(Event, Debug, Clone, Copy)]
pub struct SpawnRouteSegmentEvent {
    pub grid_pos: GridPos,
    pub segment_type: RouteSegmentType,
    pub direction: Direction,
}

// System to update the GridState.route_segments HashMap
pub fn update_route_segments_system(
    mut grid_state: ResMut<GridState>,
    query: Query<(&GridPos, &RouteSegmentComponent)>, /* Query for all entities with these components */
) {
    // Clear the existing route segments. This is a simple approach.
    // For more complex scenarios (e.g., dynamically adding/removing segments frequently),
    // you might want a more sophisticated update logic that handles additions,
    // removals, and changes without a full clear and rebuild.
    grid_state.route_segments.clear();

    for (grid_pos, route_segment_comp) in query.iter() {
        grid_state
            .route_segments
            .insert(*grid_pos, *route_segment_comp); // RouteSegmentComponent is Copy
    }
}

// 网格位置组件 - 表示逻辑网格坐标
#[derive(Component, Default, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GridPos {
    pub x: i32,
    pub y: i32,
}

impl GridPos {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    // 获取相邻位置（上、下、左、右）
    pub fn adjacent(&self) -> [GridPos; 4] {
        [
            GridPos::new(self.x, self.y + 1), // 上
            GridPos::new(self.x, self.y - 1), // 下
            GridPos::new(self.x - 1, self.y), // 左
            GridPos::new(self.x + 1, self.y), // 右
        ]
    }

    // 计算到另一个网格位置的曼哈顿距离
    pub fn distance_to(&self, other: &GridPos) -> i32 {
        (self.x - other.x).abs() + (self.y - other.y).abs()
    }

    pub fn to_world_pos(&self, grid_size: f32) -> Vec3 {
        Vec3::new(self.x as f32 * grid_size, self.y as f32 * grid_size, 0.0)
    }
}

// 管理网格配置的资源
#[derive(Resource, Debug)] // 添加Debug用于日志记录
pub struct GridConfig {
    pub tile_size: f32,      // 每个网格瓦片在世界单位中的大小
    pub grid_width: i32,     // 水平方向的瓦片数量
    pub grid_height: i32,    // 垂直方向的瓦片数量
    pub origin_offset: Vec2, // 从世界原点到网格中心的偏移量
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
    // 将网格位置转换为世界坐标
    pub fn grid_to_world(&self, grid_pos: GridPos) -> Vec2 {
        Vec2::new(
            grid_pos.x as f32 * self.tile_size + self.origin_offset.x,
            grid_pos.y as f32 * self.tile_size + self.origin_offset.y,
        )
    }

    // 将世界坐标转换为网格位置
    pub fn world_to_grid(&self, world_pos: Vec2) -> GridPos {
        let adjusted_pos = world_pos - self.origin_offset;
        GridPos::new(
            (adjusted_pos.x / self.tile_size).round() as i32,
            (adjusted_pos.y / self.tile_size).round() as i32,
        )
    }

    // 检查网格位置是否在边界内
    pub fn is_valid_position(&self, grid_pos: GridPos) -> bool {
        grid_pos.x >= 0
            && grid_pos.x < self.grid_width
            && grid_pos.y >= 0
            && grid_pos.y < self.grid_height
    }
}

// 标记应该对齐到网格的实体的组件
#[derive(Component)]
pub struct GridSnap;

// 路线段类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RouteSegmentType {
    Straight, // 直线段
    Curve,    // 弯道
    TSplit,   // T型分岔
    Cross,    // 十字路口
    Bridge,   // 桥梁（可跨越水域）
    Tunnel,   // 隧道（可穿越山丘）
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum TerrainType {
    Empty,    // 可放置路线的空地
    Building, // 建筑物障碍
    Water,    // 河流/湖泊
    Park,     // 公园绿地
    Mountain, // 山丘地形
}

// 路线段的方向枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Direction {
    North = 0,
    East = 1,
    South = 2,
    West = 3,
}

impl Direction {
    // 顺时针旋转方向
    pub fn rotate_cw(&self) -> Direction {
        match self {
            Direction::North => Direction::East,
            Direction::East => Direction::South,
            Direction::South => Direction::West,
            Direction::West => Direction::North,
        }
    }

    // 获取相反方向
    pub fn opposite(&self) -> Direction {
        match self {
            Direction::North => Direction::South,
            Direction::East => Direction::West,
            Direction::South => Direction::North,
            Direction::West => Direction::East,
        }
    }

    // 转换为网格偏移量
    pub fn to_offset(&self) -> (i32, i32) {
        match self {
            Direction::North => (0, 1),
            Direction::East => (1, 0),
            Direction::South => (0, -1),
            Direction::West => (-1, 0),
        }
    }
}

// 带方向的路线段组件
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct RouteSegmentComponent {
    pub segment_type: RouteSegmentType,
    pub direction: Direction,
}

// 标记地形/背景元素（如草地）的组件
#[derive(Component)]
pub struct TerrainElement;

// 标记实际路线元素（道路、车站）的组件
#[derive(Component)]
pub struct RouteElement;

// 网格状态资源，用于跟踪各处放置的内容
#[derive(Resource, Default)]
pub struct GridState {
    pub occupied: std::collections::HashMap<GridPos, Entity>,
    pub route_segments: std::collections::HashMap<GridPos, RouteSegmentComponent>,
}

impl GridState {
    // Check if a grid position is occupied
    pub fn is_occupied(&self, pos: GridPos) -> bool {
        self.occupied.contains_key(&pos)
    }

    // Place an entity at a grid position
    pub fn place_entity(&mut self, pos: GridPos, entity: Entity) {
        self.occupied.insert(pos, entity);
    }

    // Remove entity from grid position
    pub fn remove_entity(&mut self, pos: GridPos) -> Option<Entity> {
        self.occupied.remove(&pos)
    }

    // Get entity at grid position
    pub fn get_entity(&self, pos: GridPos) -> Option<Entity> {
        self.occupied.get(&pos).copied()
    }

    // Place a route segment
    pub fn place_route_segment(&mut self, pos: GridPos, segment: RouteSegmentComponent) {
        self.route_segments.insert(pos, segment);
    }

    // Get route segment at position
    pub fn get_route_segment(&self, pos: GridPos) -> Option<&RouteSegmentComponent> {
        self.route_segments.get(&pos)
    }
}

// 在启动时根据窗口大小设置GridConfig的系统
fn setup_grid_from_window_size(
    mut grid_config: ResMut<GridConfig>,
    window: Single<&Window, With<PrimaryWindow>>,
) {
    let window_width = window.width();
    let window_height = window.height();

    // 假设世界坐标中的(0,0)是窗口的中心。
    // 窗口左下角的世界坐标。
    grid_config.origin_offset = Vec2::new(-window_width / 2.0, -window_height / 2.0);

    grid_config.grid_width = (window_width / grid_config.tile_size).ceil() as i32;
    grid_config.grid_height = (window_height / grid_config.tile_size).ceil() as i32;

    info!("GridConfig适应窗口大小：{:?}", *grid_config);
}

// 将带有GridSnap组件的实体对齐到网格位置的系统
pub fn grid_snap_system(
    mut query: Query<(&mut Transform, &GridPos), (With<GridSnap>, Changed<GridPos>)>,
    grid_config: Res<GridConfig>,
) {
    for (mut transform, grid_pos) in query.iter_mut() {
        let world_pos = grid_config.grid_to_world(*grid_pos);
        transform.translation.x = world_pos.x;
        transform.translation.y = world_pos.y;
    }
}

// 当带有GridPosition的实体移动时更新网格状态的系统
pub fn update_grid_state_system(
    mut grid_state: ResMut<GridState>,
    query: Query<(Entity, &GridPos), Changed<GridPos>>,
) {
    for (entity, grid_pos) in query.iter() {
        // 从旧位置移除
        grid_state.occupied.retain(|_, &mut v| v != entity);
        // 添加到新位置
        grid_state.place_entity(*grid_pos, entity);
    }
}

// System to spawn route segments based on SpawnRouteSegmentEvent
// pub fn spawn_route_segment_system(
//     event: Trigger<SpawnRouteSegmentEvent>,
//     mut commands: Commands,
//     asset_server: Res<AssetServer>,
//     grid_config: Res<GridConfig>,
//     mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
//     mut grid_state: ResMut<GridState>, // Added to update grid state
// ) {
//     let grid_pos = event.grid_pos;
//     let segment_type = event.segment_type;
//     let direction = event.direction;
//
//     info!(
//         "Spawning route segment via event: pos={:?}, type={:?}, dir={:?}",
//         grid_pos, segment_type, direction
//     );
//
//     let sprite = match segment_type {
//         RouteSegmentType::DeadEnd => {
//             let texture = asset_server.load("textures/CP_V1.0.4.png");
//             let layout = TextureAtlasLayout::from_grid(
//                 UVec2::splat(100),
//                 4,
//                 2,
//                 None,
//                 Some(UVec2::new(0, 150)),
//             );
//             let texture_atlas_layout = texture_atlas_layouts.add(layout);
//
//             let mut sprite = Sprite::from_atlas_image(
//                 texture,
//                 TextureAtlas {
//                     layout: texture_atlas_layout,
//                     index: 0,
//                 },
//             );
//             sprite.custom_size = Some(Vec2::splat(grid_config.tile_size));
//             sprite
//         }
//         _ => {
//             let texture = asset_server.load("textures/roads2W.png");
//             let layout = TextureAtlasLayout::from_grid(UVec2::splat(64), 8, 3, None, None);
//             let texture_atlas_layout = texture_atlas_layouts.add(layout);
//             let texture_index = segment_type.to_index().unwrap();
//
//             Sprite::from_atlas_image(
//                 texture,
//                 TextureAtlas {
//                     layout: texture_atlas_layout,
//                     index: texture_index,
//                 },
//             )
//         }
//     };
//
//     let final_rotation_angle = segment_type_rotation(segment_type, direction);
//
//     let entity_id = commands
//         .spawn((
//             sprite,
//             Transform {
//                 translation: grid_config.grid_to_world(grid_pos).extend(0.0),
//                 rotation: Quat::from_rotation_z(final_rotation_angle),
//                 ..default()
//             },
//             grid_pos,
//             GridSnap,
//             RouteSegmentComponent {
//                 segment_type,
//                 direction,
//             },
//         ))
//         .insert(Selectable)
//         .with_children(|parent| {
//             // if let RouteSegmentType::DeadEnd = segment_type {
//             //     parent.spawn((
//             //         Sprite::from_color(destination.get_color(), Vec2::splat(grid_config.tile_size)),
//             //         Transform::from_xyz(0.0, 0.0, -1.0),
//             //     ));
//             // } fixme
//         })
//         .id();
//
//     // Update GridState
//     grid_state.place_entity(grid_pos, entity_id);
//     grid_state.place_route_segment(
//         grid_pos,
//         RouteSegmentComponent {
//             segment_type,
//             direction,
//         },
//     );
//     info!(
//         "Entity {:?} spawned and GridState updated at {:?}",
//         entity_id, grid_pos
//     );
// }

// pub fn segment_type_rotation(segment_type: RouteSegmentType, direction: Direction) -> f32 {
//     // Calculate rotation
//     let current_direction_angle = direction as u8 as f32 * std::f32::consts::PI / 2.0;
//     let final_rotation_angle = if segment_type == RouteSegmentType::Turn {
//         let corner_rotation_factor = match direction {
//             Direction::North => 0.0,
//             Direction::East => 3.0,
//             Direction::South => 2.0,
//             Direction::West => 1.0,
//         };
//         corner_rotation_factor * std::f32::consts::PI / 2.0
//     } else if segment_type == RouteSegmentType::TSplit {
//         let corner_rotation_factor = match direction {
//             Direction::North => 1.0,
//             Direction::East => 0.0,
//             Direction::South => 3.0,
//             Direction::West => 2.0,
//         };
//         corner_rotation_factor * std::f32::consts::PI / 2.0
//     } else {
//         current_direction_angle
//     };
//     final_rotation_angle
// }
