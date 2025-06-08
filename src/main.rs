// 支持在代码中配置Bevy的lint检查。
#![cfg_attr(bevy_lint, feature(register_tool), register_tool(bevy))]
// 在Windows非开发版本中禁用控制台。
#![cfg_attr(not(feature = "dev"), windows_subsystem = "windows")]
extern crate core;

mod bus_puzzle;
#[cfg(feature = "dev")]
mod dev_tools;

use bevy::{asset::AssetMetaCheck, prelude::*};

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
        app.add_systems(Startup, spawn_camera);

        #[cfg(not(target_family = "wasm"))]
        app.add_systems(Update, screenshot_system);
    }
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn((Name::new("Camera"), Camera2d));
}

#[cfg(not(target_family = "wasm"))]
fn screenshot_system(keyboard_input: Res<ButtonInput<KeyCode>>, mut commands: Commands) {
    use bevy::render::view::screenshot::{save_to_disk, Screenshot};
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
