// 生成主要关卡

use crate::game::{
    grid::{Direction, GridConfig, GridPosition, RouteSegment, spawn_route_segment},
    passenger::{Destination, PassengerManager},
};
use bevy::prelude::*;

pub(super) fn plugin(app: &mut App) {
    // 可以在这里添加关卡相关的系统
}

// 生成游戏关卡
pub fn spawn_level(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    grid_config: Res<GridConfig>,
    mut passenger_manager: ResMut<PassengerManager>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    // 生成一些基础路线和车站
    spawn_initial_routes(
        &mut commands,
        &asset_server,
        &grid_config,
        &mut passenger_manager,
        &mut texture_atlas_layouts,
    );
}

// 生成初始路线和车站
fn spawn_initial_routes(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    grid_config: &Res<GridConfig>,
    passenger_manager: &mut PassengerManager,
    mut texture_atlas_layouts: &mut ResMut<Assets<TextureAtlasLayout>>,
) {
    // 生成红色线路车站
    let red_station_pos = GridPosition::new(1, 1);
    spawn_route_segment(
        commands,
        red_station_pos,
        RouteSegment::Station,
        Direction::North,
        asset_server,
        grid_config,
        &mut texture_atlas_layouts,
    );
    passenger_manager.add_station(red_station_pos, vec![Destination::Red]);
    
    // 生成蓝色线路车站
    let blue_station_pos = GridPosition::new(grid_config.grid_width - 2, 1);
    spawn_route_segment(
        commands,
        blue_station_pos,
        RouteSegment::Station,
        Direction::North,
        asset_server,
        grid_config,
        &mut texture_atlas_layouts,
    );
    passenger_manager.add_station(blue_station_pos, vec![Destination::Blue]);
    
    // 生成绿色线路车站
    let green_station_pos = GridPosition::new(1, grid_config.grid_height - 2);
    spawn_route_segment(
        commands,
        green_station_pos,
        RouteSegment::Station,
        Direction::North,
        asset_server,
        grid_config,
        &mut texture_atlas_layouts,
    );
    passenger_manager.add_station(green_station_pos, vec![Destination::Green]);

    // 生成黄色线路车站
    let yellow_station_pos =
        // GridPosition::new(grid_config.grid_width - 2, grid_config.grid_height - 2);
        GridPosition::new(grid_config.grid_width / 2 , grid_config.grid_height / 2 + 4);
    spawn_route_segment(
        commands,
        yellow_station_pos,
        RouteSegment::Station,
        Direction::North,
        asset_server,
        grid_config,
        &mut texture_atlas_layouts,
    );
    passenger_manager.add_station(yellow_station_pos, vec![Destination::Yellow]);

    // 生成中央换乘站
    let central_station_pos =
        GridPosition::new(grid_config.grid_width / 2, grid_config.grid_height / 2);
    spawn_route_segment(
        commands,
        central_station_pos,
        RouteSegment::Station,
        Direction::North,
        asset_server,
        grid_config,
        &mut texture_atlas_layouts,
    );
    passenger_manager.add_station(
        central_station_pos,
        vec![
            Destination::Red,
            Destination::Blue,
            Destination::Green,
            Destination::Yellow,
        ],
    );

    // 可以在这里添加更多的初始路线
}
