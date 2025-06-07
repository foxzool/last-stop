// src/bus_puzzle/level_system.rs

use crate::bus_puzzle::{
    GameState, GameStateEnum, GridPos, GridTile, LevelManager, PASSENGER_Z, PassengerColor,
    PassengerEntity, PathfindingAgent, ROUTE_Z, RouteSegment, RouteSegmentType, STATION_Z,
    StationEntity, StationType, TERRAIN_Z, TerrainType, get_passenger_color,
    spawn_passenger_no_texture,
};
use bevy::{platform::collections::HashMap, prelude::*};
use rand::Rng;
use serde::{Deserialize, Serialize};

// ============ 关卡数据结构 ============

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Station {
    pub position: GridPos,
    pub station_type: StationType,
    pub name: String,
    pub capacity: u32,                        // 站台容量
    pub passenger_types: Vec<PassengerColor>, // 这个站会出现的乘客类型
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PassengerDemand {
    pub color: PassengerColor,
    pub origin: String,                       // 起点站名称
    pub destination: String,                  // 终点站名称
    pub spawn_rate: f32,                      // 每秒生成数量
    pub patience: f32,                        // 耐心值（秒）
    pub spawn_time_range: Option<(f32, f32)>, // 生成时间窗口
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AvailableSegment {
    pub segment_type: RouteSegmentType,
    pub count: u32,
    pub cost: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObjectiveCondition {
    pub description: String,
    pub condition_type: ObjectiveType,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ObjectiveType {
    ConnectAllPassengers,
    MaxTransfers(u32),
    MaxSegments(u32),
    MaxCost(u32),
    MinEfficiency(f32),
    TimeLimit(f32),
    PassengerSatisfaction(f32),
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

// ============ 主要关卡数据结构 ============
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

// ============ 地图生成插件 ============

pub struct LevelGenerationPlugin;

impl Plugin for LevelGenerationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LevelManager>()
            .add_systems(Startup, setup_debug_level)
            .add_systems(
                Update,
                (
                    sync_level_data,
                    update_passenger_spawning_no_texture.run_if(in_state(GameStateEnum::Playing)),
                    handle_dynamic_events.run_if(in_state(GameStateEnum::Playing)),
                    debug_passenger_spawning,
                    manual_spawn_passenger_debug_no_texture
                        .run_if(in_state(GameStateEnum::Playing)),
                )
                    .chain(),
            );
    }
}

// 添加数据同步系统
fn sync_level_data(mut level_manager: ResMut<LevelManager>, game_state: Res<GameState>) {
    if let Some(level_data) = &game_state.current_level {
        if level_manager
            .current_level
            .as_ref()
            .map_or(true, |current| current.id != level_data.id)
        {
            level_manager.current_level = Some(level_data.clone());
            info!("同步关卡数据: {}", level_data.name);
        }
    }
}

// 无纹理的乘客生成系统
fn update_passenger_spawning_no_texture(
    time: Res<Time>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    game_state: Res<GameState>,
    mut stations: Query<&mut StationEntity>,
) {
    if let Some(level_data) = &game_state.current_level {
        let mut rng = rand::rng();
        let current_time = time.elapsed_secs();

        for (demand_index, demand) in level_data.passenger_demands.iter().enumerate() {
            // 检查时间窗口
            if let Some((start, end)) = demand.spawn_time_range {
                if current_time < start || current_time > end {
                    continue;
                }
            }

            // 计算生成概率
            let spawn_chance = demand.spawn_rate * time.delta_secs();
            let random_value = rng.random::<f32>();

            if random_value < spawn_chance {
                spawn_passenger_no_texture(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    demand,
                    level_data,
                );
                info!(
                    "生成乘客概率检查: {} < {} = 成功",
                    random_value, spawn_chance
                );
            } else if demand_index == 0 && time.elapsed_secs() as u32 % 2 == 0 {
                // 每2秒输出一次调试信息（只对第一个需求）
                info!(
                    "生成乘客概率检查: {} >= {} = 失败",
                    random_value, spawn_chance
                );
            }
        }
    } else {
        // 每5秒警告一次没有关卡数据
        if time.elapsed_secs() as u32 % 5 == 0 {
            warn!("update_passenger_spawning_no_texture: 没有关卡数据");
        }
    }
}

// 调试系统（保持不变）
fn debug_passenger_spawning(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    game_state: Res<GameState>,
    passengers: Query<&PathfindingAgent>,
    time: Res<Time>,
) {
    if keyboard_input.just_pressed(KeyCode::F2) {
        info!("=== 乘客生成调试信息 ===");
        info!("当前游戏时间: {:.1}秒", time.elapsed_secs());
        info!("当前乘客数量: {}", passengers.iter().count());

        if let Some(level_data) = &game_state.current_level {
            info!("关卡名称: {}", level_data.name);
            info!("乘客需求数量: {}", level_data.passenger_demands.len());

            for (i, demand) in level_data.passenger_demands.iter().enumerate() {
                info!(
                    "需求 {}: {:?} {} -> {}, 生成率: {}/秒",
                    i, demand.color, demand.origin, demand.destination, demand.spawn_rate
                );

                if let Some((start, end)) = demand.spawn_time_range {
                    info!("  时间窗口: {:.1}s - {:.1}s", start, end);
                } else {
                    info!("  无时间限制");
                }

                let spawn_chance_per_second = demand.spawn_rate;
                info!("  每秒生成概率: {:.1}%", spawn_chance_per_second * 100.0);
            }

            info!("站点数量: {}", level_data.stations.len());
            for station in &level_data.stations {
                info!("站点: {} 位置: {:?}", station.name, station.position);
            }
        } else {
            error!("GameState 中没有关卡数据！");
        }
    }
}

// 手动生成测试乘客（无纹理版本）
fn manual_spawn_passenger_debug_no_texture(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    game_state: Res<GameState>,
) {
    if keyboard_input.just_pressed(KeyCode::F3) {
        info!("手动生成测试乘客");

        if let Some(level_data) = &game_state.current_level {
            if let Some(demand) = level_data.passenger_demands.first() {
                spawn_passenger_no_texture(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    demand,
                    level_data,
                );
                info!("成功手动生成测试乘客: {:?}", demand.color);
            } else {
                warn!("没有乘客需求数据可以生成测试乘客");
            }
        } else {
            error!("GameState 中没有关卡数据，无法生成测试乘客");
        }
    }
}

// 其他函数保持不变
fn setup_debug_level(mut level_manager: ResMut<LevelManager>, mut game_state: ResMut<GameState>) {
    let tutorial_level = create_tutorial_level();

    level_manager.current_level = Some(tutorial_level.clone());
    game_state.current_level = Some(tutorial_level.clone());

    let mut inventory = HashMap::new();
    for segment in &tutorial_level.available_segments {
        inventory.insert(segment.segment_type.clone(), segment.count);
    }
    game_state.player_inventory = inventory;
    game_state.objectives_completed = vec![false; tutorial_level.objectives.len()];

    info!("设置了教学关卡作为默认关卡");
}

// ============ 地图生成系统 ============

// 地图生成核心函数
pub fn generate_level_map(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    level_data: &LevelData,
    tile_size: f32,
) {
    let (width, height) = level_data.grid_size;

    // 生成地形网格（在最底层）
    for x in 0..width as i32 {
        for y in 0..height as i32 {
            let grid_pos = GridPos::new(x, y);
            let mut world_pos = grid_pos.to_world_pos(tile_size, width, height);
            world_pos.z = TERRAIN_Z; // 确保地形在最底层

            let terrain_type = level_data
                .terrain
                .get(&grid_pos)
                .cloned()
                .unwrap_or(TerrainType::Empty);

            let texture_path = get_terrain_texture(&terrain_type);

            commands.spawn((
                Sprite::from_image(asset_server.load(texture_path)),
                Transform::from_translation(world_pos),
                GridTile {
                    grid_pos,
                    terrain_type,
                },
            ));
        }
    }

    // 生成站点（在地形上方）
    for station in &level_data.stations {
        let mut world_pos = station.position.to_world_pos(tile_size, width, height);
        world_pos.z = STATION_Z; // 站点在地形上方

        let texture_path = get_station_texture(&station.station_type);

        commands.spawn((
            Sprite::from_image(asset_server.load(texture_path)),
            Transform::from_translation(world_pos),
            StationEntity {
                station_data: station.clone(),
                current_passengers: 0,
            },
        ));

        info!(
            "生成站点: {} 位置: {:?} Z={:.1}",
            station.name, world_pos, STATION_Z
        );
    }

    // 生成预设路线（在地形上方，但在站点下方）
    for preset_route in &level_data.preset_routes {
        for (pos, segment_type, rotation) in &preset_route.segments {
            let mut world_pos = pos.to_world_pos(tile_size, width, height);
            world_pos.z = ROUTE_Z; // 路线段在地形和站点之间

            let texture_path = segment_type.get_texture_path();

            commands.spawn((
                Sprite::from_image(asset_server.load(texture_path)),
                Transform::from_translation(world_pos).with_rotation(Quat::from_rotation_z(
                    (*rotation as f32) * std::f32::consts::PI / 180.0,
                )),
                RouteSegment {
                    grid_pos: *pos,
                    segment_type: segment_type.clone(),
                    rotation: *rotation,
                    is_active: true,
                },
            ));

            info!(
                "生成预设路线段: {:?} 位置: {:?} Z={:.1}",
                segment_type, world_pos, ROUTE_Z
            );
        }
    }

    info!(
        "地图生成完成，Z层级: 地形={:.1}, 路线={:.1}, 站点={:.1}, 乘客={:.1}",
        TERRAIN_Z, ROUTE_Z, STATION_Z, PASSENGER_Z
    );
}

// ============ 系统函数 ============

fn handle_level_load(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut level_manager: ResMut<LevelManager>,
    // 这里可以添加加载关卡的触发条件
) {
    // 示例：加载关卡数据的逻辑
    // if let Some(level_data) = load_level_from_file("level_01.json") {
    //     generate_level_map(&mut commands, &asset_server, &level_data, level_manager.tile_size);
    //     level_manager.current_level = Some(level_data);
    // }
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
            let world_pos = origin_station
                .position
                .to_world_pos_with_level(level_manager.tile_size, level_data);

            commands.spawn((
                Sprite::from_image(asset_server.load(texture_path)),
                Transform::from_translation(world_pos + Vec3::Z * 2.0),
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

// ============ 纹理路径辅助函数 ============

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

// ============ 示例关卡创建函数 ============

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

pub fn create_advanced_level() -> LevelData {
    let mut terrain = HashMap::new();

    // 创建一个更大的地图，包含多种地形
    for x in 0..15 {
        for y in 0..12 {
            let terrain_type = match (x, y) {
                (6..=8, 4..=7) => TerrainType::Water,
                (2..=4, 8..=10) => TerrainType::Building,
                (10..=12, 2..=4) => TerrainType::Building,
                (0..=2, 0..=2) => TerrainType::Park,
                (13..=14, 9..=11) => TerrainType::Mountain,
                _ => TerrainType::Empty,
            };
            terrain.insert(GridPos::new(x, y), terrain_type);
        }
    }

    let stations = vec![
        Station {
            position: GridPos::new(1, 5),
            station_type: StationType::Terminal,
            name: "住宅区站".to_string(),
            capacity: 15,
            passenger_types: vec![PassengerColor::Red, PassengerColor::Blue],
        },
        Station {
            position: GridPos::new(7, 1),
            station_type: StationType::TransferHub,
            name: "中央换乘站".to_string(),
            capacity: 25,
            passenger_types: vec![],
        },
        Station {
            position: GridPos::new(13, 6),
            station_type: StationType::Terminal,
            name: "商业区站".to_string(),
            capacity: 20,
            passenger_types: vec![],
        },
        Station {
            position: GridPos::new(5, 10),
            station_type: StationType::BusStop,
            name: "学校站".to_string(),
            capacity: 12,
            passenger_types: vec![PassengerColor::Green],
        },
        Station {
            position: GridPos::new(11, 9),
            station_type: StationType::Terminal,
            name: "工业区站".to_string(),
            capacity: 18,
            passenger_types: vec![],
        },
    ];

    let passenger_demands = vec![
        PassengerDemand {
            color: PassengerColor::Red,
            origin: "住宅区站".to_string(),
            destination: "商业区站".to_string(),
            spawn_rate: 0.8,
            patience: 60.0,
            spawn_time_range: Some((0.0, 180.0)),
        },
        PassengerDemand {
            color: PassengerColor::Blue,
            origin: "住宅区站".to_string(),
            destination: "工业区站".to_string(),
            spawn_rate: 0.6,
            patience: 45.0,
            spawn_time_range: Some((30.0, 210.0)),
        },
        PassengerDemand {
            color: PassengerColor::Green,
            origin: "学校站".to_string(),
            destination: "中央换乘站".to_string(),
            spawn_rate: 1.0,
            patience: 40.0,
            spawn_time_range: Some((60.0, 120.0)),
        },
        PassengerDemand {
            color: PassengerColor::Yellow,
            origin: "商业区站".to_string(),
            destination: "住宅区站".to_string(),
            spawn_rate: 0.5,
            patience: 50.0,
            spawn_time_range: Some((120.0, 300.0)),
        },
    ];

    let available_segments = vec![
        AvailableSegment {
            segment_type: RouteSegmentType::Straight,
            count: 12,
            cost: 1,
        },
        AvailableSegment {
            segment_type: RouteSegmentType::Curve,
            count: 8,
            cost: 2,
        },
        AvailableSegment {
            segment_type: RouteSegmentType::TSplit,
            count: 4,
            cost: 3,
        },
        AvailableSegment {
            segment_type: RouteSegmentType::Cross,
            count: 3,
            cost: 4,
        },
        AvailableSegment {
            segment_type: RouteSegmentType::Bridge,
            count: 2,
            cost: 5,
        },
        AvailableSegment {
            segment_type: RouteSegmentType::Tunnel,
            count: 1,
            cost: 6,
        },
    ];

    let objectives = vec![
        ObjectiveCondition {
            description: "连接所有乘客到目的地".to_string(),
            condition_type: ObjectiveType::ConnectAllPassengers,
        },
        ObjectiveCondition {
            description: "最多使用25个路线段".to_string(),
            condition_type: ObjectiveType::MaxSegments(25),
        },
        ObjectiveCondition {
            description: "总成本不超过50".to_string(),
            condition_type: ObjectiveType::MaxCost(50),
        },
        ObjectiveCondition {
            description: "平均换乘次数不超过1次".to_string(),
            condition_type: ObjectiveType::MaxTransfers(1),
        },
        ObjectiveCondition {
            description: "在5分钟内完成".to_string(),
            condition_type: ObjectiveType::TimeLimit(300.0),
        },
    ];

    let dynamic_events = vec![
        DynamicEvent {
            trigger_time: 90.0,
            event_type: EventType::SurgePassengers(PassengerColor::Red, 2.0),
            duration: Some(30.0),
        },
        DynamicEvent {
            trigger_time: 150.0,
            event_type: EventType::SegmentFailure(GridPos::new(7, 5)),
            duration: Some(20.0),
        },
        DynamicEvent {
            trigger_time: 200.0,
            event_type: EventType::NewDemand(PassengerDemand {
                color: PassengerColor::Purple,
                origin: "工业区站".to_string(),
                destination: "学校站".to_string(),
                spawn_rate: 1.5,
                patience: 30.0,
                spawn_time_range: None,
            }),
            duration: Some(60.0),
        },
    ];

    LevelData {
        id: "advanced_01".to_string(),
        name: "城市交通网络".to_string(),
        description: "设计一个高效的城市交通网络，应对复杂的乘客需求和突发事件".to_string(),
        difficulty: 4,
        grid_size: (15, 12),
        terrain,
        stations,
        passenger_demands,
        available_segments,
        objectives,
        preset_routes: vec![],
        dynamic_events,
        scoring: ScoringConfig {
            base_points: 200,
            efficiency_bonus: 100,
            speed_bonus: 50,
            cost_bonus: 50,
        },
    }
} // src/bus_puzzle/level_system.rs
