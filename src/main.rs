// 支持在代码中配置Bevy的lint检查。
#![cfg_attr(bevy_lint, feature(register_tool), register_tool(bevy))]
// 在Windows非开发版本中禁用控制台。
#![cfg_attr(not(feature = "dev"), windows_subsystem = "windows")]

mod asset_tracking;
mod audio;
mod bus_puzzle;
#[cfg(feature = "dev")]
mod dev_tools;
mod menus;
mod screens;
mod theme;

use bevy::{
    asset::AssetMetaCheck,
    prelude::*,
    render::view::screenshot::{Screenshot, save_to_disk},
};

fn main() -> AppExit {
    App::new().add_plugins(AppPlugin).run()
}

pub struct AppPlugin;

impl Plugin for AppPlugin {
    fn build(&self, app: &mut App) {
        // 添加Bevy插件。
        app.add_plugins(
            DefaultPlugins
                .set(AssetPlugin {
                    // 如果不设置这个，Wasm构建将检查元数据文件（不存在的）。
                    // 这会在itch上的web构建中导致错误，甚至崩溃。
                    // 参见 https://github.com/bevyengine/bevy_github_ci_template/issues/48。
                    meta_check: AssetMetaCheck::Never,
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: Window {
                        title: "Last Stop".to_string(),
                        fit_canvas_to_parent: true,
                        ..default()
                    }
                    .into(),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
        );

        // 添加其他插件。
        app.add_plugins((
            bus_puzzle::BusPuzzleGamePlugin,
            // #[cfg(feature = "dev")]
            // dev_tools::plugin,
        ));

        // 生成主摄像机。
        app.add_systems(Startup, spawn_camera)
            .add_systems(Update, (debug_info_system, screenshot_system));
    }
}

/// 应用程序在`Update`调度中的高级系统分组。
/// 添加新的变体时，请确保在`configure_sets`中对其进行排序
/// 上面的调用。
#[derive(SystemSet, Debug, Clone, Copy, Eq, PartialEq, Hash, PartialOrd, Ord)]
enum AppSystems {
    /// 更新计时器。
    TickTimers,
    /// 记录玩家输入。
    RecordInput,
    /// 处理所有其他事项（考虑将其拆分为更多变体）。
    Update,
}

/// 游戏是否暂停。
#[derive(States, Copy, Clone, Eq, PartialEq, Hash, Debug, Default)]
#[states(scoped_entities)]
struct Pause(pub bool);

/// 在游戏暂停时不应运行的系统集。
#[derive(SystemSet, Copy, Clone, Eq, PartialEq, Hash, Debug)]
struct PausableSystems;

fn spawn_camera(mut commands: Commands) {
    commands.spawn((Name::new("Camera"), Camera2d));
}

fn debug_info_system(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    game_state: Res<bus_puzzle::GameState>,
    passengers: Query<&bus_puzzle::PathfindingAgent>,
    placed_segments: Query<&bus_puzzle::RouteSegment>,
) {
    if keyboard_input.just_pressed(KeyCode::F1) {
        info!("=== 调试信息 ===");
        info!("游戏时间: {:.1}秒", game_state.game_time);
        info!("总成本: {}", game_state.total_cost);
        info!("已放置路段数: {}", placed_segments.iter().count());
        info!("乘客总数: {}", passengers.iter().count());

        let arrived_count = passengers
            .iter()
            .filter(|agent| matches!(agent.state, bus_puzzle::AgentState::Arrived))
            .count();

        info!("已到达乘客数: {}", arrived_count);
        info!("目标完成情况: {:?}", game_state.objectives_completed);
        info!("当前得分: {}", game_state.score.total_score);
    }
}

fn screenshot_system(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    main_window: Query<Entity, With<bevy::window::PrimaryWindow>>,
) {
    if keyboard_input.just_pressed(KeyCode::F12) {
        let path = format!(
            "screenshot_{}.png",
            chrono::Utc::now().format("%Y%m%d_%H%M%S")
        );
        info!("截图保存到: {}", path);
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path));
    }
}
