use crate::bus_puzzle::{LevelData, PathNode, Station};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

// 基础数据结构
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GridPos {
    pub x: i32,
    pub y: i32,
}

impl GridPos {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// 将网格坐标转换为世界坐标，使地图居中显示在原点附近
    pub fn to_world_pos(&self, tile_size: f32, grid_width: u32, grid_height: u32) -> Vec3 {
        let center_offset_x = (grid_width as f32 - 1.0) * tile_size * 0.5;
        let center_offset_y = (grid_height as f32 - 1.0) * tile_size * 0.5;

        Vec3::new(
            self.x as f32 * tile_size - center_offset_x,
            self.y as f32 * tile_size - center_offset_y,
            0.0,
        )
    }

    /// 便捷方法：从LevelData获取网格尺寸
    pub fn to_world_pos_with_level(&self, tile_size: f32, level_data: &LevelData) -> Vec3 {
        self.to_world_pos(tile_size, level_data.grid_size.0, level_data.grid_size.1)
    }
}

// 地形类型
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TerrainType {
    Empty,
    Building,
    Water,
    Park,
    Mountain,
}

// 路线段类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RouteSegmentType {
    Straight,
    Curve,
    TSplit,
    Cross,
    Bridge,
    Tunnel,
}

impl RouteSegmentType {
    pub fn get_cost(&self) -> u32 {
        match self {
            RouteSegmentType::Straight => 1,
            RouteSegmentType::Curve => 2,
            RouteSegmentType::TSplit => 3,
            RouteSegmentType::Cross => 4,
            RouteSegmentType::Bridge => 5,
            RouteSegmentType::Tunnel => 6,
        }
    }

    pub fn get_texture_path(&self) -> &'static str {
        match self {
            RouteSegmentType::Straight => "textures/routes/straight.png",
            RouteSegmentType::Curve => "textures/routes/curve.png",
            RouteSegmentType::TSplit => "textures/routes/t_split.png",
            RouteSegmentType::Cross => "textures/routes/cross.png",
            RouteSegmentType::Bridge => "textures/routes/bridge.png",
            RouteSegmentType::Tunnel => "textures/routes/tunnel.png",
        }
    }
}

// 站点类型
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StationType {
    BusStop,
    TransferHub,
    Terminal,
}

// 乘客颜色
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PassengerColor {
    Red,
    Blue,
    Green,
    Yellow,
    Purple,
    Orange,
}

// Bevy 组件
#[derive(Component)]
pub struct GridTile {
    pub grid_pos: GridPos,
    pub terrain_type: TerrainType,
}

#[derive(Component)]
pub struct RouteSegment {
    pub grid_pos: GridPos,
    pub segment_type: RouteSegmentType,
    pub rotation: u32,
    pub is_active: bool,
}

#[derive(Component)]
pub struct StationEntity {
    pub station_data: Station,
    pub current_passengers: u32,
}

#[derive(Component)]
pub struct PassengerEntity {
    pub color: PassengerColor,
    pub origin: String,
    pub destination: String,
    pub current_patience: f32,
    pub path: Vec<GridPos>,
}

// 寻路组件
#[derive(Component)]
pub struct PathfindingAgent {
    pub color: PassengerColor,
    pub origin: String,
    pub destination: String,
    pub current_path: Vec<PathNode>,
    pub current_step: usize,
    pub state: AgentState,
    pub patience: f32,
    pub max_patience: f32,
    pub waiting_time: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AgentState {
    WaitingAtStation,
    Traveling,
    Transferring,
    Arrived,
    GaveUp,
}

// UI 组件
#[derive(Component)]
pub struct MainMenuUI;

#[derive(Component)]
pub struct GameplayUI;

#[derive(Component)]
pub struct PauseMenuUI;

#[derive(Component)]
pub struct LevelCompleteUI;

#[derive(Component)]
pub struct InventoryUI {
    pub segment_type: RouteSegmentType,
    pub slot_index: usize,
}

#[derive(Component)]
pub struct ObjectiveUI {
    pub objective_index: usize,
}

#[derive(Component)]
pub struct ScoreText;

#[derive(Component)]
pub struct TimerText;

#[derive(Component)]
pub struct CostText;

#[derive(Component)]
pub struct PassengerCountText;

// 交互组件
#[derive(Component)]
pub struct DraggableSegment {
    pub segment_type: RouteSegmentType,
    pub rotation: u32,
    pub is_being_dragged: bool,
    pub is_placed: bool,
    pub cost: u32,
}

#[derive(Component)]
pub struct SegmentPreview {
    pub segment_type: RouteSegmentType,
    pub rotation: u32,
    pub target_position: GridPos,
}

#[derive(Component)]
pub struct GridHighlight {
    pub is_valid_placement: bool,
}

#[derive(Component)]
pub struct UIElement;

#[derive(Component)]
pub struct InventorySlot {
    pub slot_index: usize,
    pub segment_type: Option<RouteSegmentType>,
    pub available_count: u32,
}

#[derive(Component)]
pub struct ObjectiveTracker {
    pub objective_index: usize,
    pub is_completed: bool,
}
