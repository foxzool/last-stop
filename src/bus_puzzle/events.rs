use bevy::prelude::Event;
use crate::bus_puzzle::{GridPos, RouteSegmentType};

#[derive(Event)]
pub struct SegmentPlacedEvent {
    pub position: GridPos,
    pub segment_type: RouteSegmentType,
    pub rotation: u32,
}

#[derive(Event)]
pub struct SegmentRemovedEvent {
    pub position: GridPos,
}

#[derive(Event)]
pub struct ObjectiveCompletedEvent {
    pub objective_index: usize,
}

#[derive(Event)]
pub struct LevelCompletedEvent {
    pub final_score: u32,
    pub completion_time: f32,
}

#[derive(Event)]
pub struct InventoryUpdatedEvent {
    pub segment_type: RouteSegmentType,
    pub new_count: u32,
}
