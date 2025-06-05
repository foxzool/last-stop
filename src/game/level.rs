// 生成主要关卡

use crate::game::{
    grid::{Direction, GridConfig, GridPosition, RouteSegment, SpawnRouteSegmentEvent},
    passenger::{Destination, PassengerManager},
};
use bevy::prelude::*;

pub(super) fn plugin(app: &mut App) {
    // 可以在这里添加关卡相关的系统
}

// 生成游戏关卡
pub fn spawn_level(
    mut commands: Commands, // Kept for now, in case it's used for other things in spawn_level
    grid_config: Res<GridConfig>,
    mut passenger_manager: ResMut<PassengerManager>,
) {
}

// 生成初始路线和车站
pub fn spawn_initial_routes(
    mut commands: Commands, // Kept for now, in case it's used for other things in spawn_level
    grid_config: Res<GridConfig>,
    mut passenger_manager: ResMut<PassengerManager>,
) {
    // 生成红色线路车站
    let red_station_pos = GridPosition::new(1, 1);
    commands.trigger(SpawnRouteSegmentEvent {
        grid_pos: red_station_pos,
        segment_type: RouteSegment::Station,
        direction: Direction::North,
    });
    passenger_manager.add_station(red_station_pos, vec![Destination::Red]);

    // 生成蓝色线路车站
    let blue_station_pos = GridPosition::new(grid_config.grid_width - 2, 1);
    commands.trigger(SpawnRouteSegmentEvent {
        grid_pos: blue_station_pos,
        segment_type: RouteSegment::Station,
        direction: Direction::North,
    });
    passenger_manager.add_station(blue_station_pos, vec![Destination::Blue]);

    // 生成绿色线路车站
    let green_station_pos = GridPosition::new(1, grid_config.grid_height - 2);
    commands.trigger(SpawnRouteSegmentEvent {
        grid_pos: green_station_pos,
        segment_type: RouteSegment::Station,
        direction: Direction::North,
    });
    passenger_manager.add_station(green_station_pos, vec![Destination::Green]);

    // 生成黄色线路车站
    let yellow_station_pos =
        // GridPosition::new(grid_config.grid_width - 2, grid_config.grid_height - 2);
        GridPosition::new(grid_config.grid_width / 2 , grid_config.grid_height / 2 + 4);
    commands.trigger(SpawnRouteSegmentEvent {
        grid_pos: yellow_station_pos,
        segment_type: RouteSegment::Station,
        direction: Direction::North,
    });
    passenger_manager.add_station(yellow_station_pos, vec![Destination::Yellow]);

    // 生成中央换乘站
    let central_station_pos =
        GridPosition::new(grid_config.grid_width / 2, grid_config.grid_height / 2);
    commands.trigger(SpawnRouteSegmentEvent {
        grid_pos: central_station_pos,
        segment_type: RouteSegment::Station,
        direction: Direction::North,
    });
    passenger_manager.add_station(
        central_station_pos,
        vec![
            Destination::Red,
            Destination::Blue,
            Destination::Green,
            Destination::Yellow,
        ],
    );
}
