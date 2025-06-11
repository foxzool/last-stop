// src/bus_puzzle/tips_system.rs - 游戏提示系统

use crate::bus_puzzle::{
    get_text, get_text_with_args, CurrentLanguage, GameState, GameStateEnum, LevelData,
    BUDGET_WARNING, BUS_ROUTES_READY_INFO, PASSENGERS_GAVE_UP_WARNING, PASSENGERS_WAITING_HINT,
};
use bevy::prelude::{
    Val::{Percent, Px},
    *,
};
// ============ Tips 组件和资源 ============

#[derive(Component)]
pub struct TipsPanel;

#[derive(Component)]
#[allow(dead_code)]
pub struct TipText {
    pub tip_type: TipType,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TipType {
    LevelGoal, // 关卡目标
    Strategy,  // 策略建议
    Controls,  // 操作提示
    Warning,   // 注意事项
}

#[derive(Resource, Default)]
pub struct TipsManager {
    pub current_tips: Vec<GameTip>,
    pub is_expanded: bool,
    pub last_level_id: String,
}

#[derive(Debug, Clone)]
pub struct GameTip {
    pub tip_type: TipType,
    pub title: String,
    pub content: String,
    pub icon: String,
    pub color: Color,
}

// ============ Tips 系统插件 ============

pub struct TipsSystemPlugin;

impl Plugin for TipsSystemPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TipsManager>().add_systems(
            Update,
            (
                update_tips_for_level,
                handle_tips_panel_toggle,
                update_tips_display,
                cleanup_expired_tips, // 新增：清理过期提示
            )
                .run_if(in_state(GameStateEnum::Playing)),
        );
    }
}

// ============ Tips 内容生成 ============

impl TipsManager {
    pub fn generate_tips_for_level(&mut self, level_data: &LevelData) {
        self.current_tips.clear();

        match level_data.id.as_str() {
            "tutorial_01" => {
                self.current_tips = vec![
                    GameTip {
                        tip_type: TipType::LevelGoal,
                        title: "关卡目标".to_string(),
                        content: "连接A站和B站，让红色乘客能够到达目的地".to_string(),
                        icon: "🎯".to_string(),
                        color: Color::srgb(0.2, 0.8, 0.2),
                    },
                    GameTip {
                        tip_type: TipType::Strategy,
                        title: "建设策略".to_string(),
                        content: "使用直线段是最经济的选择，只需要简单的直线连接即可".to_string(),
                        icon: "💡".to_string(),
                        color: Color::srgb(0.9, 0.7, 0.2),
                    },
                    GameTip {
                        tip_type: TipType::Controls,
                        title: "操作提示".to_string(),
                        content: "左键放置路段，右键旋转方向，Delete键删除路段".to_string(),
                        icon: "🎮".to_string(),
                        color: Color::srgb(0.2, 0.6, 0.9),
                    },
                ];
            }
            "level_02_transfer" => {
                self.current_tips = vec![
                    GameTip {
                        tip_type: TipType::LevelGoal,
                        title: "关卡目标".to_string(),
                        content: "学会使用换乘：乘客需要在中转站换乘前往不同目的地".to_string(),
                        icon: "🎯".to_string(),
                        color: Color::srgb(0.2, 0.8, 0.2),
                    },
                    GameTip {
                        tip_type: TipType::Strategy,
                        title: "换乘策略".to_string(),
                        content: "规划两条路线：A站→中转站，中转站→B站/C站，让乘客在中转站换乘"
                            .to_string(),
                        icon: "🔄".to_string(),
                        color: Color::srgb(0.9, 0.7, 0.2),
                    },
                    GameTip {
                        tip_type: TipType::Warning,
                        title: "避免绕路".to_string(),
                        content: "避免让乘客换乘太多次，每次换乘都会增加等待时间".to_string(),
                        icon: "⚠️".to_string(),
                        color: Color::srgb(0.9, 0.3, 0.2),
                    },
                ];
            }
            "level_03_multiple_routes" => {
                self.current_tips = vec![
                    GameTip {
                        tip_type: TipType::LevelGoal,
                        title: "关卡目标".to_string(),
                        content: "管理多条路线，优化整个交通网络的效率".to_string(),
                        icon: "🎯".to_string(),
                        color: Color::srgb(0.2, 0.8, 0.2),
                    },
                    GameTip {
                        tip_type: TipType::Strategy,
                        title: "网络规划".to_string(),
                        content: "善用中央枢纽作为换乘点，可以减少总的路线段数量".to_string(),
                        icon: "🗺️".to_string(),
                        color: Color::srgb(0.9, 0.7, 0.2),
                    },
                    GameTip {
                        tip_type: TipType::Warning,
                        title: "预算控制".to_string(),
                        content: "注意控制成本！优先使用便宜的直线段，谨慎使用昂贵的桥梁"
                            .to_string(),
                        icon: "💰".to_string(),
                        color: Color::srgb(0.9, 0.3, 0.2),
                    },
                    GameTip {
                        tip_type: TipType::Strategy,
                        title: "跨河策略".to_string(),
                        content: "河流阻挡了直接路径，使用桥梁路段跨越水面".to_string(),
                        icon: "🌉".to_string(),
                        color: Color::srgb(0.2, 0.7, 0.9),
                    },
                ];
            }
            "level_04_time_pressure" => {
                self.current_tips = vec![
                    GameTip {
                        tip_type: TipType::LevelGoal,
                        title: "关卡目标".to_string(),
                        content: "在60秒内完成网络建设，快速响应是关键".to_string(),
                        icon: "⏰".to_string(),
                        color: Color::srgb(0.2, 0.8, 0.2),
                    },
                    GameTip {
                        tip_type: TipType::Strategy,
                        title: "快速建设".to_string(),
                        content: "优先建设主要路线，不要追求完美的网络设计".to_string(),
                        icon: "⚡".to_string(),
                        color: Color::srgb(0.9, 0.7, 0.2),
                    },
                    GameTip {
                        tip_type: TipType::Warning,
                        title: "时间压力".to_string(),
                        content: "乘客耐心较短，延误可能导致大量乘客放弃！".to_string(),
                        icon: "🚨".to_string(),
                        color: Color::srgb(0.9, 0.3, 0.2),
                    },
                    GameTip {
                        tip_type: TipType::Strategy,
                        title: "穿山隧道".to_string(),
                        content: "山脉阻挡了路径，使用隧道路段穿越山区".to_string(),
                        icon: "🏔️".to_string(),
                        color: Color::srgb(0.6, 0.4, 0.2),
                    },
                ];
            }
            _ => {
                // 默认通用提示
                self.current_tips = vec![
                    GameTip {
                        tip_type: TipType::Strategy,
                        title: "基本策略".to_string(),
                        content: "观察乘客需求，规划最短有效路径".to_string(),
                        icon: "💡".to_string(),
                        color: Color::srgb(0.9, 0.7, 0.2),
                    },
                    GameTip {
                        tip_type: TipType::Controls,
                        title: "操作提示".to_string(),
                        content: "F4发现公交路线，F6查看乘客状态".to_string(),
                        icon: "🎮".to_string(),
                        color: Color::srgb(0.2, 0.6, 0.9),
                    },
                ];
            }
        }

        self.last_level_id = level_data.id.clone();
        info!("生成 {} 条关卡提示", self.current_tips.len());
    }

    pub fn get_segment_tips(&self) -> Vec<GameTip> {
        vec![
            GameTip {
                tip_type: TipType::Strategy,
                title: "路段选择".to_string(),
                content: "直线段(成本1) < 转弯段(成本2) < T型(成本3) < 十字(成本4)".to_string(),
                icon: "🛤️".to_string(),
                color: Color::srgb(0.7, 0.7, 0.7),
            },
            GameTip {
                tip_type: TipType::Strategy,
                title: "特殊路段".to_string(),
                content: "桥梁跨越水面(成本5)，隧道穿越山脉(成本6)".to_string(),
                icon: "🌉".to_string(),
                color: Color::srgb(0.2, 0.7, 0.9),
            },
        ]
    }
}

// ============ Tips 更新系统 ============

fn update_tips_for_level(mut tips_manager: ResMut<TipsManager>, game_state: Res<GameState>) {
    if let Some(level_data) = &game_state.current_level {
        // 只在关卡改变时更新提示
        if tips_manager.last_level_id != level_data.id {
            tips_manager.generate_tips_for_level(level_data);
        }
    }
}

fn handle_tips_panel_toggle(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut tips_manager: ResMut<TipsManager>,
) {
    if keyboard_input.just_pressed(KeyCode::F1) {
        tips_manager.is_expanded = !tips_manager.is_expanded;
        info!(
            "Tips面板切换: {}",
            if tips_manager.is_expanded {
                "展开"
            } else {
                "收起"
            }
        );
    }
}

fn update_tips_display(
    tips_manager: Res<TipsManager>,
    mut tips_panels: Query<(&mut Visibility, &Children), With<TipsPanel>>,
    mut tip_items: Query<&mut Visibility, Without<TipsPanel>>,
) {
    for (mut visibility, children) in tips_panels.iter_mut() {
        *visibility = if tips_manager.is_expanded {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };

        for child in children {
            if let Ok(mut child_visibility) = tip_items.get_mut(*child) {
                *child_visibility = *visibility;
            }
        }
    }
}

// ============ Tips UI 组件创建 ============

pub fn create_tips_panel(
    parent: &mut ChildSpawnerCommands,
    ui_assets: &crate::bus_puzzle::UIAssets,
    tips_manager: &TipsManager,
) {
    // 标题栏（为1280x720优化）
    parent
        .spawn((Node {
            width: Percent(100.0),
            height: Px(24.0), // 缩小标题栏高度
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            margin: UiRect::bottom(Px(6.0)), // 减少底部边距
            ..default()
        },))
        .with_children(|parent| {
            parent.spawn((
                Text::new("💡 关卡提示"),
                TextFont {
                    font: ui_assets.font.clone(),
                    font_size: 15.0, // 缩小字体
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.9, 0.3)),
            ));

            parent.spawn((
                Text::new("F1 切换"),
                TextFont {
                    font: ui_assets.font.clone(),
                    font_size: 10.0, // 缩小字体
                    ..default()
                },
                TextColor(Color::srgb(0.7, 0.7, 0.7)),
            ));
        });

    // 提示内容滚动区域（为1280x720优化）
    parent
        .spawn((
            Node {
                width: Percent(100.0),
                height: Px(280.0), // 设置固定高度，适应面板大小
                flex_direction: FlexDirection::Column,
                row_gap: Px(8.0), // 减少间距
                overflow: Overflow::clip_y(),
                ..default()
            },
            Visibility::Visible, // 默认显示
        ))
        .with_children(|parent| {
            // 当前关卡提示
            for tip in &tips_manager.current_tips {
                create_tip_item(parent, ui_assets, tip);
            }

            // 分隔线（适应小窗口）
            if !tips_manager.current_tips.is_empty() {
                parent.spawn((
                    Node {
                        width: Percent(100.0),
                        height: Px(1.0),
                        margin: UiRect::vertical(Px(6.0)), // 减少间距
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.4, 0.4, 0.4)),
                ));
            }

            // 通用路段提示
            let segment_tips = tips_manager.get_segment_tips();
            for tip in &segment_tips {
                create_tip_item(parent, ui_assets, tip);
            }
        });
}

fn create_tip_item(
    parent: &mut ChildSpawnerCommands,
    ui_assets: &crate::bus_puzzle::UIAssets,
    tip: &GameTip,
) {
    parent
        .spawn((
            Node {
                width: Percent(100.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Px(6.0)), // 减少内边距
                row_gap: Px(3.0),              // 减少间距
                border: UiRect::all(Px(1.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.2, 0.2, 0.3, 0.7)),
            BorderColor(tip.color),
            TipText {
                tip_type: tip.tip_type.clone(),
            },
        ))
        .with_children(|parent| {
            // 标题行
            parent
                .spawn((Node {
                    width: Percent(100.0),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Px(4.0), // 减少间距
                    ..default()
                },))
                .with_children(|parent| {
                    // 图标
                    parent.spawn((
                        Text::new(&tip.icon),
                        TextFont {
                            font: ui_assets.font.clone(),
                            font_size: 14.0, // 缩小图标字体
                            ..default()
                        },
                        TextColor(tip.color),
                    ));

                    // 标题
                    parent.spawn((
                        Text::new(&tip.title),
                        TextFont {
                            font: ui_assets.font.clone(),
                            font_size: 12.0, // 缩小标题字体
                            ..default()
                        },
                        TextColor(Color::WHITE),
                        Node {
                            flex_grow: 1.0,
                            ..default()
                        },
                    ));
                });

            // 内容
            parent.spawn((
                Text::new(&tip.content),
                TextFont {
                    font: ui_assets.font.clone(),
                    font_size: 10.0, // 缩小内容字体
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.9, 0.9)),
                Node {
                    width: Percent(100.0),
                    ..default()
                },
            ));
        });
}

// ============ 动态提示系统 ============

pub fn show_contextual_tip(
    commands: &mut Commands,
    ui_assets: &crate::bus_puzzle::UIAssets,
    tip_content: &str,
    tip_type: TipType,
    duration: f32,
) {
    let tip_color = match tip_type {
        TipType::LevelGoal => Color::srgb(0.2, 0.8, 0.2),
        TipType::Strategy => Color::srgb(0.9, 0.7, 0.2),
        TipType::Controls => Color::srgb(0.2, 0.6, 0.9),
        TipType::Warning => Color::srgb(0.9, 0.3, 0.2),
    };

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: Px(70.0),               // 调整位置，避免与右侧面板重叠
                top: Px(450.0),                 // 调整位置，适应1280x720窗口
                width: Px(250.0),               // 稍微缩小宽度
                padding: UiRect::all(Px(10.0)), // 减少内边距
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
            BorderColor(tip_color),
            Outline::new(Px(2.0), Val::ZERO, tip_color),
            ZIndex(200),
            Name::new("Contextual Tip"),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new(tip_content),
                TextFont {
                    font: ui_assets.font.clone(),
                    font_size: 12.0, // 缩小字体
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        })
        .insert(
            // 添加自动消失组件
            TipTimer(Timer::from_seconds(duration, TimerMode::Once)),
        );
}

#[derive(Debug, Component, Deref, DerefMut)]
struct TipTimer(Timer);

// ============ 上下文感知提示 ============

// 添加清理过期提示的系统
fn cleanup_expired_tips(
    mut commands: Commands,
    mut timers: Query<(Entity, &mut TipTimer), With<TipTimer>>,
    time: Res<Time>,
) {
    for (entity, mut timer) in timers.iter_mut() {
        timer.tick(time.delta());
        if timer.just_finished() {
            commands.entity(entity).despawn();
        }
    }
}

pub fn check_and_show_contextual_tips(
    mut commands: Commands,
    ui_assets: Res<crate::bus_puzzle::UIAssets>,
    game_state: Res<GameState>,
    passengers: Query<&crate::bus_puzzle::PathfindingAgent>,
    segments: Query<&crate::bus_puzzle::RouteSegment>,
    mut last_tip_time: Local<f32>,
    time: Res<Time>,
    current_language: Res<CurrentLanguage>,
) {
    // 每5秒检查一次，避免提示过于频繁
    if time.elapsed_secs() - *last_tip_time < 5.0 {
        return;
    }

    let gave_up_count = passengers
        .iter()
        .filter(|agent| matches!(agent.state, crate::bus_puzzle::AgentState::GaveUp))
        .count();

    let waiting_count = passengers
        .iter()
        .filter(|agent| matches!(agent.state, crate::bus_puzzle::AgentState::WaitingAtStation))
        .count();

    let segments_count = segments.iter().count();

    // 根据游戏状态显示相应提示
    if gave_up_count > 0 && gave_up_count < 3 {
        show_contextual_tip(
            &mut commands,
            &ui_assets,
            &get_text_with_args(
                &PASSENGERS_GAVE_UP_WARNING,
                current_language.language,
                &[
                    &game_state.total_cost.to_string(),
                    &gave_up_count.to_string(),
                ],
            ),
            TipType::Warning,
            4.0,
        );
        *last_tip_time = time.elapsed_secs();
    } else if waiting_count > 5 {
        show_contextual_tip(
            &mut commands,
            &ui_assets,
            &get_text(&PASSENGERS_WAITING_HINT, current_language.language),
            TipType::Strategy,
            4.0,
        );
        *last_tip_time = time.elapsed_secs();
    } else if segments_count > 0 && waiting_count > 0 && passengers.iter().count() < 2 {
        show_contextual_tip(
            &mut commands,
            &ui_assets,
            &get_text(&BUS_ROUTES_READY_INFO, current_language.language),
            TipType::Strategy,
            3.0,
        );
        *last_tip_time = time.elapsed_secs();
    }

    // 预算警告
    if let Some(level_data) = &game_state.current_level {
        for objective in &level_data.objectives {
            if let crate::bus_puzzle::ObjectiveType::MaxCost(max_cost) = &objective.condition_type {
                let cost_ratio = game_state.total_cost as f32 / *max_cost as f32;
                if cost_ratio > 0.8 && cost_ratio <= 1.0 {
                    show_contextual_tip(
                        &mut commands,
                        &ui_assets,
                        &get_text_with_args(
                            &BUDGET_WARNING,
                            current_language.language,
                            &[&game_state.total_cost.to_string(), &max_cost.to_string()],
                        ),
                        TipType::Warning,
                        3.0,
                    );
                    *last_tip_time = time.elapsed_secs();
                }
            }
        }
    }
}
