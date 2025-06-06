use bevy::prelude::*;

use super::{LevelData, Station};
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RouteSegmentType {
    Straight,
    Curve,
    TSplit,
    Cross,
    Bridge,
    Tunnel,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AvailableSegment {
    pub segment_type: RouteSegmentType,
    pub count: u32, // 可用数量
    pub cost: u32,  // 建设成本
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObjectiveCondition {
    pub description: String,
    pub condition_type: crate::bus_puzzle::ObjectiveType,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ObjectiveType {
    ConnectAllPassengers,       // 连接所有乘客到目的地
    MaxTransfers(u32),          // 最大换乘次数限制
    MaxSegments(u32),           // 最大路段数量限制
    MaxCost(u32),               // 最大建设成本限制
    MinEfficiency(f32),         // 最小效率要求
    TimeLimit(f32),             // 时间限制（秒）
    PassengerSatisfaction(f32), // 乘客满意度要求
}
