use std::collections::HashMap;
use bevy::math::Vec3;
use bevy::prelude::{Entity, Resource};
use crate::bus_puzzle::{GridPos, LevelData, RouteSegmentType};

#[derive(Resource)]
pub struct GameState {
    pub current_level: Option<LevelData>,
    pub player_inventory: HashMap<RouteSegmentType, u32>,
    pub placed_segments: HashMap<GridPos, crate::bus_puzzle::PlacedSegment>,
    pub total_cost: u32,
    pub game_time: f32,
    pub is_paused: bool,
    pub objectives_completed: Vec<bool>,
    pub score: GameScore,
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

#[derive(Resource)]
pub struct InputState {
    pub mouse_world_pos: Vec3,
    pub selected_segment: Option<RouteSegmentType>,
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
