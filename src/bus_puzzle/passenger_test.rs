// src/bus_puzzle/passenger_test.rs - 完整版本

use crate::bus_puzzle::{
    AgentState, GameStateEnum, GridPos, PASSENGER_Z, PassengerColor, PassengerDemand,
    PassengerEntity, PathNode, PathNodeType, PathfindingAgent, STATION_Z, TERRAIN_Z,
    get_passenger_color,
};
use bevy::prelude::*;
use rand::Rng;

pub struct PassengerTestPlugin;

impl Plugin for PassengerTestPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                simple_passenger_spawner.run_if(in_state(GameStateEnum::Playing)),
                passenger_counter,
                force_spawn_passenger,
                quick_movement_test, // 添加移动测试
            ),
        );
    }
}

// 简单的乘客生成器 - 每3秒强制生成一个乘客（不使用纹理）
fn simple_passenger_spawner(
    time: Res<Time>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut last_spawn: Local<f32>,
) {
    let current_time = time.elapsed_secs();

    if current_time - *last_spawn > 3.0 {
        *last_spawn = current_time;

        // 使用配置文件中的 PASSENGER_Z 常量
        let spawn_pos = Vec3::new(-200.0, 0.0, PASSENGER_Z);

        let entity = commands
            .spawn((
                Name::new("Test Passenger"),
                Mesh2d(meshes.add(Circle::new(16.0))),
                MeshMaterial2d(materials.add(Color::srgb(1.0, 0.2, 0.2))),
                Transform::from_translation(spawn_pos),
                PassengerEntity {
                    color: PassengerColor::Red,
                    origin: "A站".to_string(),
                    destination: "B站".to_string(),
                    current_patience: 60.0,
                    path: Vec::new(),
                },
                PathfindingAgent {
                    color: PassengerColor::Red,
                    origin: "A站".to_string(),
                    destination: "B站".to_string(),
                    current_path: Vec::new(),
                    current_step: 0,
                    state: AgentState::WaitingAtStation,
                    patience: 60.0,
                    max_patience: 60.0,
                    waiting_time: 0.0,
                },
            ))
            .id();

        info!(
            "生成测试乘客 (Entity: {:?}) 位置: {:?} (层级: PASSENGER_Z={:.1})",
            entity, spawn_pos, PASSENGER_Z
        );
    }
}

// 乘客计数器
fn passenger_counter(
    passengers: Query<(Entity, &PathfindingAgent, &Transform)>,
    mut last_count: Local<usize>,
    time: Res<Time>,
    mut last_log: Local<f32>,
) {
    let current_count = passengers.iter().count();
    let current_time = time.elapsed_secs();

    if current_count != *last_count {
        *last_count = current_count;
        info!("乘客数量变化: {}", current_count);

        for (entity, agent, transform) in passengers.iter() {
            let z_layer = if transform.translation.z == PASSENGER_Z {
                "PASSENGER"
            } else if transform.translation.z == STATION_Z {
                "STATION"
            } else if transform.translation.z == TERRAIN_Z {
                "TERRAIN"
            } else {
                "UNKNOWN"
            };

            info!(
                "  乘客 {:?}: {:?} 状态: {:?} 层级: {} (Z={:.1})",
                entity, agent.color, agent.state, z_layer, transform.translation.z
            );
        }
    }

    if current_time - *last_log > 5.0 {
        *last_log = current_time;
        info!("当前乘客总数: {}", current_count);
    }
}

// 按F5强制生成乘客（不使用纹理）
fn force_spawn_passenger(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    if keyboard_input.just_pressed(KeyCode::F5) {
        let mut rng = rand::thread_rng();
        let spawn_pos = Vec3::new(
            rng.gen_range(-300.0..300.0),
            rng.gen_range(-200.0..200.0),
            PASSENGER_Z, // 使用配置常量
        );

        let colors = [
            PassengerColor::Red,
            PassengerColor::Blue,
            PassengerColor::Green,
            PassengerColor::Yellow,
            PassengerColor::Purple,
            PassengerColor::Orange,
        ];

        let passenger_color = colors[rng.gen_range(0..colors.len())];
        let bevy_color = get_passenger_color(passenger_color);

        let entity = commands
            .spawn((
                Name::new(format!("Force Spawned Passenger {:?}", passenger_color)),
                Mesh2d(meshes.add(Circle::new(16.0))),
                MeshMaterial2d(materials.add(bevy_color)),
                Transform::from_translation(spawn_pos),
                PassengerEntity {
                    color: passenger_color,
                    origin: "测试起点".to_string(),
                    destination: "测试终点".to_string(),
                    current_patience: 60.0,
                    path: Vec::new(),
                },
                PathfindingAgent {
                    color: passenger_color,
                    origin: "测试起点".to_string(),
                    destination: "测试终点".to_string(),
                    current_path: Vec::new(),
                    current_step: 0,
                    state: AgentState::WaitingAtStation,
                    patience: 60.0,
                    max_patience: 60.0,
                    waiting_time: 0.0,
                },
            ))
            .id();

        info!(
            "F5强制生成乘客 (Entity: {:?}): {:?} 位置: {:?} 层级: PASSENGER_Z={:.1}",
            entity, passenger_color, spawn_pos, PASSENGER_Z
        );
    }
}

// 快速移动测试 - 让乘客立即开始移动
fn quick_movement_test(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut passengers: Query<(&mut PathfindingAgent, &mut Transform)>,
    time: Res<Time>,
) {
    if keyboard_input.just_pressed(KeyCode::Space) {
        info!("=== 开始快速移动测试 ===");

        for (mut agent, mut transform) in passengers.iter_mut() {
            agent.state = AgentState::Traveling;
            agent.waiting_time = 0.0;

            // 确保乘客在正确的层级
            transform.translation.z = PASSENGER_Z;

            let current_world_pos = transform.translation;
            let target_world_pos = current_world_pos + Vec3::new(300.0, 0.0, 0.0);

            agent.current_path = vec![
                PathNode {
                    position: GridPos::new(0, 0),
                    node_type: PathNodeType::Station("起点".to_string()),
                    estimated_wait_time: 0.0,
                    route_id: None,
                },
                PathNode {
                    position: GridPos::new(5, 0),
                    node_type: PathNodeType::Station("终点".to_string()),
                    estimated_wait_time: 0.0,
                    route_id: None,
                },
            ];
            agent.current_step = 0;

            info!(
                "设置乘客 {:?} 移动测试，层级: PASSENGER_Z={:.1}",
                agent.color, PASSENGER_Z
            );
        }
    }

    if keyboard_input.pressed(KeyCode::Space) {
        let dt = time.delta_secs();
        let speed = 200.0;

        for (mut agent, mut transform) in passengers.iter_mut() {
            if matches!(agent.state, AgentState::Traveling) {
                transform.translation.x += speed * dt;
                transform.translation.z = PASSENGER_Z; // 确保始终在正确层级

                agent.waiting_time += dt;
                if agent.waiting_time.fract() < dt {
                    info!(
                        "乘客 {:?} 移动中，层级: PASSENGER (Z={:.1})",
                        agent.color, transform.translation.z
                    );
                }

                if agent.waiting_time > 3.0 {
                    agent.state = AgentState::Arrived;
                    info!("乘客 {:?} 完成移动测试", agent.color);
                }
            }
        }
    }
}

// 无纹理的乘客生成函数（供其他模块使用）
pub fn spawn_passenger_no_texture(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<ColorMaterial>>,
    demand: &crate::bus_puzzle::PassengerDemand,
    level_data: &crate::bus_puzzle::LevelData,
) {
    if let Some(origin_station) = level_data.stations.iter().find(|s| s.name == demand.origin) {
        let tile_size = 64.0;
        let (grid_width, grid_height) = level_data.grid_size;
        let world_pos = origin_station
            .position
            .to_world_pos(tile_size, grid_width, grid_height);

        // 使用配置文件中的 PASSENGER_Z 常量
        let passenger_world_pos = Vec3::new(world_pos.x, world_pos.y, PASSENGER_Z);
        let bevy_color = get_passenger_color(demand.color);

        let entity = commands
            .spawn((
                Name::new(format!(
                    "Passenger {:?} {} -> {}",
                    demand.color, demand.origin, demand.destination
                )),
                Mesh2d(meshes.add(Circle::new(16.0))),
                MeshMaterial2d(materials.add(bevy_color)),
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
            "生成乘客 (Entity: {:?}): {:?} {} -> {} 层级: PASSENGER_Z={:.1}",
            entity, demand.color, demand.origin, demand.destination, PASSENGER_Z
        );
    } else {
        error!("找不到起点站: {}", demand.origin);
    }
}
