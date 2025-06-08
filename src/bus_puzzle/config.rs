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
#[allow(dead_code)]
pub const EFFECT_Z: f32 = 4.0;

/// UI元素层（游戏内UI，如路径预览等）
#[allow(dead_code)]
pub const GAME_UI_Z: f32 = 5.0;

/// 菜单UI层（暂停菜单、设置等）
#[allow(dead_code)]
pub const MENU_UI_Z: f32 = 10.0;

/// 调试层（调试信息显示）
#[allow(dead_code)]
pub const DEBUG_Z: f32 = 20.0;

// ============ 游戏核心常量 ============

/// 游戏版本号
#[allow(dead_code)]
pub const GAME_VERSION: &str = env!("CARGO_PKG_VERSION");

/// 默认地图瓦片大小
pub const DEFAULT_TILE_SIZE: f32 = 64.0;

// ============ 游戏平衡性常量 ============

/// 每个站点最大乘客容量
#[allow(dead_code)]
pub const MAX_PASSENGERS_PER_STATION: u32 = 20;

/// 默认乘客耐心值（秒）
pub const DEFAULT_PASSENGER_PATIENCE: f32 = 60.0;

/// 路线段放置成本数组 [直线, 转弯, T型, 十字, 桥梁, 隧道]
pub const SEGMENT_PLACEMENT_COST: [u32; 6] = [1, 2, 3, 4, 5, 6];

// ============ 寻路算法常量 ============

/// 寻路算法最大迭代次数
#[allow(dead_code)]
pub const MAX_PATHFINDING_ITERATIONS: usize = 1000;

/// 换乘成本倍数
#[allow(dead_code)]
pub const TRANSFER_COST_MULTIPLIER: f32 = 5.0;

/// 步行速度 (像素/秒)
#[allow(dead_code)]
pub const WALKING_SPEED: f32 = 50.0;

/// 公交车速度 (像素/秒)
#[allow(dead_code)]
pub const BUS_SPEED: f32 = 150.0;

// ============ UI 相关常量 ============

/// UI 动画持续时间（秒）
#[allow(dead_code)]
pub const UI_ANIMATION_DURATION: f32 = 0.5;

/// 按钮悬停时的缩放比例
#[allow(dead_code)]
pub const BUTTON_HOVER_SCALE: f32 = 1.1;

/// 库存槽位大小（像素）
pub const INVENTORY_SLOT_SIZE: f32 = 70.0;
