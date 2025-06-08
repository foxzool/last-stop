use crate::bus_puzzle::{GridPos, PassengerColor};
use bevy::prelude::*;

/// 将世界坐标转换为网格坐标
pub fn world_to_grid(
    world_pos: Vec3,
    tile_size: f32,
    grid_width: u32,
    grid_height: u32,
) -> GridPos {
    let center_offset_x = (grid_width as f32 - 1.0) * tile_size * 0.5;
    let center_offset_y = (grid_height as f32 - 1.0) * tile_size * 0.5;

    let adjusted_x = world_pos.x + center_offset_x;
    let adjusted_y = world_pos.y + center_offset_y;

    // 使用 round() 来获得最接近的网格坐标
    // 这样可以确保鼠标点击位置正确映射到最近的网格中心
    GridPos::new(
        (adjusted_x / tile_size).round() as i32,
        (adjusted_y / tile_size).round() as i32,
    )
}

/// 调试用：验证坐标转换的准确性
pub fn debug_coordinate_conversion(
    world_pos: Vec3,
    tile_size: f32,
    grid_width: u32,
    grid_height: u32,
) {
    let grid_pos = world_to_grid(world_pos, tile_size, grid_width, grid_height);
    let back_to_world = grid_pos.to_world_pos(tile_size, grid_width, grid_height);

    let distance = world_pos.distance(back_to_world);

    info!(
        "坐标转换验证: 世界 {:?} -> 网格 {:?} -> 世界 {:?}, 距离差: {:.2}",
        world_pos, grid_pos, back_to_world, distance
    );

    if distance > tile_size * 0.1 {
        warn!("坐标转换精度可能有问题，距离差过大: {:.2}", distance);
    }
}

/// 计算两点间的曼哈顿距离
pub fn manhattan_distance(pos1: GridPos, pos2: GridPos) -> u32 {
    ((pos1.x - pos2.x).abs() + (pos1.y - pos2.y).abs()) as u32
}

/// 获取网格位置的相邻位置（四个方向）
pub fn get_neighbors(pos: GridPos) -> Vec<GridPos> {
    vec![
        GridPos::new(pos.x, pos.y - 1), // 上
        GridPos::new(pos.x, pos.y + 1), // 下
        GridPos::new(pos.x - 1, pos.y), // 左
        GridPos::new(pos.x + 1, pos.y), // 右
    ]
}

/// 缓动函数 - ease out back
pub fn ease_out_back(t: f32) -> f32 {
    let c1 = 1.70158;
    let c3 = c1 + 1.0;
    1.0 + c3 * (t - 1.0).powi(3) + c1 * (t - 1.0).powi(2)
}

/// 格式化时间显示
pub fn format_time(seconds: f32) -> String {
    let minutes = (seconds / 60.0) as u32;
    let secs = (seconds % 60.0) as u32;
    format!("{:02}:{:02}", minutes, secs)
}

/// 颜色工具函数
pub fn get_passenger_color(passenger_color: PassengerColor) -> Color {
    match passenger_color {
        PassengerColor::Red => Color::srgb(1.0, 0.2, 0.2),
        PassengerColor::Blue => Color::srgb(0.2, 0.2, 1.0),
        PassengerColor::Green => Color::srgb(0.2, 1.0, 0.2),
        PassengerColor::Yellow => Color::srgb(1.0, 1.0, 0.2),
        PassengerColor::Purple => Color::srgb(0.8, 0.2, 0.8),
        PassengerColor::Orange => Color::srgb(1.0, 0.6, 0.2),
    }
}
