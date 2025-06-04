// Passenger system implementation
use crate::game::grid::{Direction, GridPosition, GridState, RouteSegment};
use bevy::{color::palettes::basic, prelude::*};
use std::collections::VecDeque;
use rand::Rng;

// 乘客插件
pub struct PassengerPlugin;

impl Plugin for PassengerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PassengerSpawnTimer>()
            .init_resource::<PassengerManager>()
            .add_systems(
                Update,
                (
                    spawn_passengers,
                    update_passengers,
                    remove_impatient_passengers,
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
        if let Some(next_pos) = self.path.pop_front() {
            self.target_position = next_pos;
        }
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
    pub fn find_destination_station(&self, destination: Destination) -> Option<GridPosition> {
        let valid_stations: Vec<_> = self
            .stations
            .iter()
            .filter(|(_, dests)| dests.contains(&destination))
            .collect();

        if valid_stations.is_empty() {
            return None;
        }

        let index = rand::thread_rng().gen_range(0..valid_stations.len());
        Some(valid_stations[index].0)
    }

    // 寻找从起点到终点的最短路径
    pub fn find_path(
        &self,
        start: GridPosition,
        end: GridPosition,
        grid_state: &GridState,
    ) -> Option<VecDeque<GridPosition>> {
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
            None => return false,
        };

        let segment2 = match grid_state.get_route_segment(*pos2) {
            Some(s) => s,
            None => return false,
        };

        // 确定两个位置的相对方向
        let direction = if pos2.x > pos1.x {
            Direction::East
        } else if pos2.x < pos1.x {
            Direction::West
        } else if pos2.y > pos1.y {
            Direction::North
        } else {
            Direction::South
        };

        // 检查第一个位置是否可以向指定方向移动
        let can_exit_pos1 = match segment1.segment_type {
            RouteSegment::Straight => {
                // 直线只能沿着其方向移动
                segment1.direction == direction || segment1.direction.opposite() == direction
            }
            RouteSegment::Corner => {
                // 转角可以向两个方向移动
                let dir1 = segment1.direction;
                let dir2 = dir1.rotate_cw();
                direction == dir1 || direction == dir2
            }
            RouteSegment::TJunction => {
                // T型路口可以向三个方向移动，不能向其背面移动
                direction != segment1.direction.opposite()
            }
            RouteSegment::Cross => {
                // 十字路口可以向所有方向移动
                true
            }
            RouteSegment::Station => {
                // 车站可以向所有方向移动
                true
            }
            RouteSegment::Grass => {
                // 草地不能移动
                false
            }
        };

        // 检查第二个位置是否可以从指定方向进入
        let can_enter_pos2 = match segment2.segment_type {
            RouteSegment::Straight => {
                // 直线只能沿着其方向移动
                segment2.direction == direction.opposite()
                    || segment2.direction.opposite() == direction.opposite()
            }
            RouteSegment::Corner => {
                // 转角可以从两个方向进入
                let dir1 = segment2.direction;
                let dir2 = dir1.rotate_cw();
                direction.opposite() == dir1 || direction.opposite() == dir2
            }
            RouteSegment::TJunction => {
                // T型路口可以从三个方向进入，不能从其背面进入
                direction.opposite() != segment2.direction.opposite()
            }
            RouteSegment::Cross => {
                // 十字路口可以从所有方向进入
                true
            }
            RouteSegment::Station => {
                // 车站可以从所有方向进入
                true
            }
            RouteSegment::Grass => {
                // 草地不能进入
                false
            }
        };

        can_exit_pos1 && can_enter_pos2
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
        info!("尝试生成新乘客，当前已有乘客数量: {}", passenger_manager.passengers.len());
        info!("可用车站数量: {}", passenger_manager.stations.len());

        // 获取随机起点站
        if let Some(start_pos) = passenger_manager.get_random_start_station() {
            info!("选择起点站: ({}, {})", start_pos.x, start_pos.y);

            // 随机选择目的地类型
            let destination = Destination::random();
            info!("随机选择目的地类型: {:?}", destination);

            // 寻找对应目的地类型的终点站
            if let Some(end_pos) = passenger_manager.find_destination_station(destination) {
                info!("找到终点站: ({}, {})", end_pos.x, end_pos.y);

                // 创建乘客
                let mut passenger = Passenger::new(start_pos, destination);

                // 寻找从起点到终点的路径
                let path_result = passenger_manager.find_path(start_pos, end_pos, &grid_state);

                if let Some(path) = path_result {
                    info!("找到路径，长度: {}", path.len());
                    passenger.set_path(path);
                } else {
                    warn!("无法找到从 ({}, {}) 到 ({}, {}) 的路径，乘客将无法移动", start_pos.x, start_pos.y, end_pos.x, end_pos.y);
                    // 即使没有路径，也会生成乘客，但它们会停留在原地直到失去耐心
                }

                // 无论是否找到路径，都生成乘客实体
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
                info!("成功生成 {:?} 乘客，实体ID: {:?}", destination, passenger_entity);
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
