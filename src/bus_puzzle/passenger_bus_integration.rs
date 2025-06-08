// src/bus_puzzle/passenger_bus_integration.rs - 乘客与公交车交互系统

use crate::bus_puzzle::{
    AgentState, BusRoutesManager, BusState, BusVehicle, LevelManager, PathfindingAgent,
    DEFAULT_PASSENGER_PATIENCE, PASSENGER_Z,
};
use bevy::prelude::*;

// ============ 乘客候车组件 ============

#[derive(Component, Debug)]
pub struct WaitingForBus {
    pub target_route_id: String,
    pub target_destination: String,
    pub wait_time: f32,
    pub max_wait_time: f32,
}

#[derive(Component, Debug)]
pub struct OnBus {
    pub vehicle_id: String,
    pub target_stop: String,
    pub boarding_time: f32,
}

// ============ 乘客状态更新系统 ============

pub fn update_passenger_bus_interaction(
    mut commands: Commands,
    mut passengers: Query<
        (Entity, &mut PathfindingAgent, &Transform),
        (Without<OnBus>, Without<WaitingForBus>),
    >,
    mut waiting_passengers: Query<
        (
            Entity,
            &mut WaitingForBus,
            &mut PathfindingAgent,
            &Transform,
        ),
        Without<OnBus>,
    >,
    mut buses: Query<(&mut BusVehicle, &Transform), With<BusVehicle>>,
    bus_routes_manager: Res<BusRoutesManager>,
    time: Res<Time>,
    // level_manager: Res<LevelManager>,
) {
    let dt = time.delta_secs();

    // 第一步：将寻路状态的乘客转换为等车状态
    for (entity, mut agent, _transform) in passengers.iter_mut() {
        // 只处理刚开始等待的乘客
        if matches!(agent.state, AgentState::WaitingAtStation) {
            // 找到适合的路线
            if let Some(route_id) =
                find_suitable_route(&agent.origin, &agent.destination, &bus_routes_manager)
            {
                info!("乘客 {:?} 开始等车，路线: {}", agent.color, route_id);

                // 清空乘客的寻路路径
                agent.current_path.clear();
                agent.current_step = 0;

                // 添加等车组件
                commands.entity(entity).insert(WaitingForBus {
                    target_route_id: route_id,
                    target_destination: agent.destination.clone(),
                    wait_time: 0.0,
                    max_wait_time: DEFAULT_PASSENGER_PATIENCE,
                });
            } else {
                // s
                // agent.state = AgentState::GaveUp;
            }
        }
    }

    // 第二步：更新等车乘客
    for (entity, mut waiting, mut agent, passenger_transform) in waiting_passengers.iter_mut() {
        waiting.wait_time += dt;

        agent.patience -= dt * 0.5; // 等车时耐心消耗稍快
        info!("agent patience {}", agent.patience);

        // 检查是否等车超时
        if waiting.wait_time > waiting.max_wait_time || agent.patience <= 0.0 {
            warn!("乘客 {:?} 等车超时或耐心耗尽", agent.color);
            agent.state = AgentState::GaveUp;
            commands.entity(entity).remove::<WaitingForBus>();
            continue;
        }

        // 检查附近是否有合适的公交车到站
        for (mut bus, bus_transform) in buses.iter_mut() {
            if bus.route_id == waiting.target_route_id && matches!(bus.state, BusState::AtStop) {
                let distance = passenger_transform
                    .translation
                    .distance(bus_transform.translation);
                if distance < 64.0 {
                    // 在合理范围内
                    // 检查公交车是否还有座位
                    if bus.current_passengers.len() < bus.capacity as usize {
                        // 乘客上车
                        bus.current_passengers.push(entity);
                        agent.state = AgentState::Traveling; // 改为乘车状态

                        info!("乘客 {:?} 上车，车辆: {}", agent.color, bus.vehicle_id);

                        // 移除等车组件，添加乘车组件
                        commands.entity(entity).remove::<WaitingForBus>();
                        commands.entity(entity).insert(OnBus {
                            vehicle_id: bus.vehicle_id.clone(),
                            target_stop: waiting.target_destination.clone(),
                            boarding_time: time.elapsed_secs(),
                        });
                        break;
                    } else {
                        trace!(
                            "公交车 {} 已满载，乘客 {:?} 继续等待",
                            bus.vehicle_id,
                            agent.color
                        );
                    }
                }
            }
        }
    }
}

// ============ 乘车乘客更新系统 ============

pub fn update_passengers_on_bus(
    mut commands: Commands,
    mut passengers_on_bus: Query<
        (Entity, &mut OnBus, &mut PathfindingAgent, &mut Transform),
        Without<BusVehicle>,
    >,
    mut buses: Query<(&mut BusVehicle, &Transform), With<BusVehicle>>,
    bus_routes_manager: Res<BusRoutesManager>,
    // stations: Query<&StationEntity>,
    level_manager: Res<LevelManager>,
) {
    for (passenger_entity, on_bus, mut agent, mut passenger_transform) in
        passengers_on_bus.iter_mut()
    {
        // 找到乘客所在的公交车
        if let Some((mut bus, bus_transform)) = buses
            .iter_mut()
            .find(|(bus, _)| bus.vehicle_id == on_bus.vehicle_id)
        {
            // 乘客位置跟随公交车
            passenger_transform.translation = bus_transform.translation + Vec3::Z * 0.1;
            passenger_transform.translation.z = PASSENGER_Z;

            // 检查是否到达目的地站点
            if let Some(route) = bus_routes_manager.get_route(&bus.route_id) {
                if bus.current_stop_index < route.stops.len() {
                    let current_stop = &route.stops[bus.current_stop_index];

                    // 如果当前站点是乘客的目的地且公交车在站点停靠
                    if current_stop.name == on_bus.target_stop
                        && matches!(bus.state, BusState::AtStop)
                    {
                        // 乘客下车
                        info!("乘客 {:?} 在 {} 下车", agent.color, current_stop.name);

                        // 从公交车乘客列表中移除
                        bus.current_passengers.retain(|&id| id != passenger_entity);

                        // 设置乘客位置为站点位置
                        let (grid_width, grid_height) =
                            if let Some(level_data) = &level_manager.current_level {
                                level_data.grid_size
                            } else {
                                (10, 8)
                            };

                        let station_world_pos = current_stop.position.to_world_pos(
                            level_manager.tile_size,
                            grid_width,
                            grid_height,
                        );

                        passenger_transform.translation = station_world_pos + Vec3::Z * PASSENGER_Z;

                        // 更新乘客状态
                        agent.state = AgentState::Arrived;

                        // 移除乘车组件
                        commands.entity(passenger_entity).remove::<OnBus>();
                    }
                }
            }
        } else {
            // 如果找不到对应的公交车，乘客下车（公交车可能被删除了）
            warn!(
                "乘客 {:?} 的公交车 {} 消失了",
                agent.color, on_bus.vehicle_id
            );
            agent.state = AgentState::GaveUp;
            commands.entity(passenger_entity).remove::<OnBus>();
        }
    }
}

// ============ 辅助函数 ============

/// 为乘客找到合适的路线
fn find_suitable_route(
    origin: &str,
    destination: &str,
    bus_routes_manager: &BusRoutesManager,
) -> Option<String> {
    for (route_id, route) in &bus_routes_manager.routes {
        // 检查路线是否包含起点和终点
        let has_origin = route.stops.iter().any(|stop| stop.name == origin);
        let has_destination = route.stops.iter().any(|stop| stop.name == destination);

        if has_origin && has_destination {
            // 确保起点在终点之前（简化版本，不考虑环线）
            let origin_index = route.stops.iter().position(|stop| stop.name == origin);
            let dest_index = route.stops.iter().position(|stop| stop.name == destination);

            if let (Some(origin_idx), Some(dest_idx)) = (origin_index, dest_index) {
                if origin_idx != dest_idx {
                    info!(
                        "为 {} -> {} 找到路线: {} (站点顺序: {} -> {})",
                        origin, destination, route.route_name, origin_idx, dest_idx
                    );
                    return Some(route_id.clone());
                }
            }
        }
    }
    None
}

// ============ 禁用原寻路系统 ============

/// 禁用乘客的自主寻路，改为等车模式
pub fn disable_passenger_pathfinding(
    mut passengers: Query<&mut PathfindingAgent, Added<PathfindingAgent>>,
) {
    for mut agent in passengers.iter_mut() {
        // 清空寻路路径，让乘客在起点等车
        agent.current_path.clear();
        agent.current_step = 0;
        agent.state = AgentState::WaitingAtStation;

        info!(
            "乘客 {:?} 已切换到等车模式: {} -> {}",
            agent.color, agent.origin, agent.destination
        );
    }
}

// ============ 调试系统 ============

pub fn debug_passenger_bus_system(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    waiting_passengers: Query<(&WaitingForBus, &PathfindingAgent)>,
    passengers_on_bus: Query<(&OnBus, &PathfindingAgent)>,
    buses: Query<&BusVehicle>,
) {
    if keyboard_input.just_pressed(KeyCode::F6) {
        info!("=== 乘客-公交车交互调试 ===");

        info!("等车乘客数: {}", waiting_passengers.iter().count());
        for (waiting, agent) in waiting_passengers.iter() {
            info!(
                "  乘客 {:?}: 等车 {:.1}s, 目标路线: {}, 目的地: {}",
                agent.color, waiting.wait_time, waiting.target_route_id, waiting.target_destination
            );
        }

        info!("乘车乘客数: {}", passengers_on_bus.iter().count());
        for (on_bus, agent) in passengers_on_bus.iter() {
            info!(
                "  乘客 {:?}: 乘坐 {}, 目的地: {}, 乘车时长: {:.1}s",
                agent.color, on_bus.vehicle_id, on_bus.target_stop, on_bus.boarding_time
            );
        }

        info!("公交车载客情况:");
        for bus in buses.iter() {
            info!(
                "  公交车 {}: {}/{} 乘客",
                bus.vehicle_id,
                bus.current_passengers.len(),
                bus.capacity
            );
        }
    }
}
