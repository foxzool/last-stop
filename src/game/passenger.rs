// 乘客系统实现
use crate::game::{
    grid::{GridPosition, GridState},
    validation::can_segments_connect,
};
use bevy::{color::palettes::basic, prelude::*};
use rand::Rng;
use std::collections::VecDeque;

// 乘客插件
pub struct PassengerPlugin;

impl Plugin for PassengerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PassengerSpawnTimer>()
            .init_resource::<PassengerManager>()
            .init_resource::<PathReplannigTimer>()
            .add_systems(
                Update,
                (
                    spawn_passengers,
                    update_passengers,
                    remove_impatient_passengers,
                    replan_passenger_paths,
                ),
            );
    }
}

// 乘客生成计时器
#[derive(Resource)]
pub struct PassengerSpawnTimer {
    pub timer: Timer,
}

impl Default for PassengerSpawnTimer {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(5.0, TimerMode::Repeating),
        }
    }
}

// 路径重新规划计时器
#[derive(Resource)]
pub struct PathReplannigTimer {
    pub timer: Timer,
}

impl Default for PathReplannigTimer {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(1.0, TimerMode::Repeating),
        }
    }
}

// 乘客目的地类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Destination {
    Red,
    Blue,
    Green,
    Yellow,
}

impl Destination {
    // 获取目的地对应的颜色
    pub fn get_color(&self) -> Color {
        match self {
            Destination::Red => Color::from(basic::RED),
            Destination::Blue => Color::from(basic::BLUE),
            Destination::Green => Color::from(basic::GREEN),
            Destination::Yellow => Color::from(basic::YELLOW),
        }
    }

    // 随机生成目的地
    pub fn random() -> Self {
        let random = rand::random::<usize>() % 4;
        match random {
            0 => Destination::Red,
            1 => Destination::Blue,
            2 => Destination::Green,
            _ => Destination::Yellow,
        }
    }
}

// 乘客组件
#[derive(Component, Debug)]
pub struct Passenger {
    pub destination: Destination,       // 目的地类型
    pub patience: f32,                  // 耐心值 (0.0-100.0)
    pub path: VecDeque<GridPosition>,   // 规划的路径
    pub current_position: GridPosition, // 当前位置
    pub target_position: GridPosition,  // 目标位置
    pub progress: f32,                  // 移动进度 (0.0-1.0)
    pub speed: f32,                     // 移动速度
    pub arrived: bool,                  // 是否已到达目的地
}

impl Passenger {
    pub fn new(start: GridPosition, destination: Destination) -> Self {
        Self {
            destination,
            patience: 100.0,
            path: VecDeque::new(),
            current_position: start,
            target_position: start, // 初始目标位置与当前位置相同
            progress: 0.0,
            speed: 0.5, // 每秒移动0.5个格子
            arrived: false,
        }
    }

    // 更新乘客位置
    pub fn update_position(&mut self, delta_time: f32) -> bool {
        // 如果已到达目的地或没有路径，则不移动
        if self.arrived || self.path.is_empty() {
            return false;
        }

        // 更新移动进度
        self.progress += self.speed * delta_time;

        // 如果完成了当前格子的移动
        if self.progress >= 1.0 {
            // 移动到下一个格子
            self.current_position = self.target_position;
            self.progress = 0.0;

            // 获取下一个目标位置
            if let Some(next_pos) = self.path.pop_front() {
                self.target_position = next_pos;
                true
            } else {
                // 路径已完成
                self.arrived = true;
                false
            }
        } else {
            true
        }
    }

    // 减少耐心值
    pub fn decrease_patience(&mut self, amount: f32) {
        self.patience -= amount;
        self.patience = self.patience.max(0.0);
    }

    // 检查是否失去耐心
    pub fn is_impatient(&self) -> bool {
        self.patience <= 0.0
    }

    // 设置路径
    pub fn set_path(&mut self, path: VecDeque<GridPosition>) {
        self.path = path;
        if !self.path.is_empty() { // 确保路径不为空
            // 从路径的第一个点开始
            self.target_position = *self.path.front().unwrap(); // unwrap是安全的，因为我们检查了is_empty
            self.arrived = false; // 重置到达状态，准备开始新的路径
            self.progress = 0.0; // 重置移动进度
        } else {
            // 如果路径为空，标记为已到达 (或处理错误)
            self.arrived = true;
            // 可以考虑在这里打日志，因为通常不应该给乘客设置一个空路径
            warn!("Attempted to set an empty path for a passenger.");
        }
    }

    // 获取目的地位置
    pub fn get_destination_position(&self) -> Option<GridPosition> {
        if !self.path.is_empty() {
            return Some(*self.path.back().unwrap());
        }
        None
    }
}

// 乘客管理器
#[derive(Resource, Default)]
pub struct PassengerManager {
    pub passengers: Vec<Entity>,
    pub stations: Vec<(GridPosition, Vec<Destination>)>, // 车站位置和可到达的目的地
}

impl PassengerManager {
    // 添加乘客
    pub fn add_passenger(&mut self, entity: Entity) {
        self.passengers.push(entity);
    }

    // 移除乘客
    pub fn remove_passenger(&mut self, entity: Entity) {
        if let Some(index) = self.passengers.iter().position(|&p| p == entity) {
            self.passengers.swap_remove(index);
        }
    }

    // 添加车站
    pub fn add_station(&mut self, position: GridPosition, destinations: Vec<Destination>) {
        self.stations.push((position, destinations));
    }

    // 获取随机起点站
    pub fn get_random_start_station(&self) -> Option<GridPosition> {
        if self.stations.is_empty() {
            return None;
        }
       


        let index = rand::thread_rng().gen_range(0..self.stations.len());
        Some(self.stations[index].0)
    }

    // 为指定目的地找到合适的终点站
    pub fn find_destination_station(
        &self,
        destination: Destination,
        current_pos_to_avoid: Option<GridPosition>,
    ) -> Option<GridPosition> {
        let all_matching_stations: Vec<GridPosition> = self
            .stations
            .iter()
            .filter(|(_, dests)| dests.contains(&destination))
            .map(|(pos, _)| *pos)
            .collect();

        if all_matching_stations.is_empty() {
            return None; // No station serves this destination type
        }

        let final_candidate_list = if let Some(avoid_pos) = current_pos_to_avoid {
            let preferred_options: Vec<GridPosition> = all_matching_stations
                .iter()
                .filter(|&&p| p != avoid_pos)
                .cloned()
                .collect();
            if !preferred_options.is_empty() {
                preferred_options
            } else {
                all_matching_stations // Fallback: current pos is the only option, or no other options
            }
        } else {
            all_matching_stations // No position to avoid specified
        };

        if final_candidate_list.is_empty() {
            // This should ideally not happen if all_matching_stations was not empty.
            // However, to be safe, one might return None or log an error.
            return None;
        }

        let index = rand::thread_rng().gen_range(0..final_candidate_list.len());
        Some(final_candidate_list[index])
    }

    // 寻找从起点到终点的最短路径
    pub fn find_path(
        &self,
        start: GridPosition,
        end: GridPosition,
        grid_state: &GridState,
    ) -> Option<VecDeque<GridPosition>> {
        info!(
            "开始寻找路径: 从 ({}, {}) 到 ({}, {})",
            start.x, start.y, end.x, end.y
        );

        // 记录网格状态
        info!(
            "当前网格中的路线段数量: {}",
            grid_state.route_segments.len()
        );

        // 使用A*算法寻找最短路径
        let mut open_set = Vec::new();
        let mut came_from = std::collections::HashMap::new();
        let mut g_score = std::collections::HashMap::new();
        let mut f_score = std::collections::HashMap::new();

        open_set.push(start);
        g_score.insert(start, 0);
        f_score.insert(start, start.distance_to(&end));

        while !open_set.is_empty() {
            // 找到f_score最小的节点
            let current = *open_set
                .iter()
                .min_by_key(|&&pos| f_score.get(&pos).unwrap_or(&i32::MAX))
                .unwrap();

            info!(
                "当前节点: ({}, {}), 开放集大小: {}",
                current.x,
                current.y,
                open_set.len()
            );

            // 如果到达终点
            if current == end {
                // 重建路径
                let mut path = VecDeque::new();
                let mut current = end;
                while current != start {
                    path.push_front(current);
                    current = came_from[&current];
                }
                return Some(path);
            }

            // 从开放集中移除当前节点
            open_set.retain(|&pos| pos != current);

            // 检查相邻节点
            for neighbor in current.adjacent().iter() {
                // 检查是否有路线连接当前节点和邻居节点
                if !self.is_connected(&current, neighbor, grid_state) {
                    continue;
                }

                // 计算通过当前节点到达邻居节点的g_score
                let tentative_g_score = g_score.get(&current).unwrap_or(&i32::MAX) + 1;

                // 如果找到了更好的路径
                if tentative_g_score < *g_score.get(neighbor).unwrap_or(&i32::MAX) {
                    came_from.insert(*neighbor, current);
                    g_score.insert(*neighbor, tentative_g_score);
                    f_score.insert(*neighbor, tentative_g_score + neighbor.distance_to(&end));

                    // 如果邻居节点不在开放集中，添加它
                    if !open_set.contains(neighbor) {
                        open_set.push(*neighbor);
                    }
                }
            }
        }

        // 没有找到路径
        None
    }

    // 检查两个相邻格子之间是否有路线连接
    fn is_connected(
        &self,
        pos1: &GridPosition,
        pos2: &GridPosition,
        grid_state: &GridState,
    ) -> bool {
        // 获取两个位置的路线段
        let segment1 = match grid_state.get_route_segment(*pos1) {
            Some(s) => s,
            None => {
                trace!("is_connected: No segment at pos1: {:?}", pos1);
                return false;
            }
        };

        let segment2 = match grid_state.get_route_segment(*pos2) {
            Some(s) => s,
            None => {
                trace!("is_connected: No segment at pos2: {:?}", pos2);
                return false;
            }
        };

        trace!(
            "is_connected: Checking connection between {:?} ({:?}/{:?}) and {:?} ({:?}/{:?})",
            pos1,
            segment1.segment_type,
            segment1.direction,
            pos2,
            segment2.segment_type,
            segment2.direction
        );

        // 使用 validation 模块中的 can_segments_connect 方法检查连接
        // 创建一个临时的 ConnectionMap，因为我们只需要检查连接性，不需要保存状态
        let result = can_segments_connect(*pos1, segment1, *pos2, segment2, &Default::default());
        trace!(
            "is_connected: Result for {:?} and {:?} is {}",
            pos1, pos2, result
        );
        result
    }
}

// 生成乘客系统
fn spawn_passengers(
    mut commands: Commands,
    time: Res<Time>,
    mut spawn_timer: ResMut<PassengerSpawnTimer>,
    mut passenger_manager: ResMut<PassengerManager>,
    grid_state: Res<GridState>,
    asset_server: Res<AssetServer>,
) {
    // 更新计时器
    spawn_timer.timer.tick(time.delta());

    // 如果计时器完成，生成新乘客
    if spawn_timer.timer.just_finished() {
        info!(
            "尝试生成新乘客，当前已有乘客数量: {}",
            passenger_manager.passengers.len()
        );
        info!("可用车站数量: {}", passenger_manager.stations.len());

        // 获取随机起点站
        if let Some(start_pos) = passenger_manager.get_random_start_station() {
            info!("选择起点站: ({}, {})", start_pos.x, start_pos.y);

            // 随机选择目的地类型
            // let destination = Destination::random();
            let destination = Destination::Yellow;
            info!("随机选择目的地类型: {:?}", destination);

            // 寻找对应目的地类型的终点站
            // Pass start_pos as the position to avoid for the initial destination
            if let Some(end_pos) = passenger_manager.find_destination_station(destination, Some(start_pos)) {
                info!("找到终点站: ({}, {})", end_pos.x, end_pos.y);

                // 创建乘客
                let passenger = Passenger::new(start_pos, destination);

                let passenger_entity = commands
                    .spawn((
                        passenger,
                        Sprite {
                            image: asset_server.load("sprites/passenger.png"),
                            color: destination.get_color(),
                            custom_size: Some(Vec2::new(16.0, 16.0)),
                            ..default()
                        },
                        Transform::from_translation(Vec3::new(0.0, 0.0, 1.0)),
                        Name::new(format!("{:?} Passenger", destination)),
                    ))
                    .id();

                // 添加到乘客管理器
                passenger_manager.add_passenger(passenger_entity);
                info!(
                    "成功生成 {:?} 乘客，实体ID: {:?}",
                    destination, passenger_entity
                );
            } else {
                warn!("无法为目的地类型 {:?} 找到终点站", destination);
            }
        } else {
            warn!("没有可用的起点站");
        }
    }
}

// 更新乘客位置和状态
fn update_passengers(
    time: Res<Time>,
    mut query: Query<(&mut Passenger, &mut Transform)>,
    grid_config: Res<crate::game::grid::GridConfig>,
) {
    for (mut passenger, mut transform) in query.iter_mut() {
        // 如果已到达目的地，不再更新
        if passenger.arrived {
            continue;
        }

        // 更新乘客位置
        let is_moving = passenger.update_position(time.delta_secs());

        if is_moving {
            // 如果乘客正在移动，更新其世界坐标
            let current_world_pos = grid_config.grid_to_world(passenger.current_position);
            let target_world_pos = grid_config.grid_to_world(passenger.target_position);

            // 插值计算当前位置
            let world_pos = current_world_pos.lerp(target_world_pos, passenger.progress);
            transform.translation.x = world_pos.x;
            transform.translation.y = world_pos.y;

            // 移动时减少一点耐心值
            passenger.decrease_patience(time.delta_secs() * 1.0);
        } else if !passenger.arrived {
            // 如果没有移动且未到达目的地，减少更多耐心值
            passenger.decrease_patience(time.delta_secs() * 5.0);
        }
    }
}

// 移除失去耐心的乘客
fn remove_impatient_passengers(
    mut commands: Commands,
    query: Query<(Entity, &Passenger)>,
    mut passenger_manager: ResMut<PassengerManager>,
) {
    for (entity, passenger) in query.iter() {
        if passenger.is_impatient() {
            // 从世界中移除乘客实体
            commands.entity(entity).despawn();

            // 从乘客管理器中移除
            passenger_manager.remove_passenger(entity);
        }
    }
}

// 定期重新规划乘客路径系统
fn replan_passenger_paths(
    time: Res<Time>,
    mut replan_timer: ResMut<PathReplannigTimer>,
    mut passenger_manager: ResMut<PassengerManager>,
    grid_state: Res<GridState>,
    mut query: Query<(Entity, &mut Passenger)>,
) {
    // 更新计时器
    replan_timer.timer.tick(time.delta());

    // 如果计时器完成，为每个乘客重新规划路径
    if replan_timer.timer.just_finished() {
        info!("开始为所有乘客重新规划路径");

        let mut replan_count = 0;
        let mut success_count = 0;

        for (entity, mut passenger) in query.iter_mut() {
            // 跳过已经到达目的地的乘客
            if passenger.arrived {
                continue;
            }

            // 获取乘客当前位置和目的地类型
            let current_pos = passenger.current_position;
            let destination_type = passenger.destination;

            // 寻找对应目的地类型的终点站
            // Pass passenger's current_pos as the position to avoid
            if let Some(end_pos) = passenger_manager.find_destination_station(destination_type, Some(current_pos)) {
                replan_count += 1;

                // 寻找从当前位置到终点的路径
                let path_result = passenger_manager.find_path(current_pos, end_pos, &grid_state);

                if let Some(path) = path_result {
                    info!(
                        "为乘客 {:?} 重新规划路径，从 ({}, {}) 到 ({}, {}), 路径长度: {}",
                        entity,
                        current_pos.x,
                        current_pos.y,
                        end_pos.x,
                        end_pos.y,
                        path.len()
                    );
                    passenger.set_path(path);
                    success_count += 1;
                } else {
                    warn!(
                        "无法为乘客 {:?} 重新规划从 ({}, {}) 到 ({}, {}) 的路径",
                        entity, current_pos.x, current_pos.y, end_pos.x, end_pos.y
                    );
                }
            }
        }

        info!(
            "路径重新规划完成，共 {} 个乘客需要规划，成功 {} 个",
            replan_count, success_count
        );
    }
}
