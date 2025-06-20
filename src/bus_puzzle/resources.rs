use crate::bus_puzzle::{GridPos, LevelData, RouteSegmentType, DEFAULT_TILE_SIZE};
use bevy::{platform::collections::HashMap, prelude::*};

// 游戏状态
#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
#[states(scoped_entities)]
pub enum GameStateEnum {
    #[default]
    Splash,
    MainMenu,
    Loading,
    Playing,
    Paused,
    LevelComplete,
    GameOver,
}

#[derive(Resource, Default)]
pub struct GameState {
    pub current_level: Option<LevelData>,
    pub player_inventory: HashMap<RouteSegmentType, u32>,
    pub placed_segments: HashMap<GridPos, PlacedSegment>,
    pub total_cost: u32,
    pub game_time: f32,
    pub level_start_time: f32, // 新增：关卡开始时的系统时间
    pub is_paused: bool,
    pub objectives_completed: Vec<bool>,
    pub score: GameScore,
    pub passenger_stats: PassengerStats,
}

#[derive(Default)]
pub struct PassengerStats {
    pub total_spawned: u32,
    pub total_arrived: u32,
    pub total_gave_up: u32,
}

#[derive(Debug, Clone)]
pub struct PlacedSegment {
    pub segment_type: RouteSegmentType,
    pub rotation: u32,
    pub entity: Entity,
    pub cost: u32,
}

#[derive(Resource, Default)]
pub struct GameScore {
    pub base_points: u32,
    pub efficiency_bonus: u32,
    pub speed_bonus: u32,
    pub cost_bonus: u32,
    pub total_score: u32,
}

#[derive(Resource, Default)]
#[allow(dead_code)]
pub struct InputState {
    pub mouse_world_pos: Vec3,
    pub selected_segment: Option<RouteSegmentType>,
    pub preview_rotation: u32, // 新增：预览旋转角度
    pub is_dragging: bool,
    pub drag_entity: Option<Entity>,
    pub grid_cursor_pos: Option<GridPos>,
}

#[derive(Resource)]
pub struct CameraController {
    pub zoom: f32,
    pub min_zoom: f32,
    pub max_zoom: f32,
    pub pan_speed: f32,
    pub zoom_speed: f32,
}

impl Default for CameraController {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            min_zoom: 0.3,
            max_zoom: 3.0,
            pan_speed: 500.0,
            zoom_speed: 0.1,
        }
    }
}

#[derive(Resource)]
#[allow(dead_code)]
pub struct LevelManager {
    pub current_level: Option<LevelData>,
    pub tile_size: f32,
    pub available_levels: Vec<String>,
    pub current_level_index: usize,
    pub unlocked_levels: Vec<bool>,
    pub level_scores: HashMap<String, u32>,
}

impl Default for LevelManager {
    fn default() -> Self {
        Self {
            current_level: None,
            tile_size: DEFAULT_TILE_SIZE,
            available_levels: vec![
                "tutorial_01".to_string(),
                "level_02_transfer".to_string(),
                "level_03_multiple_routes".to_string(),
                "level_04_time_pressure".to_string(),
            ],
            current_level_index: 0,
            unlocked_levels: vec![true, false, false, false], // 只有第一关解锁
            level_scores: HashMap::new(),
        }
    }
}

// 寻路资源
#[derive(Resource, Default)]
pub struct PathfindingGraph {
    pub nodes: HashMap<GridPos, GraphNode>,
    pub connections: HashMap<GridPos, Vec<Connection>>,
    pub station_lookup: HashMap<String, GridPos>,
    pub route_network: HashMap<String, RouteInfo>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct GraphNode {
    pub position: GridPos,
    pub node_type: GraphNodeType,
    pub station_name: Option<String>,
    pub is_accessible: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GraphNodeType {
    Station,
    RouteSegment,
    Intersection,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Connection {
    pub to: GridPos,
    pub cost: f32,
    pub route_id: Option<String>,
    pub connection_type: ConnectionType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionType {
    Walk,
    BusRoute,
    Transfer,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct RouteInfo {
    pub id: String,
    pub segments: Vec<GridPos>,
    pub frequency: f32,
    pub capacity: u32,
    pub is_active: bool,
}

#[derive(Resource)]
pub struct AudioAssets {
    pub background_music: Handle<AudioSource>,
    pub segment_place_sound: Handle<AudioSource>,
    pub segment_remove_sound: Handle<AudioSource>,
    pub passenger_arrive_sound: Handle<AudioSource>,
    pub objective_complete_sound: Handle<AudioSource>,
    pub level_complete_sound: Handle<AudioSource>,
    pub button_click_sound: Handle<AudioSource>,
    pub error_sound: Handle<AudioSource>,
}
