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
use serde::{Deserialize, Serialize};
use crate::game::grid::TerrainType;

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
        segment_type: RouteSegmentType::DeadEnd,
        direction: Direction::North,
    });
    passenger_manager.add_station(red_station_pos, vec![Destination::Red]);

    // 生成蓝色线路车站
    let blue_station_pos = GridPos::new(grid_config.grid_width - 2, 1);
    commands.trigger(SpawnRouteSegmentEvent {
        grid_pos: blue_station_pos,
        segment_type: RouteSegmentType::DeadEnd,
        direction: Direction::North,
    });
    passenger_manager.add_station(blue_station_pos, vec![Destination::Blue]);

    // 生成绿色线路车站
    let green_station_pos = GridPos::new(1, grid_config.grid_height - 2);
    commands.trigger(SpawnRouteSegmentEvent {
        grid_pos: green_station_pos,
        segment_type: RouteSegmentType::DeadEnd,
        direction: Direction::North,
    });
    passenger_manager.add_station(green_station_pos, vec![Destination::Green]);

    // 生成黄色线路车站
    let yellow_station_pos =
        // GridPosition::new(grid_config.grid_width - 2, grid_config.grid_height - 2);
        GridPos::new(grid_config.grid_width / 2, grid_config.grid_height / 2 + 4);
    commands.trigger(SpawnRouteSegmentEvent {
        grid_pos: yellow_station_pos,
        segment_type: RouteSegmentType::DeadEnd,
        direction: Direction::North,
    });
    passenger_manager.add_station(yellow_station_pos, vec![Destination::Yellow]);

    // 生成中央换乘站
    let central_station_pos = GridPos::new(grid_config.grid_width / 2, grid_config.grid_height / 2);
    commands.trigger(SpawnRouteSegmentEvent {
        grid_pos: central_station_pos,
        segment_type: RouteSegmentType::DeadEnd,
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

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum StationType {
    Start,    // 起点站
    End,      // 终点站
    Transfer, // 换乘站
    Regular,  // 普通站点
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StationConfig {
    pub position: GridPos,
    pub station_type: StationType,
    pub passenger_color: Option<Color>, // 起点站的乘客颜色，终点站的目标颜色
    pub name: String,
    pub capacity: u32, // 站点容量
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerrainTile {
    pub position: GridPos,
    pub terrain_type: TerrainType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrebuiltRoute {
    pub segments: Vec<(GridPos, RouteSegmentType, Direction)>,
    pub is_locked: bool,  // 是否为预设路线（玩家不能修改）
}