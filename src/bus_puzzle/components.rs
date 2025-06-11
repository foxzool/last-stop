use crate::bus_puzzle::{PathNode, Station, SEGMENT_PLACEMENT_COST};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// 旋转偏移量
pub fn rotate_offset(dx: i32, dy: i32, rotation: u32) -> (i32, i32) {
    match rotation % 360 {
        0 => (dx, dy),
        90 => (-dy, dx),
        180 => (-dx, -dy),
        270 => (dy, -dx),
        _ => (dx, dy),
    }
}

/// 方向枚举（用于可视化）
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub enum ConnectionDirection {
    North, // 上 (y-1)
    South, // 下 (y+1)
    East,  // 右 (x+1)
    West,  // 左 (x-1)
}

impl ConnectionDirection {
    #[allow(dead_code)]
    pub fn from_offset(dx: i32, dy: i32) -> Option<Self> {
        match (dx, dy) {
            (0, -1) => Some(ConnectionDirection::North),
            (0, 1) => Some(ConnectionDirection::South),
            (1, 0) => Some(ConnectionDirection::East),
            (-1, 0) => Some(ConnectionDirection::West),
            _ => None,
        }
    }

    #[allow(dead_code)]
    pub fn to_offset(self) -> (i32, i32) {
        match self {
            ConnectionDirection::North => (0, -1),
            ConnectionDirection::South => (0, 1),
            ConnectionDirection::East => (1, 0),
            ConnectionDirection::West => (-1, 0),
        }
    }
}

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
    pub fn to_world_pos(self, tile_size: f32, grid_width: u32, grid_height: u32) -> Vec3 {
        let center_offset_x = (grid_width as f32 - 1.0) * tile_size * 0.5;
        let center_offset_y = (grid_height as f32 - 1.0) * tile_size * 0.5;

        // 确保网格坐标转换为世界坐标时指向瓦片中心
        Vec3::new(
            self.x as f32 * tile_size - center_offset_x,
            self.y as f32 * tile_size - center_offset_y,
            0.0,
        )
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
            RouteSegmentType::Straight => SEGMENT_PLACEMENT_COST[0],
            RouteSegmentType::Curve => SEGMENT_PLACEMENT_COST[1],
            RouteSegmentType::TSplit => SEGMENT_PLACEMENT_COST[2],
            RouteSegmentType::Cross => SEGMENT_PLACEMENT_COST[3],
            RouteSegmentType::Bridge => SEGMENT_PLACEMENT_COST[4],
            RouteSegmentType::Tunnel => SEGMENT_PLACEMENT_COST[5],
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

    /// 获取路线段的基础连接方向（0°旋转时的方向）
    /// 返回相对偏移量：(dx, dy)
    pub fn get_base_connection_offsets(&self) -> Vec<(i32, i32)> {
        match self {
            RouteSegmentType::Straight => vec![(-1, 0), (1, 0)], // 水平：左右
            RouteSegmentType::Curve => vec![(-1, 0), (0, -1)],   // L型：左和上
            RouteSegmentType::TSplit => vec![(0, -1), (0, 1), (1, 0)], // T型：上下右
            RouteSegmentType::Cross => vec![(0, -1), (0, 1), (-1, 0), (1, 0)], // 十字：四方向
            RouteSegmentType::Bridge | RouteSegmentType::Tunnel => vec![(-1, 0), (1, 0)], /* 水平：左右 */
        }
    }

    /// 获取旋转后的连接偏移量
    pub fn get_connection_offsets(&self, rotation: u32) -> Vec<(i32, i32)> {
        let base_offsets = self.get_base_connection_offsets();

        // 对于直线段、桥梁、隧道，特殊处理旋转
        if matches!(
            self,
            RouteSegmentType::Straight | RouteSegmentType::Bridge | RouteSegmentType::Tunnel
        ) {
            match rotation % 180 {
                0 => base_offsets,           // 水平：左右
                90 => vec![(0, -1), (0, 1)], // 垂直：上下
                _ => base_offsets,
            }
        } else {
            // 其他类型应用旋转变换
            base_offsets
                .into_iter()
                .map(|(dx, dy)| rotate_offset(dx, dy, rotation))
                .collect()
        }
    }

    /// 获取连接方向（用于可视化）
    #[allow(dead_code)]
    pub fn get_connection_directions(&self, rotation: u32) -> Vec<ConnectionDirection> {
        self.get_connection_offsets(rotation)
            .into_iter()
            .filter_map(|(dx, dy)| ConnectionDirection::from_offset(dx, dy))
            .collect()
    }

    /// 检查路线段在指定旋转下是否有连接到目标位置的端口
    pub fn has_connection_to(
        &self,
        segment_pos: GridPos,
        target_pos: GridPos,
        rotation: u32,
    ) -> bool {
        let connection_offsets = self.get_connection_offsets(rotation);
        let target_offset = (target_pos.x - segment_pos.x, target_pos.y - segment_pos.y);
        connection_offsets.contains(&target_offset)
    }

    /// 获取路线段在指定位置和旋转下的所有连接位置
    pub fn get_connection_positions(&self, segment_pos: GridPos, rotation: u32) -> Vec<GridPos> {
        self.get_connection_offsets(rotation)
            .into_iter()
            .map(|(dx, dy)| GridPos::new(segment_pos.x + dx, segment_pos.y + dy))
            .collect()
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
#[allow(dead_code)]
pub struct GridTile {
    pub grid_pos: GridPos,
    pub terrain_type: TerrainType,
}

#[derive(Component, Clone, Copy)]
pub struct RouteSegment {
    pub grid_pos: GridPos,
    pub segment_type: RouteSegmentType,
    pub rotation: u32,
    pub is_active: bool,
}

#[derive(Component)]
#[allow(dead_code)]
pub struct StationEntity {
    pub station_data: Station,
    pub current_passengers: u32,
}

#[derive(Component)]
#[allow(dead_code)]
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
#[allow(dead_code)]
pub struct InventoryUI {
    pub segment_type: RouteSegmentType,
    pub slot_index: usize,
}

#[derive(Component)]
#[allow(dead_code)]
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
#[allow(dead_code)]
pub struct DraggableSegment {
    pub segment_type: RouteSegmentType,
    pub rotation: u32,
    pub is_being_dragged: bool,
    pub is_placed: bool,
    pub cost: u32,
}

#[derive(Component)]
#[allow(dead_code)]
pub struct SegmentPreview {
    pub segment_type: RouteSegmentType,
    pub rotation: u32,
    pub target_position: GridPos,
}

#[derive(Component)]
#[allow(dead_code)]
pub struct GridHighlight {
    pub is_valid_placement: bool,
}

#[derive(Component)]
pub struct UIElement;

#[derive(Component)]
#[allow(dead_code)]
pub struct InventorySlot {
    pub slot_index: usize,
    pub segment_type: Option<RouteSegmentType>,
    pub available_count: u32,
}

#[derive(Component)]
pub struct InventoryCountText {
    pub segment_type: RouteSegmentType,
}

#[derive(Component)]
#[allow(dead_code)]
pub struct ObjectiveTracker {
    pub objective_index: usize,
    pub is_completed: bool,
}

// 显示旋转提示的UI组件
#[derive(Component)]
pub struct RotationHintUI;
