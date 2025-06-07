// src/bus_puzzle/passenger_test.rs - 简化版本，只保留必要的测试功能

use crate::bus_puzzle::{
    AgentState, GameStateEnum, GridPos, PASSENGER_Z, PassengerColor, PassengerDemand,
    PassengerEntity, PathNode, PathNodeType, PathfindingAgent, get_passenger_color,
};
use bevy::prelude::*;

pub struct PassengerTestPlugin;

impl Plugin for PassengerTestPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                passenger_counter,
                force_spawn_passenger,
            ).run_if(in_state(GameStateEnum::Playing)),
        );
    }
}

// 保留基本的乘客计数器用于调试
fn passenger_counter(
    passengers: Query<(Entity, &PathfindingAgent, &Transform)>,
    mut last_count: Local<usize>,
) {
    let current_count = passengers.iter().count();

    if current_count != *last_count {
        *last_count = current_count;
        info!("乘客数量变化: {}", current_count);
    }
}

// 保留F5快速生成乘客功能用于测试
fn force_spawn_passenger(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    if keyboard_input.just_pressed(KeyCode::F5) {
        let spawn_pos = Vec3::new(-200.0, 0.0, PASSENGER_Z);
        let passenger_color = PassengerColor::Red;
        let bevy_color = get_passenger_color(passenger_color);

        let entity = commands
            .spawn((
                Name::new("Test Passenger"),
                Mesh2d(meshes.add(Circle::new(16.0))),
                MeshMaterial2d(materials.add(bevy_color)),
                Transform::from_translation(spawn_pos),
                PassengerEntity {
                    color: passenger_color,
                    origin: "A站".to_string(),
                    destination: "B站".to_string(),
                    current_patience: 120.0, // 增加耐心值
                    path: Vec::new(),
                },
                PathfindingAgent {
                    color: passenger_color,
                    origin: "A站".to_string(),
                    destination: "B站".to_string(),
                    current_path: Vec::new(),
                    current_step: 0,
                    state: AgentState::WaitingAtStation,
                    patience: 120.0, // 增加耐心值
                    max_patience: 120.0,
                    waiting_time: 0.0,
                },
            ))
            .id();

        info!("F5生成测试乘客 (Entity: {:?})", entity);
    }
}

// 无纹理的乘客生成函数（供其他模块使用）
pub fn spawn_passenger_no_texture(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<ColorMaterial>>,
    demand: &PassengerDemand,
    level_data: &crate::bus_puzzle::LevelData,
) {
    if let Some(origin_station) = level_data.stations.iter().find(|s| s.name == demand.origin) {
        let tile_size = 64.0;
        let (grid_width, grid_height) = level_data.grid_size;
        let world_pos = origin_station
            .position
            .to_world_pos(tile_size, grid_width, grid_height);

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
            "生成乘客: {:?} {} -> {}",
            demand.color, demand.origin, demand.destination
        );
    } else {
        error!("找不到起点站: {}", demand.origin);
    }
}
