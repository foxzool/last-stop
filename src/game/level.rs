// 生成主要关卡

use crate::{
    game::{
        grid::{Direction, GridConfig, GridPos, RouteSegmentType, SpawnRouteSegmentEvent},
        level,
        passenger::{Destination, PassengerManager},
    },
    screens::Screen,
};
use bevy::prelude::*;

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<WantLevel>();
}

#[derive(Resource, Default)]
pub struct WantLevel(pub u8);

// 生成初始路线和车站
#[allow(dead_code)]
pub fn spawn_initial_routes(
    mut commands: Commands,
    grid_config: Res<GridConfig>,
    mut passenger_manager: ResMut<PassengerManager>,
) {
    // 生成红色线路车站
    let red_station_pos = GridPos::new(1, 1);
    commands.trigger(SpawnRouteSegmentEvent {
        grid_pos: red_station_pos,
        segment_type: RouteSegmentType::Station(Destination::Red),
        direction: Direction::North,
    });
    passenger_manager.add_station(red_station_pos, vec![Destination::Red]);

    // 生成蓝色线路车站
    let blue_station_pos = GridPos::new(grid_config.grid_width - 2, 1);
    commands.trigger(SpawnRouteSegmentEvent {
        grid_pos: blue_station_pos,
        segment_type: RouteSegmentType::Station(Destination::Blue),
        direction: Direction::North,
    });
    passenger_manager.add_station(blue_station_pos, vec![Destination::Blue]);

    // 生成绿色线路车站
    let green_station_pos = GridPos::new(1, grid_config.grid_height - 2);
    commands.trigger(SpawnRouteSegmentEvent {
        grid_pos: green_station_pos,
        segment_type: RouteSegmentType::Station(Destination::Green),
        direction: Direction::North,
    });
    passenger_manager.add_station(green_station_pos, vec![Destination::Green]);

    // 生成黄色线路车站
    let yellow_station_pos =
        // GridPosition::new(grid_config.grid_width - 2, grid_config.grid_height - 2);
        GridPos::new(grid_config.grid_width / 2, grid_config.grid_height / 2 + 4);
    commands.trigger(SpawnRouteSegmentEvent {
        grid_pos: yellow_station_pos,
        segment_type: RouteSegmentType::Station(Destination::Yellow),
        direction: Direction::North,
    });
    passenger_manager.add_station(yellow_station_pos, vec![Destination::Yellow]);

    // 生成中央换乘站
    let central_station_pos = GridPos::new(grid_config.grid_width / 2, grid_config.grid_height / 2);
    commands.trigger(SpawnRouteSegmentEvent {
        grid_pos: central_station_pos,
        segment_type: RouteSegmentType::Station(Destination::White),
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
