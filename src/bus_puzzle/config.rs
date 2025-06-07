// ============ Z 轴渲染层级常量 ============

/// 地形层（草地、水面、建筑等背景）
pub const TERRAIN_Z: f32 = 0.0;

/// 路线段层（公交路线、道路等）
pub const ROUTE_Z: f32 = 1.0;

/// 站点层（公交站、换乘枢纽等）
pub const STATION_Z: f32 = 2.0;

/// 乘客层（移动的乘客实体）
pub const PASSENGER_Z: f32 = 3.0;

/// 特效层（粒子效果、动画等）
pub const EFFECT_Z: f32 = 4.0;

/// UI元素层（游戏内UI，如路径预览等）
pub const GAME_UI_Z: f32 = 5.0;

/// 菜单UI层（暂停菜单、设置等）
pub const MENU_UI_Z: f32 = 10.0;

/// 调试层（调试信息显示）
pub const DEBUG_Z: f32 = 20.0;

// 游戏常量
pub const GAME_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const DEFAULT_TILE_SIZE: f32 = 64.0;
pub const MAX_PASSENGERS_PER_STATION: u32 = 20;
pub const DEFAULT_PASSENGER_PATIENCE: f32 = 60.0;
pub const SEGMENT_PLACEMENT_COST: [u32; 6] = [1, 2, 3, 4, 5, 6]; // 对应不同路线段类型

// 路径寻找相关常量
pub const MAX_PATHFINDING_ITERATIONS: usize = 1000;
pub const TRANSFER_COST_MULTIPLIER: f32 = 5.0;
pub const WALKING_SPEED: f32 = 50.0;
pub const BUS_SPEED: f32 = 150.0;

// UI 相关常量
pub const UI_ANIMATION_DURATION: f32 = 0.5;
pub const BUTTON_HOVER_SCALE: f32 = 1.1;
pub const INVENTORY_SLOT_SIZE: f32 = 80.0;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_z_layer_ordering() {
        assert!(TERRAIN_Z < ROUTE_Z);
        assert!(ROUTE_Z < STATION_Z);
        assert!(STATION_Z < PASSENGER_Z);
        assert!(PASSENGER_Z < EFFECT_Z);
        assert!(EFFECT_Z < GAME_UI_Z);
        assert!(GAME_UI_Z < MENU_UI_Z);
        assert!(MENU_UI_Z < DEBUG_Z);
    }
}
