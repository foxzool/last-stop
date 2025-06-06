

// 模块声明
pub mod level_system;
pub mod pathfinding;
pub mod interaction;
pub mod ui_audio;
pub mod components;
pub mod resources;
pub mod events;


// 重新导出主要类型
pub use components::*;
pub use resources::*;
pub use events::*;
pub use level_system::*;
pub use pathfinding::*;
pub use interaction::*;
pub use ui_audio::*;

use bevy::prelude::*;

// 主游戏插件
pub struct BusPuzzleGamePlugin;

impl Plugin for BusPuzzleGamePlugin {
    fn build(&self, app: &mut App) {
        app
            // 添加状态
            .init_state::<GameStateEnum>()

            // 添加资源
            .insert_resource(GameConfig::default())
            .insert_resource(LevelManager::default())
            .insert_resource(GameState::default())
            .insert_resource(InputState::default())
            .insert_resource(CameraController::default())
            .insert_resource(PathfindingGraph::default())

            // 添加事件
            .add_event::<SegmentPlacedEvent>()
            .add_event::<SegmentRemovedEvent>()
            .add_event::<ObjectiveCompletedEvent>()
            .add_event::<LevelCompletedEvent>()
            .add_event::<InventoryUpdatedEvent>()

            // 添加子插件
            .add_plugins((
                LevelGenerationPlugin,
                PathfindingPlugin,
                PuzzleInteractionPlugin,
                GameUIPlugin,
            ))

            // 启动系统
            .add_systems(Startup, initialize_game)

            // 游戏循环系统
            .add_systems(OnEnter(GameStateEnum::Loading), load_current_level)
            .add_systems(Update, (
                update_game_score,
                check_level_failure_conditions,
            ).run_if(in_state(GameStateEnum::Playing)));
    }
}
