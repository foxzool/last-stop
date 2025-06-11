// src/bus_puzzle/level_system.rs

use crate::bus_puzzle::{
    get_passenger_color, get_text, get_text_with_args, AgentState, CurrentLanguage, GameState,
    GameStateEnum, GridPos, GridTile, Language, LevelCompletedEvent, LevelManager, LocalizedText,
    PassengerColor, PassengerEntity, PassengerSpawnedEvent, PathfindingAgent, RouteSegment,
    RouteSegmentType, StationEntity, StationType, TerrainType, CENTRAL_HUB,
    DEFAULT_PASSENGER_PATIENCE, DEFAULT_TILE_SIZE, LEVEL_MULTIPLE, LEVEL_TRANSFER, LEVEL_TUTORIAL,
    MULTIPLE_DESCRIPTION, NORTHEAST_STATION, NORTH_STATION, OBJECTIVE_CONNECT_ALL,
    OBJECTIVE_MAX_COST, OBJECTIVE_MAX_SEGMENTS, OBJECTIVE_MAX_TRANSFERS,
    OBJECTIVE_PASSENGER_SATISFACTION, OBJECTIVE_TIME_LIMIT, PASSENGER_Z, ROUTE_Z,
    SOUTHEAST_STATION, SOUTH_STATION, START_STATION, STATION_A, STATION_B, STATION_C, STATION_Z,
    TARGET_STATION_A, TARGET_STATION_B, TARGET_STATION_C, TERRAIN_Z, TIME_PRESSURE_DESCRIPTION,
    TRANSFER_DESCRIPTION, TRANSFER_HUB, TUTORIAL_DESCRIPTION,
};
use bevy::{platform::collections::HashMap, prelude::*};
use rand::Rng;

// ============ 关卡数据结构 ============

// 直接在现有的LevelData中添加本地化支持
#[derive(Debug, Clone, PartialEq, Resource)]
pub struct LevelData {
    pub id: String,
    // 保留原有字段作为默认值，添加可选的本地化键
    pub name: String,                             // 保留：用作fallback或默认语言
    pub description: String,                      // 保留：用作fallback
    pub name_key: Option<&'static LocalizedText>, // 新增：可选的本地化键
    pub description_key: Option<&'static LocalizedText>, // 新增：可选的本地化键

    pub difficulty: u32,
    pub grid_size: (u32, u32),
    pub terrain: HashMap<GridPos, TerrainType>,
    pub stations: Vec<Station>,                  // 保持现有结构
    pub passenger_demands: Vec<PassengerDemand>, // 保持现有结构
    pub available_segments: Vec<AvailableSegment>,
    pub objectives: Vec<ObjectiveCondition>, // 保持现有结构
    pub preset_routes: Vec<PresetRoute>,
    pub dynamic_events: Vec<DynamicEvent>,
    pub scoring: ScoringConfig,
}

// 现有的Station结构也保持不变，但添加本地化支持
#[derive(Debug, Clone, PartialEq)]
pub struct Station {
    pub position: GridPos,
    pub station_type: StationType,
    pub name: String,                             // 保留：作为默认值
    pub name_key: Option<&'static LocalizedText>, // 新增：可选的本地化键
    pub capacity: u32,
    pub passenger_types: Vec<PassengerColor>,
}

// PassengerDemand同样处理
#[derive(Debug, Clone, PartialEq)]
pub struct PassengerDemand {
    pub color: PassengerColor,
    pub origin: String,                                  // 保留：作为默认值
    pub destination: String,                             // 保留：作为默认值
    pub origin_key: Option<&'static LocalizedText>,      // 新增：可选的本地化键
    pub destination_key: Option<&'static LocalizedText>, // 新增：可选的本地化键
    pub spawn_rate: f32,
    pub patience: f32,
    pub spawn_time_range: Option<(f32, f32)>,
    pub total_count: Option<u32>,
    pub spawned_count: u32,
}

// ObjectiveCondition同样处理
#[derive(Debug, Clone, PartialEq)]
pub struct ObjectiveCondition {
    pub description: String,                             // 保留：作为默认值
    pub description_key: Option<&'static LocalizedText>, // 新增：可选的本地化键
    pub condition_type: ObjectiveType,
}

// ============ 实现本地化获取方法 ============
#[allow(dead_code)]
impl LevelData {
    /// 获取本地化的关卡名称
    pub fn get_localized_name(&self, language: Language) -> String {
        if let Some(name_key) = self.name_key {
            name_key.get(language).to_string()
        } else {
            self.name.clone() // fallback到原始名称
        }
    }

    /// 获取本地化的关卡描述
    pub fn get_localized_description(&self, language: Language) -> String {
        if let Some(desc_key) = self.description_key {
            desc_key.get(language).to_string()
        } else {
            self.description.clone() // fallback到原始描述
        }
    }
}

#[allow(dead_code)]
impl Station {
    /// 获取本地化的站点名称
    pub fn get_localized_name(&self, language: Language) -> String {
        if let Some(name_key) = self.name_key {
            name_key.get(language).to_string()
        } else {
            self.name.clone() // fallback到原始名称
        }
    }
}

#[allow(dead_code)]
impl PassengerDemand {
    /// 获取本地化的起点名称
    pub fn get_localized_origin(&self, language: Language) -> String {
        if let Some(origin_key) = self.origin_key {
            origin_key.get(language).to_string()
        } else {
            self.origin.clone()
        }
    }

    /// 获取本地化的终点名称
    pub fn get_localized_destination(&self, language: Language) -> String {
        if let Some(dest_key) = self.destination_key {
            dest_key.get(language).to_string()
        } else {
            self.destination.clone()
        }
    }
}

#[allow(dead_code)]
impl ObjectiveCondition {
    /// 获取本地化的目标描述
    pub fn get_localized_description(&self, language: Language) -> String {
        if let Some(desc_key) = self.description_key {
            match &self.condition_type {
                ObjectiveType::ConnectAllPassengers => desc_key.get(language).to_string(),
                ObjectiveType::MaxTransfers(count) => {
                    get_text_with_args(desc_key, language, &[&count.to_string()])
                }
                ObjectiveType::MaxSegments(count) => {
                    get_text_with_args(desc_key, language, &[&count.to_string()])
                }
                ObjectiveType::MaxCost(cost) => {
                    get_text_with_args(desc_key, language, &[&cost.to_string()])
                }
                ObjectiveType::TimeLimit(time) => {
                    get_text_with_args(desc_key, language, &[&time.to_string()])
                }
                ObjectiveType::PassengerSatisfaction(satisfaction) => {
                    let percentage = (*satisfaction * 100.0) as u32;
                    get_text_with_args(desc_key, language, &[&percentage.to_string()])
                }
                _ => desc_key.get(language).to_string(),
            }
        } else {
            self.description.clone() // fallback到原始描述
        }
    }
}

// ============ 优化后的关卡创建函数 ============

#[derive(Debug, Clone, PartialEq)]
pub struct AvailableSegment {
    pub segment_type: RouteSegmentType,
    pub count: u32,
    pub cost: u32,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum ObjectiveType {
    ConnectAllPassengers,
    MaxTransfers(u32),
    MaxSegments(u32),
    MaxCost(u32),
    MinEfficiency(f32),
    TimeLimit(f32),
    PassengerSatisfaction(f32),
}

#[derive(Debug, Clone, PartialEq)]
pub struct PresetRoute {
    pub segments: Vec<(GridPos, RouteSegmentType, u32)>,
    pub is_removable: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DynamicEvent {
    pub trigger_time: f32,
    pub event_type: EventType,
    pub duration: Option<f32>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum EventType {
    SegmentFailure(GridPos),
    SurgePassengers(PassengerColor, f32),
    NewDemand(PassengerDemand),
    StationOverload(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScoringConfig {
    pub base_points: u32,
    pub efficiency_bonus: u32,
    pub speed_bonus: u32,
    pub cost_bonus: u32,
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
                    (
                        update_passenger_spawning,
                        handle_passenger_spawn,
                        handle_level_events,
                    )
                        .run_if(in_state(GameStateEnum::Playing)),
                    handle_dynamic_events.run_if(in_state(GameStateEnum::Playing)),
                    debug_passenger_spawning,
                    manual_spawn_passenger_debug.run_if(in_state(GameStateEnum::Playing)),
                )
                    .chain(),
            );
    }
}

fn sync_level_data(mut level_manager: ResMut<LevelManager>, game_state: Res<GameState>) {
    if let Some(level_data) = &game_state.current_level {
        if level_manager
            .current_level
            .as_ref()
            .is_none_or(|current| current.id != level_data.id)
        {
            level_manager.current_level = Some(level_data.clone());
            info!("同步关卡数据: {}", level_data.name);
        }
    }
}

fn update_passenger_spawning(
    time: Res<Time>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut game_state: ResMut<GameState>,
) {
    // 重要：使用游戏时间而不是系统时间来判断乘客生成
    let game_time = game_state.game_time;
    if let Some(level_data) = &mut game_state.current_level {
        let mut rng = rand::thread_rng();

        // 提前获取不可变借用的数据
        let level_data_ref = level_data.clone();

        for demand in level_data.passenger_demands.iter_mut() {
            // 检查是否达到生成上限
            if let Some(total_count) = demand.total_count {
                if demand.spawned_count >= total_count {
                    continue; // 跳过这个需求
                }
            }

            // 检查时间窗口 - 使用游戏时间
            if let Some((start, end)) = demand.spawn_time_range {
                if game_time < start || game_time > end {
                    continue;
                }
            }

            // 计算生成概率
            let spawn_chance = demand.spawn_rate * time.delta_secs();
            let random_value = rng.r#gen::<f32>();

            if random_value < spawn_chance {
                // 在生成前增加计数
                demand.spawned_count += 1;

                spawn_passenger_with_icon(&mut commands, &asset_server, demand, &level_data_ref);

                info!(
                    "生成乘客 {:?}: {}/{:?} (游戏时间: {:.1}s)",
                    demand.color, demand.spawned_count, demand.total_count, game_time
                );
            }
        }
    }
}

fn debug_passenger_spawning(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    game_state: Res<GameState>,
    passengers: Query<&PathfindingAgent>,
    time: Res<Time>,
) {
    if keyboard_input.just_pressed(KeyCode::F2) {
        info!("=== 乘客生成调试信息 ===");
        info!("系统时间: {:.1}秒", time.elapsed_secs());
        info!("游戏时间: {:.1}秒", game_state.game_time);
        info!("当前乘客数量: {}", passengers.iter().count());
        info!(
            "乘客统计: 生成={}, 到达={}, 放弃={}",
            game_state.passenger_stats.total_spawned,
            game_state.passenger_stats.total_arrived,
            game_state.passenger_stats.total_gave_up
        );

        if let Some(level_data) = &game_state.current_level {
            info!("关卡名称: {}", level_data.name);
            info!("乘客需求数量: {}", level_data.passenger_demands.len());

            for (i, demand) in level_data.passenger_demands.iter().enumerate() {
                let status = if let Some(total) = demand.total_count {
                    if demand.spawned_count >= total {
                        "已完成"
                    } else {
                        "进行中"
                    }
                } else {
                    "无限制"
                };

                let time_status = if let Some((start, end)) = demand.spawn_time_range {
                    format!("时间窗口: {:.1}-{:.1}s", start, end)
                } else {
                    "无时间限制".to_string()
                };

                info!(
                    "需求 {}: {:?} {} -> {}, 生成率: {}/秒, 已生成: {}/{:?} ({}), {}",
                    i,
                    demand.color,
                    demand.origin,
                    demand.destination,
                    demand.spawn_rate,
                    demand.spawned_count,
                    demand.total_count,
                    status,
                    time_status
                );
            }
        }
    }
}

// 使用图标的乘客生成函数
fn spawn_passenger_with_icon(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    demand: &PassengerDemand,
    level_data: &LevelData,
) {
    if let Some(origin_station) = level_data.stations.iter().find(|s| s.name == demand.origin) {
        let tile_size = DEFAULT_TILE_SIZE;
        let (grid_width, grid_height) = level_data.grid_size;
        let world_pos = origin_station
            .position
            .to_world_pos(tile_size, grid_width, grid_height);

        let passenger_world_pos = Vec3::new(world_pos.x, world_pos.y, PASSENGER_Z);

        // 根据乘客颜色选择对应的图标纹理
        let texture_path = match demand.color {
            PassengerColor::Red => "textures/passengers/red.png",
            PassengerColor::Blue => "textures/passengers/blue.png",
            PassengerColor::Green => "textures/passengers/green.png",
            PassengerColor::Yellow => "textures/passengers/yellow.png",
            PassengerColor::Purple => "textures/passengers/purple.png",
            PassengerColor::Orange => "textures/passengers/orange.png",
        };

        // 尝试加载乘客图标纹理，并设置颜色作为回退方案
        let texture_handle = asset_server.load(texture_path);
        let passenger_color = get_passenger_color(demand.color);

        let _entity = commands
            .spawn((
                Name::new(format!(
                    "Passenger {:?} {} -> {}",
                    demand.color, demand.origin, demand.destination
                )),
                Sprite {
                    image: texture_handle,
                    custom_size: Some(Vec2::new(32.0, 32.0)), // 设置合适的大小
                    color: passenger_color, // 使用颜色作为着色，如果纹理加载失败会显示纯色方块
                    ..default()
                },
                Transform::from_translation(passenger_world_pos),
                PassengerEntity {
                    color: demand.color,
                    origin: demand.origin.clone(),
                    destination: demand.destination.clone(),
                    current_patience: demand.patience,
                    path: Vec::new(),
                },
                PathfindingAgent {
                    color: demand.color,
                    origin: demand.origin.clone(),
                    destination: demand.destination.clone(),
                    current_path: Vec::new(),
                    current_step: 0,
                    state: AgentState::WaitingAtStation,
                    patience: demand.patience,
                    max_patience: demand.patience,
                    waiting_time: 0.0,
                },
            ))
            .id();

        info!(
            "生成乘客图标: {:?} {} -> {} (纹理: {})",
            demand.color, demand.origin, demand.destination, texture_path
        );
        commands.send_event(PassengerSpawnedEvent {
            color: demand.color,
            origin: demand.origin.clone(),
            destination: demand.destination.clone(),
        });
    } else {
        error!("找不到起点站: {}", demand.origin);
    }
}

fn manual_spawn_passenger_debug(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    game_state: Res<GameState>,
) {
    if keyboard_input.just_pressed(KeyCode::F3) {
        if let Some(level_data) = &game_state.current_level {
            if let Some(demand) = level_data.passenger_demands.first() {
                spawn_passenger_with_icon(&mut commands, &asset_server, demand, level_data);
                info!("手动生成测试乘客: {:?}", demand.color);
            }
        }
    }
}

fn setup_debug_level(
    mut level_manager: ResMut<LevelManager>,
    mut game_state: ResMut<GameState>,
    current_language: Res<CurrentLanguage>,
) {
    let tutorial_level = create_tutorial_level(current_language.language);

    level_manager.current_level = Some(tutorial_level.clone());
    game_state.current_level = Some(tutorial_level.clone());

    let mut inventory = HashMap::new();
    for segment in &tutorial_level.available_segments {
        inventory.insert(segment.segment_type, segment.count);
    }
    game_state.player_inventory = inventory;
    game_state.objectives_completed = vec![false; tutorial_level.objectives.len()];

    info!("设置教学关卡作为默认关卡");
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
                match &event.event_type {
                    EventType::SegmentFailure(pos) => {
                        for mut segment in route_segments.iter_mut() {
                            if segment.grid_pos == *pos {
                                segment.is_active = false;
                            }
                        }
                    }
                    EventType::SurgePassengers(color, multiplier) => {
                        info!("客流激增: {:?} 乘客增加 {}倍", color, multiplier);
                    }
                    EventType::NewDemand(demand) => {
                        info!("新需求出现: {} -> {}", demand.origin, demand.destination);
                    }
                    EventType::StationOverload(station_name) => {
                        info!("站点过载: {}", station_name);
                    }
                }
            }
        }
    }
}

// ============ 地图生成核心函数 ============

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
            let mut world_pos = grid_pos.to_world_pos(tile_size, width, height);
            world_pos.z = TERRAIN_Z;

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

    // 生成站点
    for station in &level_data.stations {
        let mut world_pos = station.position.to_world_pos(tile_size, width, height);
        world_pos.z = STATION_Z;

        let texture_path = get_station_texture(&station.station_type);

        commands.spawn((
            Sprite::from_image(asset_server.load(texture_path)),
            Transform::from_translation(world_pos),
            StationEntity {
                station_data: station.clone(),
                current_passengers: 0,
            },
        ));
    }

    // 生成预设路线
    for preset_route in &level_data.preset_routes {
        for (pos, segment_type, rotation) in &preset_route.segments {
            let mut world_pos = pos.to_world_pos(tile_size, width, height);
            world_pos.z = ROUTE_Z;

            let texture_path = segment_type.get_texture_path();

            commands.spawn((
                Sprite::from_image(asset_server.load(texture_path)),
                Transform::from_translation(world_pos).with_rotation(Quat::from_rotation_z(
                    (*rotation as f32) * std::f32::consts::PI / 180.0,
                )),
                RouteSegment {
                    grid_pos: *pos,
                    segment_type: *segment_type,
                    rotation: *rotation,
                    is_active: true,
                },
            ));
        }
    }

    info!("地图生成完成");
}

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

pub fn create_tutorial_level(current_language: Language) -> LevelData {
    let mut terrain = HashMap::new();

    for x in 0..10 {
        for y in 0..8 {
            terrain.insert(GridPos::new(x, y), TerrainType::Empty);
        }
    }

    terrain.insert(GridPos::new(4, 3), TerrainType::Building);
    terrain.insert(GridPos::new(4, 4), TerrainType::Building);
    terrain.insert(GridPos::new(6, 2), TerrainType::Water);
    terrain.insert(GridPos::new(7, 2), TerrainType::Water);

    LevelData {
        id: "tutorial_01".to_string(),
        // 设置默认文本和本地化键
        name: "First Connection".to_string(), // 默认英文
        description: "Learn basic route connection".to_string(), // 默认英文
        name_key: Some(&LEVEL_TUTORIAL),      // 本地化键
        description_key: Some(&TUTORIAL_DESCRIPTION), // 本地化键

        difficulty: 1,
        grid_size: (10, 8),
        terrain,
        stations: vec![
            Station {
                position: GridPos::new(1, 4),
                station_type: StationType::Terminal,
                name: "Station A".to_string(), // 默认英文
                name_key: Some(&STATION_A),    // 本地化键
                capacity: 10,
                passenger_types: vec![PassengerColor::Red],
            },
            Station {
                position: GridPos::new(8, 4),
                station_type: StationType::Terminal,
                name: "Station B".to_string(), // 默认英文
                name_key: Some(&STATION_B),    // 本地化键
                capacity: 10,
                passenger_types: vec![],
            },
        ],
        passenger_demands: vec![PassengerDemand {
            color: PassengerColor::Red,
            origin: "Station A".to_string(),      // 默认英文
            destination: "Station B".to_string(), // 默认英文
            origin_key: Some(&STATION_A),         // 本地化键
            destination_key: Some(&STATION_B),    // 本地化键
            spawn_rate: 0.5,
            patience: DEFAULT_PASSENGER_PATIENCE,
            spawn_time_range: None,
            total_count: Some(3),
            spawned_count: 0,
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
            AvailableSegment {
                segment_type: RouteSegmentType::TSplit,
                count: 4,
                cost: 3,
            },
            AvailableSegment {
                segment_type: RouteSegmentType::Cross,
                count: 4,
                cost: 4,
            },
        ],
        objectives: vec![ObjectiveCondition {
            description: get_text(&OBJECTIVE_CONNECT_ALL, current_language), // 默认英文
            description_key: Some(&OBJECTIVE_CONNECT_ALL),                   // 本地化键
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

pub fn create_transfer_level(language: Language) -> LevelData {
    let mut terrain = HashMap::new();

    for x in 0..12 {
        for y in 0..10 {
            terrain.insert(GridPos::new(x, y), TerrainType::Empty);
        }
    }

    // 添加一些障碍物
    terrain.insert(GridPos::new(5, 4), TerrainType::Building);
    terrain.insert(GridPos::new(5, 5), TerrainType::Building);
    terrain.insert(GridPos::new(6, 4), TerrainType::Building);
    terrain.insert(GridPos::new(6, 5), TerrainType::Building);

    LevelData {
        id: "level_02_transfer".to_string(),
        name: "".to_string(),
        description: "".to_string(),
        name_key: Some(&LEVEL_TRANSFER),
        description_key: Some(&TRANSFER_DESCRIPTION),
        difficulty: 2,
        grid_size: (12, 10),
        terrain,
        stations: vec![
            Station {
                position: GridPos::new(1, 2),
                station_type: StationType::Terminal,
                name: get_text(&STATION_A, language),
                name_key: Some(&STATION_A),
                capacity: 15,
                passenger_types: vec![PassengerColor::Red, PassengerColor::Blue],
            },
            Station {
                position: GridPos::new(5, 7),
                station_type: StationType::TransferHub,
                name: get_text(&TRANSFER_HUB, language),
                name_key: Some(&TRANSFER_HUB),
                capacity: 20,
                passenger_types: vec![],
            },
            Station {
                position: GridPos::new(10, 2),
                station_type: StationType::Terminal,
                name: get_text(&STATION_B, language),
                name_key: Some(&STATION_B),
                capacity: 15,
                passenger_types: vec![],
            },
            Station {
                position: GridPos::new(10, 8),
                station_type: StationType::Terminal,
                name: get_text(&STATION_C, language),
                name_key: Some(&STATION_C),
                capacity: 15,
                passenger_types: vec![],
            },
        ],
        passenger_demands: vec![
            PassengerDemand {
                color: PassengerColor::Red,
                origin: get_text(&STATION_A, language),
                destination: get_text(&STATION_B, language),
                origin_key: Some(&STATION_A),
                destination_key: Some(&STATION_B),
                spawn_rate: 0.3,
                patience: 150.0,
                spawn_time_range: Some((3.0, 20.0)),
                total_count: Some(2),
                spawned_count: 0,
            },
            PassengerDemand {
                color: PassengerColor::Blue,
                origin: get_text(&STATION_A, language),
                destination: get_text(&STATION_B, language),
                origin_key: Some(&STATION_A),
                destination_key: Some(&STATION_C),
                spawn_rate: 0.3,
                patience: 150.0,
                spawn_time_range: Some((8.0, 25.0)),
                total_count: Some(2),
                spawned_count: 0,
            },
        ],
        available_segments: vec![
            AvailableSegment {
                segment_type: RouteSegmentType::Straight,
                count: 12,
                cost: 1,
            },
            AvailableSegment {
                segment_type: RouteSegmentType::Curve,
                count: 6,
                cost: 2,
            },
            AvailableSegment {
                segment_type: RouteSegmentType::TSplit,
                count: 2,
                cost: 3,
            },
        ],
        objectives: vec![
            ObjectiveCondition {
                description: get_text(&OBJECTIVE_CONNECT_ALL, language),
                description_key: Some(&OBJECTIVE_CONNECT_ALL),
                condition_type: ObjectiveType::ConnectAllPassengers,
            },
            ObjectiveCondition {
                description: get_text(&OBJECTIVE_MAX_TRANSFERS, language),
                description_key: Some(&OBJECTIVE_MAX_TRANSFERS),
                condition_type: ObjectiveType::MaxTransfers(2),
            },
        ],
        preset_routes: vec![],
        dynamic_events: vec![],
        scoring: ScoringConfig {
            base_points: 200,
            efficiency_bonus: 100,
            speed_bonus: 50,
            cost_bonus: 50,
        },
    }
}

pub fn create_multiple_routes_level(language: Language) -> LevelData {
    let mut terrain = HashMap::new();

    for x in 0..14 {
        for y in 0..12 {
            terrain.insert(GridPos::new(x, y), TerrainType::Empty);
        }
    }

    // 添加河流障碍
    for y in 4..8 {
        terrain.insert(GridPos::new(6, y), TerrainType::Water);
        terrain.insert(GridPos::new(7, y), TerrainType::Water);
    }

    LevelData {
        id: "level_03_multiple_routes".to_string(),
        name: get_text(&LEVEL_MULTIPLE, language),
        description: get_text(&MULTIPLE_DESCRIPTION, language),
        name_key: Some(&LEVEL_MULTIPLE),
        description_key: Some(&MULTIPLE_DESCRIPTION),
        difficulty: 3,
        grid_size: (14, 12),
        terrain,
        stations: vec![
            Station {
                position: GridPos::new(2, 2),
                station_type: StationType::Terminal,
                name: get_text(&NORTH_STATION, language),
                name_key: Some(&NORTH_STATION),
                capacity: 20,
                passenger_types: vec![PassengerColor::Red, PassengerColor::Green],
            },
            Station {
                position: GridPos::new(2, 9),
                station_type: StationType::Terminal,
                name: get_text(&SOUTH_STATION, language),
                name_key: Some(&SOUTH_STATION),
                capacity: 20,
                passenger_types: vec![PassengerColor::Blue, PassengerColor::Yellow],
            },
            Station {
                position: GridPos::new(11, 2),
                station_type: StationType::Terminal,
                name: get_text(&NORTHEAST_STATION, language),
                name_key: Some(&NORTHEAST_STATION),
                capacity: 20,
                passenger_types: vec![],
            },
            Station {
                position: GridPos::new(11, 9),
                station_type: StationType::Terminal,
                name: get_text(&SOUTHEAST_STATION, language),
                name_key: Some(&SOUTHEAST_STATION),
                capacity: 20,
                passenger_types: vec![],
            },
            Station {
                position: GridPos::new(6, 11),
                station_type: StationType::TransferHub,
                name: get_text(&CENTRAL_HUB, language),
                name_key: Some(&CENTRAL_HUB),
                capacity: 30,
                passenger_types: vec![],
            },
        ],
        passenger_demands: vec![
            PassengerDemand {
                color: PassengerColor::Red,
                origin: get_text(&NORTH_STATION, language),
                destination: get_text(&NORTHEAST_STATION, language),
                origin_key: Some(&NORTH_STATION),
                destination_key: Some(&NORTHEAST_STATION),
                spawn_rate: 0.4,
                patience: 180.0,
                spawn_time_range: Some((5.0, 30.0)),
                total_count: Some(3),
                spawned_count: 0,
            },
            PassengerDemand {
                color: PassengerColor::Blue,
                origin: get_text(&SOUTH_STATION, language),
                destination: get_text(&SOUTHEAST_STATION, language),
                origin_key: Some(&SOUTH_STATION),
                destination_key: Some(&SOUTHEAST_STATION),
                spawn_rate: 0.4,
                patience: 180.0,
                spawn_time_range: Some((8.0, 35.0)),
                total_count: Some(3),
                spawned_count: 0,
            },
            PassengerDemand {
                color: PassengerColor::Green,
                origin: get_text(&NORTH_STATION, language),
                destination: get_text(&SOUTHEAST_STATION, language),
                origin_key: Some(&NORTH_STATION),
                destination_key: Some(&SOUTHEAST_STATION),
                spawn_rate: 0.3,
                patience: 200.0,
                spawn_time_range: Some((10.0, 40.0)),
                total_count: Some(2),
                spawned_count: 0,
            },
            PassengerDemand {
                color: PassengerColor::Yellow,
                origin: get_text(&SOUTH_STATION, language),
                destination: get_text(&NORTHEAST_STATION, language),
                origin_key: Some(&SOUTH_STATION),
                destination_key: Some(&NORTHEAST_STATION),
                spawn_rate: 0.3,
                patience: 200.0,
                spawn_time_range: Some((12.0, 45.0)),
                total_count: Some(2),
                spawned_count: 0,
            },
        ],
        available_segments: vec![
            AvailableSegment {
                segment_type: RouteSegmentType::Straight,
                count: 16,
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
                count: 2,
                cost: 4,
            },
            AvailableSegment {
                segment_type: RouteSegmentType::Bridge,
                count: 2,
                cost: 5,
            },
        ],
        objectives: vec![
            ObjectiveCondition {
                description: "Connect all passengers to destinations".to_string(),
                description_key: Some(&OBJECTIVE_CONNECT_ALL),
                condition_type: ObjectiveType::ConnectAllPassengers,
            },
            ObjectiveCondition {
                description: "Total cost ≤ 35".to_string(),
                description_key: Some(&OBJECTIVE_MAX_COST),
                condition_type: ObjectiveType::MaxCost(35),
            },
            ObjectiveCondition {
                description: "Use at most 20 route segments".to_string(),
                description_key: Some(&OBJECTIVE_MAX_SEGMENTS),
                condition_type: ObjectiveType::MaxSegments(20),
            },
        ],
        preset_routes: vec![],
        dynamic_events: vec![],
        scoring: ScoringConfig {
            base_points: 300,
            efficiency_bonus: 150,
            speed_bonus: 75,
            cost_bonus: 75,
        },
    }
}

pub fn create_time_pressure_level(language: Language) -> LevelData {
    let mut terrain = HashMap::new();

    for x in 0..10 {
        for y in 0..8 {
            terrain.insert(GridPos::new(x, y), TerrainType::Empty);
        }
    }

    // 添加山脉障碍
    terrain.insert(GridPos::new(4, 2), TerrainType::Mountain);
    terrain.insert(GridPos::new(4, 3), TerrainType::Mountain);
    terrain.insert(GridPos::new(5, 2), TerrainType::Mountain);
    terrain.insert(GridPos::new(5, 3), TerrainType::Mountain);

    LevelData {
        id: "level_04_time_pressure".to_string(),
        name: get_text(&OBJECTIVE_TIME_LIMIT, language),
        description: get_text(&TIME_PRESSURE_DESCRIPTION, language),
        name_key: Some(&OBJECTIVE_TIME_LIMIT),
        description_key: Some(&TIME_PRESSURE_DESCRIPTION),
        difficulty: 4,
        grid_size: (10, 8),
        terrain,
        stations: vec![
            Station {
                position: GridPos::new(1, 1),
                station_type: StationType::Terminal,
                name: get_text(&START_STATION, language),
                name_key: Some(&START_STATION),
                capacity: 25,
                passenger_types: vec![
                    PassengerColor::Red,
                    PassengerColor::Blue,
                    PassengerColor::Green,
                ],
            },
            Station {
                position: GridPos::new(8, 1),
                station_type: StationType::Terminal,
                name: get_text(&TARGET_STATION_A, language),
                name_key: Some(&TARGET_STATION_A),
                capacity: 15,
                passenger_types: vec![],
            },
            Station {
                position: GridPos::new(8, 6),
                station_type: StationType::Terminal,
                name: get_text(&TARGET_STATION_B, language),
                name_key: Some(&TARGET_STATION_B),
                capacity: 15,
                passenger_types: vec![],
            },
            Station {
                position: GridPos::new(1, 6),
                station_type: StationType::Terminal,
                name: get_text(&TARGET_STATION_C, language),
                name_key: Some(&TARGET_STATION_C),
                capacity: 15,
                passenger_types: vec![],
            },
        ],
        passenger_demands: vec![
            PassengerDemand {
                color: PassengerColor::Red,
                origin: get_text(&START_STATION, language),
                destination: get_text(&TARGET_STATION_A, language),
                origin_key: Some(&START_STATION),
                destination_key: Some(&TARGET_STATION_A),
                spawn_rate: 0.6,
                patience: 100.0, // 较短的耐心
                spawn_time_range: Some((2.0, 15.0)),
                total_count: Some(4),
                spawned_count: 0,
            },
            PassengerDemand {
                color: PassengerColor::Blue,
                origin: get_text(&START_STATION, language),
                destination: get_text(&TARGET_STATION_B, language),
                origin_key: Some(&START_STATION),
                destination_key: Some(&TARGET_STATION_B),
                spawn_rate: 0.6,
                patience: 100.0,
                spawn_time_range: Some((5.0, 20.0)),
                total_count: Some(4),
                spawned_count: 0,
            },
            PassengerDemand {
                color: PassengerColor::Green,
                origin: get_text(&START_STATION, language),
                destination: get_text(&TARGET_STATION_C, language),
                origin_key: Some(&START_STATION),
                destination_key: Some(&TARGET_STATION_C),
                spawn_rate: 0.6,
                patience: 100.0,
                spawn_time_range: Some((8.0, 25.0)),
                total_count: Some(4),
                spawned_count: 0,
            },
        ],
        available_segments: vec![
            AvailableSegment {
                segment_type: RouteSegmentType::Straight,
                count: 10,
                cost: 1,
            },
            AvailableSegment {
                segment_type: RouteSegmentType::Curve,
                count: 6,
                cost: 2,
            },
            AvailableSegment {
                segment_type: RouteSegmentType::TSplit,
                count: 3,
                cost: 3,
            },
            AvailableSegment {
                segment_type: RouteSegmentType::Tunnel,
                count: 2,
                cost: 6,
            },
        ],
        objectives: vec![
            ObjectiveCondition {
                description: get_text(&OBJECTIVE_CONNECT_ALL, language),
                description_key: Some(&OBJECTIVE_CONNECT_ALL),
                condition_type: ObjectiveType::ConnectAllPassengers,
            },
            ObjectiveCondition {
                description: get_text(&OBJECTIVE_TIME_LIMIT, language),
                description_key: Some(&OBJECTIVE_TIME_LIMIT),
                condition_type: ObjectiveType::TimeLimit(60.0),
            },
            ObjectiveCondition {
                description: get_text(&OBJECTIVE_PASSENGER_SATISFACTION, language),
                description_key: Some(&OBJECTIVE_PASSENGER_SATISFACTION),
                condition_type: ObjectiveType::PassengerSatisfaction(0.8),
            },
        ],
        preset_routes: vec![],
        dynamic_events: vec![],
        scoring: ScoringConfig {
            base_points: 500,
            efficiency_bonus: 200,
            speed_bonus: 150,
            cost_bonus: 100,
        },
    }
}

fn handle_passenger_spawn(
    mut passenger_spawned_event: EventReader<PassengerSpawnedEvent>,
    mut game_state: ResMut<GameState>,
) {
    for _spawned_passenger in passenger_spawned_event.read() {
        // 注意：不要在这里增加计数，因为在 update_passenger_spawning 中已经增加了
        // 只更新总体统计
        game_state.passenger_stats.total_spawned += 1;
    }
}

fn handle_level_events(
    mut level_completed_events: EventReader<LevelCompletedEvent>,
    mut next_state: ResMut<NextState<GameStateEnum>>,
    // level_manager: Res<LevelManager>,
) {
    for event in level_completed_events.read() {
        info!(
            "Level completed! Final score: {}, Time: {:.1}s",
            event.final_score, event.completion_time
        );

        // 计算评级
        let rating = calculate_level_rating(event.final_score, event.completion_time);
        info!("Level rating: {}", rating);

        // 可以在这里保存成绩到本地存储
        // save_level_completion(level_manager.current_level_index, event);

        // 切换到完成界面
        next_state.set(GameStateEnum::LevelComplete);
    }
}

// ============ 辅助函数 ============

fn calculate_level_rating(score: u32, completion_time: f32) -> &'static str {
    if score >= 300 && completion_time <= 60.0 {
        "★★★ Perfect!"
    } else if score >= 200 && completion_time <= 120.0 {
        "★★ Great!"
    } else if score >= 100 {
        "★ Good"
    } else {
        "Complete"
    }
}
