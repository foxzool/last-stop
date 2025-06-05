// 生成主要关卡

use crate::game::{
    grid::{Direction, GridConfig, GridPos, RouteSegmentType, SpawnRouteSegmentEvent, TerrainType},
    passenger::{Destination, PassengerManager},
};
use bevy::{platform::collections::HashMap, prelude::*};
use serde::{Deserialize, Serialize};

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<WantLevel>();
}

#[derive(Resource, Default)]
pub struct WantLevel(pub u8);

// // 生成初始路线和车站
// #[allow(dead_code)]
// pub fn spawn_initial_routes(
//     mut commands: Commands,
//     grid_config: Res<GridConfig>,
//     mut passenger_manager: ResMut<PassengerManager>,
// ) {
//     // 生成红色线路车站
//     let red_station_pos = GridPos::new(1, 1);
//     commands.trigger(SpawnRouteSegmentEvent {
//         grid_pos: red_station_pos,
//         segment_type: RouteSegmentType::DeadEnd,
//         direction: Direction::North,
//     });
//     passenger_manager.add_station(red_station_pos, vec![Destination::Red]);
//
//     // 生成蓝色线路车站
//     let blue_station_pos = GridPos::new(grid_config.grid_width - 2, 1);
//     commands.trigger(SpawnRouteSegmentEvent {
//         grid_pos: blue_station_pos,
//         segment_type: RouteSegmentType::DeadEnd,
//         direction: Direction::North,
//     });
//     passenger_manager.add_station(blue_station_pos, vec![Destination::Blue]);
//
//     // 生成绿色线路车站
//     let green_station_pos = GridPos::new(1, grid_config.grid_height - 2);
//     commands.trigger(SpawnRouteSegmentEvent {
//         grid_pos: green_station_pos,
//         segment_type: RouteSegmentType::DeadEnd,
//         direction: Direction::North,
//     });
//     passenger_manager.add_station(green_station_pos, vec![Destination::Green]);
//
//     // 生成黄色线路车站
//     let yellow_station_pos =
//         // GridPosition::new(grid_config.grid_width - 2, grid_config.grid_height - 2);
//         GridPos::new(grid_config.grid_width / 2, grid_config.grid_height / 2 + 4);
//     commands.trigger(SpawnRouteSegmentEvent {
//         grid_pos: yellow_station_pos,
//         segment_type: RouteSegmentType::DeadEnd,
//         direction: Direction::North,
//     });
//     passenger_manager.add_station(yellow_station_pos, vec![Destination::Yellow]);
//
//     // 生成中央换乘站
//     let central_station_pos = GridPos::new(grid_config.grid_width / 2, grid_config.grid_height / 2);
//     commands.trigger(SpawnRouteSegmentEvent {
//         grid_pos: central_station_pos,
//         segment_type: RouteSegmentType::DeadEnd,
//         direction: Direction::North,
//     });
//     passenger_manager.add_station(
//         central_station_pos,
//         vec![
//             Destination::Red,
//             Destination::Blue,
//             Destination::Green,
//             Destination::Yellow,
//         ],
//     );
// }

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum StationType {
    Start,    // 起点站
    End,      // 终点站
    Transfer, // 换乘站
    Regular,  // 普通站点
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StationConfig {
    pub position: GridPos,
    pub station_type: StationType,
    pub passenger_color: Option<Color>, // 起点站的乘客颜色，终点站的目标颜色
    pub name: String,
    pub capacity: u32, // 站点容量
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerrainTile {
    pub position: GridPos,
    pub terrain_type: TerrainType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrebuiltRoute {
    pub segments: Vec<(GridPos, RouteSegmentType, Direction)>,
    pub is_locked: bool, // 是否为预设路线（玩家不能修改）
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PassengerSpawnRule {
    pub start_station: String,           // 起点站名称
    pub end_station: String,             // 终点站名称
    pub spawn_interval: f32,             // 生成间隔（秒）
    pub max_patience: f32,               // 最大耐心值
    pub priority: u8,                    // 优先级 (1-5)
    pub time_window: Option<(f32, f32)>, // 可选的时间窗口
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicEvent {
    pub trigger_time: f32, // 触发时间
    pub event_type: EventType,
    pub duration: Option<f32>, // 持续时间（None表示永久）
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventType {
    BlockTerrain { position: GridPos },              // 阻塞地形
    UnblockTerrain { position: GridPos },            // 解除阻塞
    SpawnBurst { station: String, count: u32 },      // 乘客爆发
    RouteDisruption { positions: Vec<GridPos> },     // 路线中断
    NewPassengerDemand { rule: PassengerSpawnRule }, // 新需求
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WinCondition {
    pub condition_type: WinConditionType,
    pub target_value: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WinConditionType {
    ServeAllPassengers,                      // 服务所有乘客
    AverageWaitTime { max_time: f32 },       // 平均等待时间低于目标
    RouteEfficiency { max_segments: u32 },   // 使用路段数少于目标
    PassengerSatisfaction { min_rate: f32 }, // 乘客满意度高于目标
    SurviveTime { duration: f32 },           // 存活指定时间
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LevelConfig {
    pub id: String,
    pub name: String,
    pub description: String,
    pub difficulty: u8, // 难度等级 1-5

    // 地图设置
    pub grid_size: (u32, u32),     // 网格大小
    pub terrain: Vec<TerrainTile>, // 地形配置

    // 站点配置
    pub stations: Vec<StationConfig>,

    // 预设路线
    pub prebuilt_routes: Vec<PrebuiltRoute>,

    // 可用资源
    pub available_segments: HashMap<RouteSegmentType, u32>, // 可用路段数量
    pub construction_budget: Option<u32>,                   // 建设预算

    // 乘客规则
    pub passenger_rules: Vec<PassengerSpawnRule>,

    // 动态事件
    pub dynamic_events: Vec<DynamicEvent>,

    // 胜利条件
    pub win_conditions: Vec<WinCondition>,

    // 时间设置
    pub time_limit: Option<f32>, // 时间限制
    pub game_speed: f32,         // 游戏速度倍率
}

#[derive(Component)]
pub struct Level {
    pub config: LevelConfig,
    pub current_time: f32,
    pub is_completed: bool,
    pub events_triggered: Vec<usize>, // 已触发的事件索引
}

#[derive(Component)]
pub struct Station {
    pub config: StationConfig,
    pub current_passengers: Vec<Entity>, // 当前等待的乘客
    pub connections: Vec<Entity>,        // 连接的路线段
}

#[derive(Component)]
pub struct RouteSegment {
    pub segment_type: RouteSegmentType,
    pub direction: Direction,
    pub grid_position: GridPos,
    pub is_locked: bool,
    pub connections: [Option<Entity>; 4], // 四个方向的连接
}

#[derive(Component)]
pub struct Terrain {
    pub terrain_type: TerrainType,
    pub grid_position: GridPos,
}
