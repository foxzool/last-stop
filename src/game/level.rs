// 生成主要关卡

use crate::{
    game::{
        grid::{Direction, GridPos, RouteSegmentType, TerrainType},
        passenger::{PassengerColor, PassengerDemand},
    },
    screens::Screen,
};
use bevy::{platform::collections::HashMap, prelude::*};
use serde::{Deserialize, Serialize};

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<WantLevel>()
        .init_resource::<LevelManager>();
    app.add_systems(OnEnter(Screen::Gameplay), spawn_level);

    app.add_systems(
        Update,
        (update_passenger_spawning, handle_dynamic_events).run_if(in_state(Screen::Gameplay)),
    );
}

#[derive(Resource, Default)]
pub struct WantLevel(pub u8);

fn spawn_level(
    want_level: Res<WantLevel>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut level_manager: ResMut<LevelManager>,
) {
    let level = if want_level.0 == 1 {
        create_tutorial_level()
    } else {
        create_tutorial_level()
    };

    // 在Bevy系统中生成地图
    generate_level_map(&mut commands, &asset_server, &level, 64.0);
    level_manager.current_level = Some(level);
}

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
    BusStop,     // 普通公交站
    TransferHub, // 换乘枢纽
    Terminal,    // 始发/终点站
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

#[derive(Component, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Station {
    pub position: GridPos,
    pub station_type: StationType,
    pub name: String,
    pub capacity: u32,                        // 站台容量
    pub passenger_types: Vec<PassengerColor>, // 这个站会出现的乘客类型
}

#[derive(Component)]
pub struct Terrain {
    pub terrain_type: TerrainType,
    pub grid_position: GridPos,
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
    pub condition_type: ObjectiveType,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Resource)]
pub struct LevelData {
    pub id: String,
    pub name: String,
    pub description: String,
    pub difficulty: u32, // 1-5 难度等级

    // 地图基础信息
    pub grid_size: (u32, u32), // 地图网格大小 (width, height)
    pub terrain: HashMap<GridPos, TerrainType>, // 地形数据

    // 站点信息
    pub stations: Vec<Station>,

    // 乘客需求
    pub passenger_demands: Vec<PassengerDemand>,

    // 可用路线段
    pub available_segments: Vec<AvailableSegment>,

    // 关卡目标
    pub objectives: Vec<ObjectiveCondition>,

    // 预设路线（可选，用于教学关卡）
    pub preset_routes: Vec<PresetRoute>,

    // 动态事件（用于实时挑战关卡）
    pub dynamic_events: Vec<DynamicEvent>,

    // 评分配置
    pub scoring: ScoringConfig,
}

impl LevelData {
    // 将网格位置转换为世界坐标
    // 根据grid_size计算偏移值
    pub fn grid_to_world(&self, grid_pos: GridPos) -> Vec2 {
        let half_width = self.grid_size.0 as f32 / 2.0;
        let half_height = self.grid_size.1 as f32 / 2.0;
        Vec2::new(
            (grid_pos.x - half_width as i32) as f32 * 64.0,
            (grid_pos.y - half_height as i32) as f32 * 64.0, // fixme: tile size
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PresetRoute {
    pub segments: Vec<(GridPos, RouteSegmentType, u32)>, // 位置、类型、旋转角度
    pub is_removable: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DynamicEvent {
    pub trigger_time: f32, // 触发时间（秒）
    pub event_type: EventType,
    pub duration: Option<f32>, // 持续时间
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EventType {
    SegmentFailure(GridPos),              // 路段故障
    SurgePassengers(PassengerColor, f32), // 客流激增
    NewDemand(PassengerDemand),           // 新需求出现
    StationOverload(String),              // 站点过载
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScoringConfig {
    pub base_points: u32,
    pub efficiency_bonus: u32,
    pub speed_bonus: u32,
    pub cost_bonus: u32,
}

// ============ Bevy 组件系统 ============

#[derive(Component)]
pub struct GridTile {
    pub grid_pos: GridPos,
    pub terrain_type: TerrainType,
}

#[derive(Component)]
pub struct StationEntity {
    pub station_data: Station,
    pub current_passengers: u32,
}

#[derive(Component)]
pub struct RouteSegment {
    pub grid_pos: GridPos,
    pub segment_type: RouteSegmentType,
    pub rotation: u32, // 0, 90, 180, 270 degrees
    pub is_active: bool,
}

#[derive(Component)]
pub struct PassengerEntity {
    pub color: PassengerColor,
    pub origin: String,
    pub destination: String,
    pub current_patience: f32,
    pub path: Vec<GridPos>,
}

#[derive(Resource)]
pub struct LevelManager {
    pub current_level: Option<LevelData>,
    pub tile_size: f32,
}

impl Default for LevelManager {
    fn default() -> Self {
        Self {
            current_level: None,
            tile_size: 64.0,
        }
    }
}

// 地图生成核心函数
pub fn generate_level_map(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    level_data: &LevelData,
    tile_size: f32,
) {
    let (width, height) = level_data.grid_size;

    // 生成地形网格
    for x in 0..width as i32 {
        for y in 0..height as i32 {
            let grid_pos = GridPos::new(x, y);
            let world_pos = level_data.grid_to_world(grid_pos);

            let terrain_type = level_data
                .terrain
                .get(&grid_pos)
                .cloned()
                .unwrap_or(TerrainType::Empty);

            let texture_path = get_terrain_texture(&terrain_type);

            commands.spawn((
                Sprite::from_image(asset_server.load(texture_path)),
                Transform::from_translation(world_pos.extend(0.0)),
                GridTile {
                    grid_pos,
                    terrain_type,
                },
            ));
        }
    }

    // 生成站点
    for station in &level_data.stations {
        let world_pos = level_data.grid_to_world(station.position);
        let texture_path = get_station_texture(&station.station_type);

        commands.spawn((
            Sprite::from_image(asset_server.load(texture_path)),
            Transform::from_translation(world_pos.extend(1.0)),
            StationEntity {
                station_data: station.clone(),
                current_passengers: 0,
            },
        ));
    }

    // 生成预设路线
    for preset_route in &level_data.preset_routes {
        for (pos, segment_type, rotation) in &preset_route.segments {
            let world_pos = level_data.grid_to_world(*pos);
            let texture_path = get_segment_texture(segment_type);

            commands.spawn((
                Sprite::from_image(asset_server.load(texture_path)),
                Transform::from_translation(world_pos.extend(0.5)).with_rotation(
                    Quat::from_rotation_z((*rotation as f32) * std::f32::consts::PI / 180.0),
                ),
                RouteSegment {
                    grid_pos: *pos,
                    segment_type: segment_type.clone(),
                    rotation: *rotation,
                    is_active: true,
                },
            ));
        }
    }
}

// 纹理路径辅助函数
fn get_terrain_texture(terrain_type: &TerrainType) -> &'static str {
    match terrain_type {
        TerrainType::Empty => "textures/terrain/grass.png",
        TerrainType::Building => "textures/terrain/building.png",
        TerrainType::Water => "textures/terrain/water.png",
        TerrainType::Park => "textures/terrain/park.png",
        TerrainType::Mountain => "textures/terrain/mountain.png",
    }
}

fn get_station_texture(station_type: &StationType) -> &'static str {
    match station_type {
        StationType::BusStop => "textures/stations/bus_stop.png",
        StationType::TransferHub => "textures/stations/transfer_hub.png",
        StationType::Terminal => "textures/stations/terminal.png",
    }
}

fn get_segment_texture(segment_type: &RouteSegmentType) -> &'static str {
    match segment_type {
        RouteSegmentType::Straight => "textures/routes/straight.png",
        RouteSegmentType::Curve => "textures/routes/curve.png",
        RouteSegmentType::TSplit => "textures/routes/t_split.png",
        RouteSegmentType::Cross => "textures/routes/cross.png",
        RouteSegmentType::Bridge => "textures/routes/bridge.png",
        RouteSegmentType::Tunnel => "textures/routes/tunnel.png",
    }
}

fn update_passenger_spawning(
    time: Res<Time>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    level_manager: Res<LevelManager>,
    mut stations: Query<&mut StationEntity>,
) {
    if let Some(level_data) = &level_manager.current_level {
        for demand in &level_data.passenger_demands {
            // 检查是否在生成时间窗口内
            if let Some((start, end)) = demand.spawn_time_range {
                let current_time = time.elapsed_secs();
                if current_time < start || current_time > end {
                    continue;
                }
            }

            // 按概率生成乘客
            if rand::random::<f32>() < demand.spawn_rate * time.delta_secs() {
                spawn_passenger(&mut commands, &asset_server, demand, &level_manager);
            }
        }
    }
}

fn handle_dynamic_events(
    time: Res<Time>,
    level_manager: Res<LevelManager>,
    mut route_segments: Query<&mut RouteSegment>,
) {
    if let Some(level_data) = &level_manager.current_level {
        let current_time = time.elapsed_secs();

        for event in &level_data.dynamic_events {
            if (current_time - event.trigger_time).abs() < 0.1 {
                // 触发事件
                match &event.event_type {
                    EventType::SegmentFailure(pos) => {
                        // 禁用指定位置的路段
                        for mut segment in route_segments.iter_mut() {
                            if segment.grid_pos == *pos {
                                segment.is_active = false;
                            }
                        }
                    }
                    EventType::SurgePassengers(color, multiplier) => {
                        // 处理客流激增
                        println!("客流激增: {:?} 乘客增加 {}倍", color, multiplier);
                    }
                    EventType::NewDemand(demand) => {
                        // 添加新的乘客需求
                        println!("新需求出现: {} -> {}", demand.origin, demand.destination);
                    }
                    EventType::StationOverload(station_name) => {
                        // 处理站点过载
                        println!("站点过载: {}", station_name);
                    }
                }
            }
        }
    }
}

fn spawn_passenger(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    demand: &PassengerDemand,
    level_manager: &LevelManager,
) {
    let texture_path = match demand.color {
        PassengerColor::Red => "textures/passengers/red.png",
        PassengerColor::Blue => "textures/passengers/blue.png",
        PassengerColor::Green => "textures/passengers/green.png",
        PassengerColor::Yellow => "textures/passengers/yellow.png",
        PassengerColor::Purple => "textures/passengers/purple.png",
        PassengerColor::Orange => "textures/passengers/orange.png",
    };

    // 找到起点站的位置
    if let Some(level_data) = &level_manager.current_level {
        if let Some(origin_station) = level_data.stations.iter().find(|s| s.name == demand.origin) {
            let world_pos = level_data.grid_to_world(origin_station.position);

            commands.spawn((
                Sprite::from_image(asset_server.load(texture_path)),
                Transform::from_translation(world_pos.extend(2.0)),
                PassengerEntity {
                    color: demand.color,
                    origin: demand.origin.clone(),
                    destination: demand.destination.clone(),
                    current_patience: demand.patience,
                    path: Vec::new(),
                },
            ));
        }
    }
}

// ============ 示例关卡数据 ============

pub fn create_tutorial_level() -> LevelData {
    let mut terrain = HashMap::new();

    // 创建简单的教学关卡地形
    for x in 0..10 {
        for y in 0..8 {
            terrain.insert(GridPos::new(x, y), TerrainType::Empty);
        }
    }

    // 添加一些障碍物
    terrain.insert(GridPos::new(4, 3), TerrainType::Building);
    terrain.insert(GridPos::new(4, 4), TerrainType::Building);
    terrain.insert(GridPos::new(6, 2), TerrainType::Water);
    terrain.insert(GridPos::new(7, 2), TerrainType::Water);

    LevelData {
        id: "tutorial_01".to_string(),
        name: "第一次连接".to_string(),
        description: "学习基本的路线连接操作，将红色乘客从A站送到B站".to_string(),
        difficulty: 1,
        grid_size: (10, 8),
        terrain,
        stations: vec![
            Station {
                position: GridPos::new(1, 4),
                station_type: StationType::Terminal,
                name: "A站".to_string(),
                capacity: 10,
                passenger_types: vec![PassengerColor::Red],
            },
            Station {
                position: GridPos::new(8, 4),
                station_type: StationType::Terminal,
                name: "B站".to_string(),
                capacity: 10,
                passenger_types: vec![],
            },
        ],
        passenger_demands: vec![PassengerDemand {
            color: PassengerColor::Red,
            origin: "A站".to_string(),
            destination: "B站".to_string(),
            spawn_rate: 0.5,
            patience: 30.0,
            spawn_time_range: None,
        }],
        available_segments: vec![
            AvailableSegment {
                segment_type: RouteSegmentType::Straight,
                count: 8,
                cost: 1,
            },
            AvailableSegment {
                segment_type: RouteSegmentType::Curve,
                count: 4,
                cost: 2,
            },
        ],
        objectives: vec![ObjectiveCondition {
            description: "连接所有乘客到目的地".to_string(),
            condition_type: ObjectiveType::ConnectAllPassengers,
        }],
        preset_routes: vec![],
        dynamic_events: vec![],
        scoring: ScoringConfig {
            base_points: 100,
            efficiency_bonus: 50,
            speed_bonus: 25,
            cost_bonus: 25,
        },
    }
}
