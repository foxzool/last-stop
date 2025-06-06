use bevy::prelude::*;

#[derive(Resource)]
pub struct GameConfig {
    pub tile_size: f32,
    pub camera_speed: f32,
    pub zoom_sensitivity: f32,
    pub passenger_speed: f32,
    pub ui_scale: f32,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            tile_size: 64.0,
            camera_speed: 500.0,
            zoom_sensitivity: 0.1,
            passenger_speed: 100.0,
            ui_scale: 1.0,
        }
    }
}

// 游戏常量
pub const GAME_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const DEFAULT_TILE_SIZE: f32 = 64.0;
pub const MAX_PASSENGERS_PER_STATION: u32 = 20;
pub const DEFAULT_PASSENGER_PATIENCE: f32 = 60.0;
pub const SEGMENT_PLACEMENT_COST: [u32; 6] = [1, 2, 3, 4, 5, 6]; // 对应不同路线段类型

// 路径寻找相关常量
pub const MAX_PATHFINDING_ITERATIONS: usize = 1000;
pub const TRANSFER_COST_MULTIPLIER: f32 = 5.0;
pub const WALKING_SPEED: f32 = 50.0;
pub const BUS_SPEED: f32 = 150.0;

// UI 相关常量
pub const UI_ANIMATION_DURATION: f32 = 0.5;
pub const BUTTON_HOVER_SCALE: f32 = 1.1;
pub const INVENTORY_SLOT_SIZE: f32 = 80.0;
