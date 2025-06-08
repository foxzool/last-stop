// src/bus_puzzle/passenger_boarding_system.rs - 乘客上下车系统

use crate::bus_puzzle::{
    AgentState, BusPathfindingAgent, BusPathfindingState, BusVehicle, GameStateEnum, LevelManager,
    PathfindingAgent, StationEntity, PASSENGER_Z,
};
use bevy::prelude::*;

// ============ 乘客上车组件 ============

#[derive(Component, Debug)]
pub struct WaitingForBus {
    pub target_station: String,
    pub wait_time: f32,
    pub has_suitable_bus: bool,
}

#[derive(Component, Debug)]
pub struct OnBus {
    pub bus_entity: Entity,
    pub target_station: String,
    pub boarding_time: f32,
}

// ============ 乘客上下车插件 ============

pub struct PassengerBoardingPlugin;

impl Plugin for PassengerBoardingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                update_waiting_passengers,
                handle_passenger_boarding,
                handle_passenger_alighting,
                update_passengers_on_bus,
                debug_passenger_boarding,
            )
                .chain()
                .run_if(in_state(GameStateEnum::Playing)),
        );
    }
}

// ============ 等车乘客管理 ============

fn update_waiting_passengers(
    mut commands: Commands,
    mut passengers: Query<
        (Entity, &mut PathfindingAgent, &mut Transform),
        (Without<WaitingForBus>, Without<OnBus>, Without<BusVehicle>),
    >,
    stations: Query<&StationEntity>,
    buses: Query<(&BusPathfindingAgent, &Transform), (With<BusVehicle>, Without<OnBus>)>,
    level_manager: Res<LevelManager>,
) {
    for (entity, mut agent, mut passenger_transform) in passengers.iter_mut() {
        if agent.state == AgentState::WaitingAtStation {
            // 确保乘客在起点站等车
            if let Some(origin_station) = stations
                .iter()
                .find(|station| station.station_data.name == agent.origin)
            {
                // 获取站点世界坐标
                let (grid_width, grid_height) =
                    if let Some(level_data) = &level_manager.current_level {
                        level_data.grid_size
                    } else {
                        (10, 8)
                    };

                let station_world_pos = origin_station.station_data.position.to_world_pos(
                    level_manager.tile_size,
                    grid_width,
                    grid_height,
                );

                // 将乘客直接移动到起点站（简化处理）
                passenger_transform.translation = station_world_pos + Vec3::Z * PASSENGER_Z;

                // 检查是否有合适的公交车
                let has_suitable_bus = check_suitable_bus(&agent.destination, &buses);

                // 给乘客添加等车组件
                commands.entity(entity).insert(WaitingForBus {
                    target_station: agent.destination.clone(),
                    wait_time: 0.0,
                    has_suitable_bus,
                });

                info!(
                    "乘客 {:?} 在 {} 等车前往 {} (有合适公交车: {})",
                    agent.color, agent.origin, agent.destination, has_suitable_bus
                );
            } else {
                warn!("找不到起点站: {}", agent.origin);
                agent.state = AgentState::GaveUp;
            }
        }
    }
}

/// 检查是否有前往目标站点的公交车
fn check_suitable_bus(
    target_station: &str,
    buses: &Query<(&BusPathfindingAgent, &Transform), (With<BusVehicle>, Without<OnBus>)>,
) -> bool {
    for (bus_agent, _) in buses.iter() {
        if bus_agent
            .stations_to_visit
            .contains(&target_station.to_string())
        {
            return true;
        }
    }
    false
}

// ============ 乘客上车系统 ============

fn handle_passenger_boarding(
    mut commands: Commands,
    mut waiting_passengers: Query<
        (
            Entity,
            &mut WaitingForBus,
            &mut PathfindingAgent,
            &Transform,
        ),
        Without<OnBus>,
    >,
    mut buses: Query<(
        Entity,
        &mut BusVehicle,
        &mut BusPathfindingAgent,
        &Transform,
    )>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();

    for (passenger_entity, mut waiting, mut agent, passenger_transform) in
        waiting_passengers.iter_mut()
    {
        waiting.wait_time += dt;

        // 减缓等车时的耐心消耗
        agent.patience -= dt * 0.02; // 进一步减少耐心消耗

        // 检查附近是否有合适的公交车到站
        for (bus_entity, mut bus_vehicle, bus_agent, bus_transform) in buses.iter_mut() {
            // 检查公交车是否在站点停靠
            if bus_agent.state != BusPathfindingState::AtStation {
                continue;
            }

            // 检查公交车路线是否包含乘客的目的地
            if !bus_agent
                .stations_to_visit
                .contains(&waiting.target_station)
            {
                continue;
            }

            // 检查公交车是否在乘客附近
            let distance = passenger_transform
                .translation
                .distance(bus_transform.translation);
            if distance > 80.0 {
                // 增加上车距离判定
                continue;
            }

            // 检查公交车是否还有座位
            if bus_vehicle.current_passengers.len() >= bus_vehicle.capacity as usize {
                if waiting.wait_time % 5.0 < dt {
                    // 每5秒提示一次，避免日志过多
                    info!(
                        "公交车 {} 已满载 ({}/{}), 乘客 {:?} 继续等待",
                        bus_vehicle.vehicle_id,
                        bus_vehicle.current_passengers.len(),
                        bus_vehicle.capacity,
                        agent.color
                    );
                }
                continue;
            }

            // 乘客上车！
            bus_vehicle.current_passengers.push(passenger_entity);
            agent.state = AgentState::Traveling;

            info!(
                "🚌 乘客 {:?} 上车成功！车辆: {} 目的地: {} 载客: {}/{}",
                agent.color,
                bus_vehicle.vehicle_id,
                waiting.target_station,
                bus_vehicle.current_passengers.len(),
                bus_vehicle.capacity
            );

            // 移除等车组件，添加乘车组件
            commands.entity(passenger_entity).remove::<WaitingForBus>();
            commands.entity(passenger_entity).insert(OnBus {
                bus_entity,
                target_station: waiting.target_station.clone(),
                boarding_time: time.elapsed_secs(),
            });

            break;
        }

        // 检查等车超时
        if agent.patience <= 0.0 {
            warn!(
                "乘客 {:?} 等车超时，耐心耗尽 (等待了 {:.1}s)",
                agent.color, waiting.wait_time
            );
            agent.state = AgentState::GaveUp;
            commands.entity(passenger_entity).remove::<WaitingForBus>();
        }
    }
}

// ============ 乘客下车系统 ============

fn handle_passenger_alighting(
    mut commands: Commands,
    mut passengers_on_bus: Query<
        (Entity, &mut OnBus, &mut PathfindingAgent, &mut Transform),
        Without<BusVehicle>,
    >,
    mut buses: Query<(Entity, &mut BusVehicle, &BusPathfindingAgent, &Transform), With<BusVehicle>>,
    level_manager: Res<LevelManager>,
    stations: Query<&StationEntity>,
) {
    for (passenger_entity, on_bus, mut agent, mut passenger_transform) in
        passengers_on_bus.iter_mut()
    {
        // 找到乘客所在的公交车
        if let Some((_bus_entity, mut bus_vehicle, bus_agent, bus_transform)) = buses
            .iter_mut()
            .find(|(entity, _, _, _)| *entity == on_bus.bus_entity)
        {
            // 乘客位置跟随公交车
            passenger_transform.translation = bus_transform.translation + Vec3::new(0.0, 0.0, 0.1);
            passenger_transform.translation.z = PASSENGER_Z;

            // 检查是否到达目的地站点
            if bus_agent.state == BusPathfindingState::AtStation {
                // 获取当前站点名称
                let current_station_name = &bus_agent.target_station;

                // 如果当前站点是乘客的目的地
                if current_station_name == &on_bus.target_station {
                    // 乘客下车！
                    info!(
                        "🚏 乘客 {:?} 在 {} 下车到达目的地！(乘车时长: {:.1}s)",
                        agent.color,
                        current_station_name,
                        agent.max_patience - agent.patience
                    );

                    // 从公交车乘客列表中移除
                    bus_vehicle
                        .current_passengers
                        .retain(|&id| id != passenger_entity);

                    // 设置乘客位置为站点位置
                    if let Some(station) = stations
                        .iter()
                        .find(|s| s.station_data.name == *current_station_name)
                    {
                        let (grid_width, grid_height) =
                            if let Some(level_data) = &level_manager.current_level {
                                level_data.grid_size
                            } else {
                                (10, 8)
                            };

                        let station_world_pos = station.station_data.position.to_world_pos(
                            level_manager.tile_size,
                            grid_width,
                            grid_height,
                        );

                        passenger_transform.translation = station_world_pos + Vec3::Z * PASSENGER_Z;
                    }

                    // 更新乘客状态
                    agent.state = AgentState::Arrived;

                    // 移除乘车组件
                    commands.entity(passenger_entity).remove::<OnBus>();

                    info!(
                        "公交车 {} 载客更新: {}/{}",
                        bus_vehicle.vehicle_id,
                        bus_vehicle.current_passengers.len(),
                        bus_vehicle.capacity
                    );
                }
            }
        } else {
            // 如果找不到对应的公交车，乘客强制下车
            warn!("乘客 {:?} 的公交车消失了，强制下车", agent.color);
            agent.state = AgentState::GaveUp;
            commands.entity(passenger_entity).remove::<OnBus>();
        }
    }
}

// ============ 乘车乘客更新 ============

fn update_passengers_on_bus(
    passengers_on_bus: Query<(&OnBus, &PathfindingAgent)>,
    time: Res<Time>,
) {
    // 每10秒更新一次乘车统计（避免日志过多）
    if time.elapsed_secs() % 10.0 < 0.1 {
        let total_riding = passengers_on_bus.iter().count();
        if total_riding > 0 {
            trace!("当前乘车乘客数: {}", total_riding);

            // 可以添加更详细的统计
            for (on_bus, agent) in passengers_on_bus.iter() {
                let travel_time = time.elapsed_secs() - on_bus.boarding_time;
                trace!(
                    "乘客 {:?} 乘车 {:.1}s 前往 {}",
                    agent.color,
                    travel_time,
                    on_bus.target_station
                );
            }
        }
    }
}

// ============ 调试系统 ============

fn debug_passenger_boarding(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    waiting_passengers: Query<(&WaitingForBus, &PathfindingAgent)>,
    passengers_on_bus: Query<(&OnBus, &PathfindingAgent)>,
    buses: Query<&BusVehicle>,
    all_passengers: Query<&PathfindingAgent>,
) {
    if keyboard_input.just_pressed(KeyCode::F6) {
        info!("=== 乘客上下车系统调试 ===");

        // 总体乘客统计
        let total_passengers = all_passengers.iter().count();
        let waiting_count = waiting_passengers.iter().count();
        let riding_count = passengers_on_bus.iter().count();
        let arrived_count = all_passengers
            .iter()
            .filter(|agent| matches!(agent.state, AgentState::Arrived))
            .count();
        let gave_up_count = all_passengers
            .iter()
            .filter(|agent| matches!(agent.state, AgentState::GaveUp))
            .count();

        info!("📊 乘客总览:");
        info!("  总乘客数: {}", total_passengers);
        info!("  🚏 等车: {} 人", waiting_count);
        info!("  🚌 乘车: {} 人", riding_count);
        info!("  ✅ 已到达: {} 人", arrived_count);
        info!("  ❌ 已放弃: {} 人", gave_up_count);

        // 等车乘客详情
        if waiting_count > 0 {
            info!("等车乘客详情:");
            for (waiting, agent) in waiting_passengers.iter() {
                info!(
                    "  乘客 {:?}: 等车 {:.1}s 前往 {} (有合适公交车: {})",
                    agent.color,
                    waiting.wait_time,
                    waiting.target_station,
                    waiting.has_suitable_bus
                );
            }
        }

        // 乘车乘客详情
        if riding_count > 0 {
            info!("乘车乘客详情:");
            for (on_bus, agent) in passengers_on_bus.iter() {
                info!(
                    "  乘客 {:?}: 目标 {} (乘车 {:.1}s)",
                    agent.color, on_bus.target_station, on_bus.boarding_time
                );
            }
        }

        // 公交车载客统计
        let bus_count = buses.iter().count();
        info!("🚌 公交车载客情况 ({} 辆):", bus_count);

        if bus_count > 0 {
            for bus in buses.iter() {
                let occupancy_rate = if bus.capacity > 0 {
                    (bus.current_passengers.len() as f32 / bus.capacity as f32) * 100.0
                } else {
                    0.0
                };
                info!(
                    "  {} 载客: {}/{} ({:.1}%)",
                    bus.vehicle_id,
                    bus.current_passengers.len(),
                    bus.capacity,
                    occupancy_rate
                );
            }

            // 系统整体统计
            let total_capacity: u32 = buses.iter().map(|b| b.capacity).sum();
            let total_bus_passengers: usize =
                buses.iter().map(|b| b.current_passengers.len()).sum();

            if total_capacity > 0 {
                let system_occupancy =
                    (total_bus_passengers as f32 / total_capacity as f32) * 100.0;
                info!(
                    "📈 系统载客率: {:.1}% ({}/{})",
                    system_occupancy, total_bus_passengers, total_capacity
                );
            }
        } else {
            info!("  没有运营中的公交车");
        }

        // 成功率统计
        if total_passengers > 0 {
            let success_rate = (arrived_count as f32 / total_passengers as f32) * 100.0;
            let failure_rate = (gave_up_count as f32 / total_passengers as f32) * 100.0;
            info!("🎯 运营效率:");
            info!(
                "  成功率: {:.1}% ({}/{})",
                success_rate, arrived_count, total_passengers
            );
            info!(
                "  失败率: {:.1}% ({}/{})",
                failure_rate, gave_up_count, total_passengers
            );
        }
    }
}
