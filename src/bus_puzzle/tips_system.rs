// src/bus_puzzle/tips_system.rs - 游戏提示系统

use crate::bus_puzzle::{
    get_text, get_text_with_args, CurrentLanguage, GameState, GameStateEnum, Language, LevelData,
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
                right: Px(70.0),                // 调整位置，避免与右侧面板重叠
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

// 添加清理过期提示的系统（安全删除）
fn cleanup_expired_tips(
    mut commands: Commands,
    mut timers: Query<(Entity, &mut TipTimer), With<TipTimer>>,
    time: Res<Time>,
) {
    for (entity, mut timer) in timers.iter_mut() {
        timer.tick(time.delta());
        if timer.just_finished() {
            // 安全删除：递归删除实体及其子实体
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

// ============ 本地化的游戏提示结构 ============

#[derive(Debug, Clone)]
pub struct LocalizedGameTip {
    pub tip_type: TipType,
    pub title_zh: String,   // 中文标题
    pub title_en: String,   // 英文标题
    pub content_zh: String, // 中文内容
    pub content_en: String, // 英文内容
    pub icon: String,
    pub color: Color,
}

impl LocalizedGameTip {
    pub fn new(
        tip_type: TipType,
        title_zh: &str,
        title_en: &str,
        content_zh: &str,
        content_en: &str,
        icon: &str,
        color: Color,
    ) -> Self {
        Self {
            tip_type,
            title_zh: title_zh.to_string(),
            title_en: title_en.to_string(),
            content_zh: content_zh.to_string(),
            content_en: content_en.to_string(),
            icon: icon.to_string(),
            color,
        }
    }

    pub fn get_title(&self, language: Language) -> &str {
        match language {
            Language::Chinese => &self.title_zh,
            Language::English => &self.title_en,
        }
    }

    pub fn get_content(&self, language: Language) -> &str {
        match language {
            Language::Chinese => &self.content_zh,
            Language::English => &self.content_en,
        }
    }
}

// ============ 本地化的 Tips 管理器 ============

#[derive(Resource, Default)]
pub struct LocalizedTipsManager {
    pub current_tips: Vec<LocalizedGameTip>,
    pub is_expanded: bool,
    pub last_level_id: String,
}

impl LocalizedTipsManager {
    pub fn generate_localized_tips_for_level(&mut self, level_data: &LevelData) {
        self.current_tips.clear();

        match level_data.id.as_str() {
            "tutorial_01" => {
                self.current_tips = vec![
                    LocalizedGameTip::new(
                        TipType::LevelGoal,
                        "关卡目标",
                        "Level Goal",
                        "连接A站和B站，让红色乘客能够到达目的地",
                        "Connect Station A and B, allowing red passengers to reach their destination",
                        "🎯",
                        Color::srgb(0.2, 0.8, 0.2),
                    ),
                    LocalizedGameTip::new(
                        TipType::Strategy,
                        "建设策略",
                        "Building Strategy",
                        "使用直线段是最经济的选择，只需要简单的直线连接即可",
                        "Using straight segments is the most economical choice, only simple straight connections are needed",
                        "💡",
                        Color::srgb(0.9, 0.7, 0.2),
                    ),
                    LocalizedGameTip::new(
                        TipType::Controls,
                        "操作提示",
                        "Controls",
                        "左键放置路段，右键旋转方向，Delete键删除路段",
                        "Left click to place segments, right click to rotate, Delete key to remove segments",
                        "🎮",
                        Color::srgb(0.2, 0.6, 0.9),
                    ),
                ];
            }
            "level_02_transfer" => {
                self.current_tips = vec![
                    LocalizedGameTip::new(
                        TipType::LevelGoal,
                        "关卡目标",
                        "Level Goal",
                        "学会使用换乘：乘客需要在中转站换乘前往不同目的地",
                        "Learn to use transfers: passengers need to transfer at hub stations to reach different destinations",
                        "🎯",
                        Color::srgb(0.2, 0.8, 0.2),
                    ),
                    LocalizedGameTip::new(
                        TipType::Strategy,
                        "换乘策略",
                        "Transfer Strategy",
                        "规划两条路线：A站→中转站，中转站→B站/C站，让乘客在中转站换乘",
                        "Plan two routes: Station A → Transfer Hub, Transfer Hub → Station B/C, let passengers transfer at the hub",
                        "🔄",
                        Color::srgb(0.9, 0.7, 0.2),
                    ),
                    LocalizedGameTip::new(
                        TipType::Warning,
                        "避免绕路",
                        "Avoid Detours",
                        "避免让乘客换乘太多次，每次换乘都会增加等待时间",
                        "Avoid making passengers transfer too many times, each transfer increases waiting time",
                        "⚠️",
                        Color::srgb(0.9, 0.3, 0.2),
                    ),
                ];
            }
            "level_03_multiple_routes" => {
                self.current_tips = vec![
                    LocalizedGameTip::new(
                        TipType::LevelGoal,
                        "关卡目标",
                        "Level Goal",
                        "管理多条路线，优化整个交通网络的效率",
                        "Manage multiple routes and optimize the entire transportation network efficiency",
                        "🎯",
                        Color::srgb(0.2, 0.8, 0.2),
                    ),
                    LocalizedGameTip::new(
                        TipType::Strategy,
                        "网络规划",
                        "Network Planning",
                        "善用中央枢纽作为换乘点，可以减少总的路线段数量",
                        "Make good use of the central hub as a transfer point to reduce the total number of route segments",
                        "🗺️",
                        Color::srgb(0.9, 0.7, 0.2),
                    ),
                    LocalizedGameTip::new(
                        TipType::Warning,
                        "预算控制",
                        "Budget Control",
                        "注意控制成本！优先使用便宜的直线段，谨慎使用昂贵的桥梁",
                        "Pay attention to cost control! Prioritize cheap straight segments, use expensive bridges carefully",
                        "💰",
                        Color::srgb(0.9, 0.3, 0.2),
                    ),
                    LocalizedGameTip::new(
                        TipType::Strategy,
                        "跨河策略",
                        "River Crossing Strategy",
                        "河流阻挡了直接路径，使用桥梁路段跨越水面",
                        "Rivers block direct paths, use bridge segments to cross water",
                        "🌉",
                        Color::srgb(0.2, 0.7, 0.9),
                    ),
                ];
            }
            "level_04_time_pressure" => {
                self.current_tips = vec![
                    LocalizedGameTip::new(
                        TipType::LevelGoal,
                        "关卡目标",
                        "Level Goal",
                        "在60秒内完成网络建设，快速响应是关键",
                        "Complete network construction within 60 seconds, quick response is key",
                        "⏰",
                        Color::srgb(0.2, 0.8, 0.2),
                    ),
                    LocalizedGameTip::new(
                        TipType::Strategy,
                        "快速建设",
                        "Fast Construction",
                        "优先建设主要路线，不要追求完美的网络设计",
                        "Prioritize building main routes, don't pursue perfect network design",
                        "⚡",
                        Color::srgb(0.9, 0.7, 0.2),
                    ),
                    LocalizedGameTip::new(
                        TipType::Warning,
                        "时间压力",
                        "Time Pressure",
                        "乘客耐心较短，延误可能导致大量乘客放弃！",
                        "Passengers have short patience, delays may cause many passengers to give up!",
                        "🚨",
                        Color::srgb(0.9, 0.3, 0.2),
                    ),
                    LocalizedGameTip::new(
                        TipType::Strategy,
                        "穿山隧道",
                        "Mountain Tunnel",
                        "山脉阻挡了路径，使用隧道路段穿越山区",
                        "Mountains block the path, use tunnel segments to cross mountainous areas",
                        "🏔️",
                        Color::srgb(0.6, 0.4, 0.2),
                    ),
                ];
            }
            _ => {
                // 默认通用提示
                self.current_tips = vec![
                    LocalizedGameTip::new(
                        TipType::Strategy,
                        "基本策略",
                        "Basic Strategy",
                        "观察乘客需求，规划最短有效路径",
                        "Observe passenger demands, plan the shortest effective paths",
                        "💡",
                        Color::srgb(0.9, 0.7, 0.2),
                    ),
                    LocalizedGameTip::new(
                        TipType::Controls,
                        "操作提示",
                        "Control Tips",
                        "F4发现公交路线，F6查看乘客状态",
                        "F4 to discover bus routes, F6 to view passenger status",
                        "🎮",
                        Color::srgb(0.2, 0.6, 0.9),
                    ),
                ];
            }
        }

        self.last_level_id = level_data.id.clone();
        info!("生成 {} 条本地化关卡提示", self.current_tips.len());
    }

    pub fn get_localized_segment_tips(&self) -> Vec<LocalizedGameTip> {
        vec![
            LocalizedGameTip::new(
                TipType::Strategy,
                "路段选择",
                "Segment Selection",
                "直线段(成本1) < 转弯段(成本2) < T型(成本3) < 十字(成本4)",
                "Straight(Cost 1) < Curve(Cost 2) < T-Split(Cost 3) < Cross(Cost 4)",
                "🛤️",
                Color::srgb(0.7, 0.7, 0.7),
            ),
            LocalizedGameTip::new(
                TipType::Strategy,
                "特殊路段",
                "Special Segments",
                "桥梁跨越水面(成本5)，隧道穿越山脉(成本6)",
                "Bridge crosses water(Cost 5), Tunnel crosses mountains(Cost 6)",
                "🌉",
                Color::srgb(0.2, 0.7, 0.9),
            ),
        ]
    }
}

// ============ 本地化的 Tips UI 创建 ============

pub fn create_localized_tips_panel(
    parent: &mut ChildSpawnerCommands,
    ui_assets: &crate::bus_puzzle::UIAssets,
    tips_manager: &LocalizedTipsManager,
    current_language: &CurrentLanguage,
) {
    // 标题栏（本地化）
    parent
        .spawn((Node {
            width: Percent(100.0),
            height: Px(24.0),
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            margin: UiRect::bottom(Px(6.0)),
            ..default()
        },))
        .with_children(|parent| {
            let title_text = match current_language.language {
                Language::Chinese => "💡 关卡提示",
                Language::English => "💡 Level Tips",
            };

            parent.spawn((
                Text::new(title_text),
                TextFont {
                    font: ui_assets.font.clone(),
                    font_size: 15.0,
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.9, 0.3)),
            ));

            let toggle_text = match current_language.language {
                Language::Chinese => "F1 切换",
                Language::English => "F1 Toggle",
            };

            parent.spawn((
                Text::new(toggle_text),
                TextFont {
                    font: ui_assets.font.clone(),
                    font_size: 10.0,
                    ..default()
                },
                TextColor(Color::srgb(0.7, 0.7, 0.7)),
            ));
        });

    // 提示内容滚动区域
    parent
        .spawn((
            Node {
                width: Percent(100.0),
                height: Px(280.0),
                flex_direction: FlexDirection::Column,
                row_gap: Px(8.0),
                overflow: Overflow::clip_y(),
                ..default()
            },
            Visibility::Visible,
        ))
        .with_children(|parent| {
            // 当前关卡提示
            for tip in &tips_manager.current_tips {
                create_localized_tip_item(parent, ui_assets, tip, current_language.language);
            }

            // 分隔线
            if !tips_manager.current_tips.is_empty() {
                parent.spawn((
                    Node {
                        width: Percent(100.0),
                        height: Px(1.0),
                        margin: UiRect::vertical(Px(6.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.4, 0.4, 0.4)),
                ));
            }

            // 通用路段提示
            let segment_tips = tips_manager.get_localized_segment_tips();
            for tip in &segment_tips {
                create_localized_tip_item(parent, ui_assets, tip, current_language.language);
            }
        });
}

fn create_localized_tip_item(
    parent: &mut ChildSpawnerCommands,
    ui_assets: &crate::bus_puzzle::UIAssets,
    tip: &LocalizedGameTip,
    language: Language,
) {
    parent
        .spawn((
            Node {
                width: Percent(100.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Px(6.0)),
                row_gap: Px(3.0),
                border: UiRect::all(Px(1.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.2, 0.2, 0.3, 0.7)),
            BorderColor(tip.color),
            LocalizedTipText {
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
                    column_gap: Px(4.0),
                    ..default()
                },))
                .with_children(|parent| {
                    // 图标
                    parent.spawn((
                        Text::new(&tip.icon),
                        TextFont {
                            font: ui_assets.font.clone(),
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(tip.color),
                    ));

                    // 标题（本地化）
                    parent.spawn((
                        Text::new(tip.get_title(language)),
                        TextFont {
                            font: ui_assets.font.clone(),
                            font_size: 12.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                        Node {
                            flex_grow: 1.0,
                            ..default()
                        },
                    ));
                });

            // 内容（本地化）
            parent.spawn((
                Text::new(tip.get_content(language)),
                TextFont {
                    font: ui_assets.font.clone(),
                    font_size: 10.0,
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

// ============ 语言切换响应系统 ============

#[derive(Component)]
#[allow(dead_code)]
pub struct LocalizedTipText {
    pub tip_type: TipType,
}

#[derive(Component)]
pub struct LocalizedTipsPanel;

// 响应语言切换的更新系统（修复层级关系问题）
fn update_tips_panel_language(
    current_language: Res<CurrentLanguage>,
    tips_manager: Res<LocalizedTipsManager>,
    mut commands: Commands,
    ui_assets: Res<crate::bus_puzzle::UIAssets>,
    // 修复：查找使用 TipsPanel 组件的实体
    existing_panels: Query<Entity, With<TipsPanel>>,
) {
    // 如果语言发生变化，重新创建整个面板内容
    if current_language.is_changed() {
        for entity in existing_panels.iter() {
            // 修复：只清除子实体，保留面板本身，避免层级关系警告
            commands.entity(entity).despawn_related::<Children>();

            // 重新创建面板内容
            commands.entity(entity).with_children(|parent| {
                create_localized_tips_panel(parent, &ui_assets, &tips_manager, &current_language);
            });
        }

        info!("Tips面板语言已更新为: {:?}", current_language.language);
    }
}

// ============ 完整的本地化 Tips 系统插件（修复版） ============

pub struct LocalizedTipsSystemPlugin;

impl Plugin for LocalizedTipsSystemPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LocalizedTipsManager>().add_systems(
            Update,
            (
                update_localized_tips_for_level,
                handle_tips_panel_toggle,   // F1键切换
                update_tips_display,        // 更新可见性
                update_tips_panel_language, // 语言切换响应
                cleanup_expired_tips,
                debug_tips_panel_state, // F2调试信息
            )
                .run_if(in_state(GameStateEnum::Playing)),
        );
    }
}

// ============ 修复后的 Tips 更新系统 ============

fn update_localized_tips_for_level(
    mut tips_manager: ResMut<LocalizedTipsManager>,
    game_state: Res<GameState>,
    mut commands: Commands,
    ui_assets: Res<crate::bus_puzzle::UIAssets>,
    current_language: Res<CurrentLanguage>,
    existing_panels: Query<Entity, With<TipsPanel>>,
) {
    if let Some(level_data) = &game_state.current_level {
        // 只在关卡改变时更新提示
        if tips_manager.last_level_id != level_data.id {
            tips_manager.generate_localized_tips_for_level(level_data);
            // 确保新关卡开始时面板是展开的
            tips_manager.is_expanded = true;

            // 修复：关卡切换时重新创建面板内容
            for entity in existing_panels.iter() {
                commands.entity(entity).despawn_related::<Children>();
                commands.entity(entity).with_children(|parent| {
                    create_localized_tips_panel(
                        parent,
                        &ui_assets,
                        &tips_manager,
                        &current_language,
                    );
                });
            }

            info!("关卡切换：{} - 已更新Tips内容", level_data.id);
        }
    }
}

fn handle_tips_panel_toggle(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut tips_manager: ResMut<LocalizedTipsManager>,
    mut commands: Commands,
    ui_assets: Res<crate::bus_puzzle::UIAssets>,
    current_language: Res<CurrentLanguage>,
    existing_panels: Query<Entity, With<TipsPanel>>,
) {
    if keyboard_input.just_pressed(KeyCode::F1) {
        tips_manager.is_expanded = !tips_manager.is_expanded;

        // 修复：F1切换时重新创建面板内容，确保完全显示/隐藏
        if tips_manager.is_expanded {
            // 显示时：重新创建内容
            for entity in existing_panels.iter() {
                commands.entity(entity).despawn_related::<Children>();
                commands.entity(entity).with_children(|parent| {
                    create_localized_tips_panel(
                        parent,
                        &ui_assets,
                        &tips_manager,
                        &current_language,
                    );
                });
            }
            info!("F1 Tips面板展开，内容已重新创建");
        } else {
            // 隐藏时：清除所有内容
            for entity in existing_panels.iter() {
                commands.entity(entity).despawn_related::<Children>();
            }
            info!("F1 Tips面板隐藏，内容已清除");
        }
    }
}

fn update_tips_display(
    tips_manager: Res<LocalizedTipsManager>,
    mut tips_panels: Query<&mut Visibility, With<TipsPanel>>,
) {
    // 简化：只控制面板本身的可见性，内容通过F1切换时的重建来管理
    let target_visibility = if tips_manager.is_expanded {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };

    for mut visibility in tips_panels.iter_mut() {
        *visibility = target_visibility;
    }
}

// ============ 调试系统 ============

fn debug_tips_panel_state(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    tips_manager: Res<LocalizedTipsManager>,
    tips_panels: Query<(Entity, &Visibility), With<TipsPanel>>,
) {
    if keyboard_input.just_pressed(KeyCode::F2) {
        info!("=== Tips面板调试信息 ===");
        info!("Tips管理器状态: is_expanded = {}", tips_manager.is_expanded);
        info!("Tips提示数量: {}", tips_manager.current_tips.len());
        info!("当前关卡ID: {}", tips_manager.last_level_id);

        for (entity, visibility) in tips_panels.iter() {
            info!("Tips面板实体 {:?}: 可见性 = {:?}", entity, visibility);
        }

        if tips_panels.is_empty() {
            warn!("❌ 没有找到Tips面板实体！");
        }
    }
}
