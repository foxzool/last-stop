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
    pub capacity: u32,
    pub passenger_types: Vec<PassengerColor>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PassengerDemand {
    pub color: PassengerColor,
    pub origin: String,
    pub destination: String,
    pub spawn_rate: f32,
    pub patience: f32,
    pub spawn_time_range: Option<(f32, f32)>,
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
    pub segments: Vec<(GridPos, RouteSegmentType, u32)>,
    pub is_removable: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DynamicEvent {
    pub trigger_time: f32,
    pub event_type: EventType,
    pub duration: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EventType {
    SegmentFailure(GridPos),
    SurgePassengers(PassengerColor, f32),
    NewDemand(PassengerDemand),
    StationOverload(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScoringConfig {
    pub base_points: u32,
    pub efficiency_bonus: u32,
    pub speed_bonus: u32,
    pub cost_bonus: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Resource)]
pub struct LevelData {
    pub id: String,
    pub name: String,
    pub description: String,
    pub difficulty: u32,
    pub grid_size: (u32, u32),
    pub terrain: HashMap<GridPos, TerrainType>,
    pub stations: Vec<Station>,
    pub passenger_demands: Vec<PassengerDemand>,
    pub available_segments: Vec<AvailableSegment>,
    pub objectives: Vec<ObjectiveCondition>,
    pub preset_routes: Vec<PresetRoute>,
    pub dynamic_events: Vec<DynamicEvent>,
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
                    update_passenger_spawning.run_if(in_state(GameStateEnum::Playing)),
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
            .map_or(true, |current| current.id != level_data.id)
        {
            level_manager.current_level = Some(level_data.clone());
            info!("同步关卡数据: {}", level_data.name);
        }
    }
}

fn update_passenger_spawning(
    time: Res<Time>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    game_state: Res<GameState>,
) {
    if let Some(level_data) = &game_state.current_level {
        let mut rng = rand::rng();
        let current_time = time.elapsed_secs();

        for demand in level_data.passenger_demands.iter() {
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
            }
        }
    }
}

fn manual_spawn_passenger_debug(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    game_state: Res<GameState>,
) {
    if keyboard_input.just_pressed(KeyCode::F3) {
        if let Some(level_data) = &game_state.current_level {
            if let Some(demand) = level_data.passenger_demands.first() {
                spawn_passenger_no_texture(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    demand,
                    level_data,
                );
                info!("手动生成测试乘客: {:?}", demand.color);
            }
        }
    }
}

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
                    segment_type: segment_type.clone(),
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

pub fn create_tutorial_level() -> LevelData {
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
