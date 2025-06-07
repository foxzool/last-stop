// 支持在代码中配置Bevy的lint检查。
#![cfg_attr(bevy_lint, feature(register_tool), register_tool(bevy))]
// 在Windows非开发版本中禁用控制台。
#![cfg_attr(not(feature = "dev"), windows_subsystem = "windows")]

mod bus_puzzle;
#[cfg(feature = "dev")]
mod dev_tools;

use bevy::{
    asset::AssetMetaCheck,
    prelude::*,
    render::view::screenshot::{save_to_disk, Screenshot},
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
            #[cfg(feature = "dev")]
            dev_tools::plugin,
        ));
        // .add_plugins(bevy_inspector_egui::bevy_egui::EguiPlugin {
        //     enable_multipass_for_primary_context: true,
        // })
        // .add_plugins(WorldInspectorPlugin::new());

        // 生成主摄像机。
        app.add_systems(Startup, spawn_camera).add_systems(
            Update,
            (debug_info_system, debug_state_switch, screenshot_system),
        );
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
    current_state: Res<State<bus_puzzle::GameStateEnum>>,
    time: Res<Time>,
) {
    if keyboard_input.just_pressed(KeyCode::F1) {
        info!("=== 详细调试信息 ===");
        info!("当前游戏状态: {:?}", current_state.get());
        info!("游戏时间: {:.1}秒", game_state.game_time);
        info!("系统运行时间: {:.1}秒", time.elapsed_secs());
        info!("总成本: {}", game_state.total_cost);
        info!("已放置路段数: {}", placed_segments.iter().count());

        // 乘客详细信息
        info!("乘客总数: {}", passengers.iter().count());
        let mut state_counts = std::collections::HashMap::new();
        for agent in passengers.iter() {
            *state_counts
                .entry(format!("{:?}", agent.state))
                .or_insert(0) += 1;
        }
        for (state, count) in state_counts {
            info!("  {}: {} 个乘客", state, count);
        }

        let arrived_count = passengers
            .iter()
            .filter(|agent| matches!(agent.state, bus_puzzle::AgentState::Arrived))
            .count();

        info!("已到达乘客数: {}", arrived_count);
        info!("目标完成情况: {:?}", game_state.objectives_completed);
        info!("当前得分: {}", game_state.score.total_score);

        // 关卡信息
        if let Some(level_data) = &game_state.current_level {
            info!("当前关卡: {} ({})", level_data.name, level_data.id);
            info!("关卡尺寸: {:?}", level_data.grid_size);
            info!("站点数: {}", level_data.stations.len());
            info!("乘客需求数: {}", level_data.passenger_demands.len());

            for (i, demand) in level_data.passenger_demands.iter().enumerate() {
                info!(
                    "  需求{}: {:?} {} -> {} (生成率: {}/秒)",
                    i, demand.color, demand.origin, demand.destination, demand.spawn_rate
                );
            }
        } else {
            warn!("没有关卡数据！");
        }

        info!("=== 按 F2 查看乘客生成详情，F3 手动生成测试乘客 ===");
    }
}

// 添加快速切换游戏状态的调试功能
fn debug_state_switch(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    current_state: Res<State<bus_puzzle::GameStateEnum>>,
    mut next_state: ResMut<NextState<bus_puzzle::GameStateEnum>>,
) {
    if keyboard_input.just_pressed(KeyCode::F4) {
        match current_state.get() {
            bus_puzzle::GameStateEnum::MainMenu => {
                next_state.set(bus_puzzle::GameStateEnum::Playing);
                info!("切换到游戏状态");
            }
            bus_puzzle::GameStateEnum::Playing => {
                next_state.set(bus_puzzle::GameStateEnum::MainMenu);
                info!("切换到主菜单");
            }
            _ => {
                next_state.set(bus_puzzle::GameStateEnum::Playing);
                info!("强制切换到游戏状态");
            }
        }
    }
}

fn screenshot_system(keyboard_input: Res<ButtonInput<KeyCode>>, mut commands: Commands) {
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
