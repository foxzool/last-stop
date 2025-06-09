// src/bus_puzzle/ui_audio.rs

// 使用相对路径引用同模块下的其他文件
use super::{
    ease_out_back, format_time, get_text, get_text_with_args, localized_text,
    localized_text_with_args, AgentState, AudioAssets, CostText, CurrentLanguage, GameState,
    GameStateEnum, InventoryCountText, InventorySlot, Language, LanguageChangedEvent,
    LevelCompletedEvent, LevelData, LevelManager, LocalizedText, LocalizedTextComponent,
    ObjectiveCompletedEvent, ObjectiveCondition, ObjectiveType, PassengerColor, PassengerCountText,
    PathfindingAgent, RouteSegmentType, ScoreText, SegmentPlacedEvent, SegmentRemovedEvent,
    TimerText, TipsPanel, UIElement, ALL_LEVELS_COMPLETE, ARRIVED, COMPLETION_TIME,
    CONGRATULATIONS, COST, DONT_GIVE_UP, FAILURE_REASON, FINAL_SCORE, GAME_DURATION, GAME_PAUSED,
    GAME_STATISTICS, GAME_TITLE, INVENTORY_SLOT_SIZE, LEVEL_COMPLETE, MAIN_MENU, MISSION_FAILED,
    NEXT_LEVEL, OBJECTIVES, PASSENGERS, PASSENGERS_GAVE_UP, PASSENGER_STATUS, PAUSE, QUIT_GAME,
    RESTART_LEVEL, RESUME_GAME, RETRY, ROUTE_SEGMENTS, SCORE, SCORE_BREAKDOWN, SCORE_EARNED,
    START_GAME, THANK_YOU, TIME, TOTAL_COST, WAITING,
};
use crate::bus_puzzle::{check_and_show_contextual_tips, create_tips_panel, TipsManager};
use bevy::{
    audio::{PlaybackMode, Volume},
    platform::collections::HashMap,
    prelude::*,
    ui::Val::*,
};

// ============ UI 组件 ============

#[derive(Component)]
pub struct MainMenuUI;

#[derive(Component)]
pub struct GameplayUI;

#[derive(Component)]
pub struct PauseMenuUI;

#[derive(Component)]
pub struct LevelCompleteUI;

#[derive(Component)]
pub struct GameOverUI;

#[derive(Component)]
#[allow(dead_code)]
pub struct ObjectiveUI {
    pub objective_index: usize,
}

#[derive(Component)]
pub struct ProgressBar {
    pub current_value: f32,
    pub max_value: f32,
    pub bar_type: ProgressBarType,
}

#[derive(Clone, PartialEq)]
#[allow(dead_code)]
pub enum ProgressBarType {
    ObjectiveProgress,
    TimeRemaining,
    BudgetUsed,
}

// 新增：乘客统计相关组件
#[derive(Component)]
pub struct PassengerStatsPanel;

#[derive(Component)]
pub struct PassengerColorCountText {
    pub color: PassengerColor,
}

#[derive(Component)]
#[allow(dead_code)]
pub struct PassengerColorIcon {
    pub color: PassengerColor,
}

#[derive(Component)]
pub struct ProgressBarFill;

#[derive(Component)]
pub struct AnimatedUI {
    pub animation_type: UIAnimation,
    pub duration: f32,
    pub elapsed: f32,
    pub start_value: f32,
    pub target_value: f32,
}

#[derive(Clone)]
#[allow(dead_code)]
pub enum UIAnimation {
    FadeIn,
    FadeOut,
    ScaleUp,
    ScaleDown,
    SlideIn(Vec2),
    Bounce,
}

#[derive(Component)]
pub struct ButtonComponent {
    pub button_type: ButtonType,
    pub is_hovered: bool,
    pub is_pressed: bool,
}

// ============ 资源定义 ============

#[derive(Resource)]
#[allow(dead_code)]
pub struct UIAssets {
    pub font: Handle<Font>,
    pub button_texture: Handle<Image>,
    pub panel_texture: Handle<Image>,
    pub progress_bar_bg: Handle<Image>,
    pub progress_bar_fill: Handle<Image>,
    pub segment_icons: HashMap<RouteSegmentType, Handle<Image>>,
    pub passenger_icons: HashMap<PassengerColor, Handle<Image>>,
}

#[derive(Resource, Default)]
pub struct LevelCompleteData {
    pub final_score: u32,
    pub completion_time: f32,
}

#[derive(Resource, Default)]
pub struct GameOverData {
    pub reason: String,
    pub final_score: u32,
    pub game_time: f32,
    pub passengers_gave_up: u32,
}

#[derive(Resource)]
pub struct AudioSettings {
    pub master_volume: f32,
    pub music_volume: f32,
    pub sfx_volume: f32,
    pub is_muted: bool,
}

// ============ 插件系统 ============

pub struct GameUIPlugin;

impl Plugin for GameUIPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(AudioSettings {
            master_volume: 1.0,
            music_volume: 0.7,
            sfx_volume: 0.8,
            is_muted: false,
        })
            .insert_resource(LevelCompleteData::default())
            .insert_resource(GameOverData::default())
            .add_systems(Startup, (load_ui_assets, load_audio_assets))
            .add_systems(OnEnter(GameStateEnum::MainMenu), setup_main_menu)
            .add_systems(OnEnter(GameStateEnum::Playing), setup_gameplay_ui)
            .add_systems(OnEnter(GameStateEnum::Paused), setup_pause_menu)
            .add_systems(
                OnEnter(GameStateEnum::LevelComplete),
                setup_level_complete_ui,
            )
            .add_systems(OnEnter(GameStateEnum::GameOver), setup_game_over_ui)
            .add_systems(OnExit(GameStateEnum::MainMenu), cleanup_main_menu)
            .add_systems(OnExit(GameStateEnum::Playing), cleanup_gameplay_ui)
            .add_systems(OnExit(GameStateEnum::Paused), cleanup_pause_menu)
            .add_systems(
                OnExit(GameStateEnum::LevelComplete),
                cleanup_level_complete_ui,
            )
            .add_systems(OnExit(GameStateEnum::GameOver), cleanup_game_over_ui)
            .add_systems(
                Update,
                (
                    handle_button_interactions,
                    update_ui_animations,
                    update_gameplay_ui_values,
                    update_progress_bars,
                    update_passenger_stats_ui, // 新增：更新乘客统计UI
                    handle_audio_events,
                    update_background_music,
                    capture_level_complete_data, // 新增：捕获关卡完成数据
                    check_and_show_contextual_tips, // 新增：上下文感知提示
                    update_inventory_selection_state, // 新增：更新库存选中状态
                )
                    .run_if(in_state(GameStateEnum::Playing)),
            )
            .add_systems(
                Update,
                (handle_menu_buttons, handle_button_interactions)
                    .run_if(in_state(GameStateEnum::MainMenu)),
            )
            .add_systems(
                Update,
                (
                    handle_pause_input,
                    handle_button_interactions.before(handle_pause_buttons), // 修复：确保交互处理在按钮逻辑之前
                    handle_pause_buttons,
                    debug_pause_menu_state, // 调试系统
                ).run_if(in_state(GameStateEnum::Paused)),
            )
            .add_systems(
                Update,
                (handle_level_complete_buttons, handle_button_interactions)
                    .run_if(in_state(GameStateEnum::LevelComplete)),
            )
            .add_systems(
                Update,
                (handle_game_over_buttons, handle_button_interactions)
                    .run_if(in_state(GameStateEnum::GameOver)),
            )
            .add_systems(Update, handle_language_toggle_button);
    }
}

// ============ 资源加载 ============

fn load_ui_assets(mut commands: Commands, asset_server: Res<AssetServer>) {
    info!("加载 UI 资源");
    let mut segment_icons = HashMap::new();
    segment_icons.insert(
        RouteSegmentType::Straight,
        asset_server.load("textures/routes/straight.png"),
    );
    segment_icons.insert(
        RouteSegmentType::Curve,
        asset_server.load("textures/routes/curve.png"),
    );
    segment_icons.insert(
        RouteSegmentType::TSplit,
        asset_server.load("textures/routes/t_split.png"),
    );
    segment_icons.insert(
        RouteSegmentType::Cross,
        asset_server.load("textures/routes/cross.png"),
    );
    segment_icons.insert(
        RouteSegmentType::Bridge,
        asset_server.load("textures/routes/bridge.png"),
    );
    segment_icons.insert(
        RouteSegmentType::Tunnel,
        asset_server.load("textures/routes/tunnel.png"),
    );

    let mut passenger_icons = HashMap::new();
    passenger_icons.insert(
        PassengerColor::Red,
        asset_server.load("textures/passengers/red.png"),
    );
    passenger_icons.insert(
        PassengerColor::Blue,
        asset_server.load("textures/passengers/blue.png"),
    );
    passenger_icons.insert(
        PassengerColor::Green,
        asset_server.load("textures/passengers/green.png"),
    );
    passenger_icons.insert(
        PassengerColor::Yellow,
        asset_server.load("textures/passengers/yellow.png"),
    );
    passenger_icons.insert(
        PassengerColor::Purple,
        asset_server.load("textures/passengers/purple.png"),
    );
    passenger_icons.insert(
        PassengerColor::Orange,
        asset_server.load("textures/passengers/orange.png"),
    );

    // 尝试加载UI纹理，如果不存在会加载失败但不会崩溃
    let button_texture = asset_server.load("ui/button.png");
    let panel_texture = asset_server.load("ui/panel.png");
    let progress_bar_bg = asset_server.load("ui/progress_bg.png");
    let progress_bar_fill = asset_server.load("ui/progress_fill.png");

    info!("UI纹理加载完成（如果文件不存在会显示错误但不影响游戏运行）");

    commands.insert_resource(UIAssets {
        font: asset_server.load("fonts/quan.ttf"),
        button_texture,
        panel_texture,
        progress_bar_bg,
        progress_bar_fill,
        segment_icons,
        passenger_icons,
    });
}

fn load_audio_assets(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(AudioAssets {
        background_music: asset_server.load("audio/background_music.ogg"),
        segment_place_sound: asset_server.load("audio/place_segment.ogg"),
        segment_remove_sound: asset_server.load("audio/remove_segment.ogg"),
        passenger_arrive_sound: asset_server.load("audio/passenger_arrive.ogg"),
        objective_complete_sound: asset_server.load("audio/objective_complete.ogg"),
        level_complete_sound: asset_server.load("audio/level_complete.ogg"),
        button_click_sound: asset_server.load("audio/button_click.ogg"),
        error_sound: asset_server.load("audio/error.ogg"),
    });
}

// ============ UI 设置系统 ============

fn setup_main_menu(
    mut commands: Commands,
    ui_assets: Res<UIAssets>,
    current_language: Res<CurrentLanguage>,
) {
    commands
        .spawn((
            Node {
                width: Percent(100.0),
                height: Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(Color::srgb(0.1, 0.1, 0.2)),
            MainMenuUI,
        ))
        .with_children(|parent| {
            // 游戏标题 - 使用本地化
            let (localized_title, title_text) = localized_text(&GAME_TITLE);
            parent.spawn((
                title_text,
                TextFont {
                    font: ui_assets.font.clone(),
                    font_size: 60.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                Node {
                    margin: UiRect::bottom(Px(50.0)),
                    ..default()
                },
                localized_title,
            ));

            // 开始游戏按钮
            parent
                .spawn((
                    Button,
                    Node {
                        width: Px(200.0),
                        height: Px(60.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        margin: UiRect::all(Px(10.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.2, 0.6, 0.2)),
                    ButtonComponent {
                        button_type: ButtonType::StartGame,
                        is_hovered: false,
                        is_pressed: false,
                    },
                ))
                .with_children(|parent| {
                    let (localized_start, start_text) = localized_text(&START_GAME);
                    parent.spawn((
                        start_text,
                        TextFont {
                            font: ui_assets.font.clone(),
                            font_size: 20.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                        localized_start,
                    ));
                });

            // 退出游戏按钮
            parent
                .spawn((
                    Button,
                    Node {
                        width: Px(200.0),
                        height: Px(60.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        margin: UiRect::all(Px(10.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.6, 0.2, 0.2)),
                    ButtonComponent {
                        button_type: ButtonType::QuitGame,
                        is_hovered: false,
                        is_pressed: false,
                    },
                ))
                .with_children(|parent| {
                    let (localized_quit, quit_text) = localized_text(&QUIT_GAME);
                    parent.spawn((
                        quit_text,
                        TextFont {
                            font: ui_assets.font.clone(),
                            font_size: 20.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                        localized_quit,
                    ));
                });

            // 语言切换按钮
            parent
                .spawn((
                    Button,
                    Node {
                        width: Px(150.0),
                        height: Px(40.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        margin: UiRect::all(Px(10.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.3, 0.3, 0.5)),
                    ButtonComponent {
                        button_type: ButtonType::ToggleLanguage,
                        is_hovered: false,
                        is_pressed: false,
                    },
                ))
                .with_children(|parent| {
                    // 显示当前语言的另一种语言（切换目标）
                    let next_language = match current_language.language {
                        Language::English => "中文",
                        Language::Chinese => "English",
                    };

                    parent.spawn((
                        Text::new(next_language),
                        TextFont {
                            font: ui_assets.font.clone(),
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                        LanguageToggleText, // 特殊组件，需要单独处理
                    ));
                });
        });
}

fn setup_gameplay_ui(
    mut commands: Commands,
    ui_assets: Res<UIAssets>,
    game_state: Res<GameState>,
    tips_manager: Res<TipsManager>,
    current_language: Res<CurrentLanguage>,
) {
    // 顶部状态栏
    commands
        .spawn((
            Node {
                width: Percent(100.0),
                height: Px(80.0),
                position_type: PositionType::Absolute,
                top: Px(0.0),
                left: Px(0.0),
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                padding: UiRect::all(Px(20.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
            ZIndex(1000),
            GameplayUI,
        ))
        .with_children(|parent| {
            // 左侧信息组
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Px(20.0),
                    ..default()
                })
                .with_children(|parent| {
                    let score = spawn_localized_score_text(parent, &ui_assets, &SCORE, 20.0);
                    parent.commands().entity(score).insert(ScoreText);
                    let time = spawn_localized_score_text(parent, &ui_assets, &TIME, 20.0);
                    parent.commands().entity(time).insert(TimerText);
                    let cost = spawn_localized_score_text(parent, &ui_assets, &COST, 20.0);
                    parent.commands().entity(cost).insert(CostText);
                    let passengers =
                        spawn_localized_score_text(parent, &ui_assets, &PASSENGERS, 20.0);
                    parent
                        .commands()
                        .entity(passengers)
                        .insert(PassengerCountText);

                    // 新增：目标完成进度条
                    parent
                        .spawn((Node {
                            width: Px(120.0),
                            height: Px(20.0),
                            justify_content: JustifyContent::FlexStart,
                            align_items: AlignItems::Center,
                            ..default()
                        },))
                        .with_children(|parent| {
                            // 进度条背景（临时使用背景色）
                            parent
                                .spawn((
                                    Node {
                                        width: Px(120.0),
                                        height: Px(8.0),
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgb(0.3, 0.3, 0.3)),
                                ))
                                .with_children(|parent| {
                                    // 进度条填充（临时使用背景色）
                                    parent.spawn((
                                        Node {
                                            width: Percent(0.0), // 初始为0%
                                            height: Percent(100.0),
                                            ..default()
                                        },
                                        BackgroundColor(Color::srgb(0.0, 0.8, 0.0)),
                                        ProgressBar {
                                            current_value: 0.0,
                                            max_value: 100.0,
                                            bar_type: ProgressBarType::ObjectiveProgress,
                                        },
                                        ProgressBarFill,
                                    ));
                                });

                            // 进度条标签
                            parent.spawn((
                                Text::new(get_text(&OBJECTIVES, current_language.language)),
                                TextFont {
                                    font: ui_assets.font.clone(),
                                    font_size: 12.0,
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                                Node {
                                    margin: UiRect::left(Px(8.0)),
                                    ..default()
                                },
                            ));
                        });
                });

            parent
                .spawn((
                    Button,
                    Node {
                        width: Px(100.0),
                        height: Px(40.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.3, 0.3, 0.3)),
                    ButtonComponent {
                        button_type: ButtonType::PauseGame,
                        is_hovered: false,
                        is_pressed: false,
                    },
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new(get_text(&PAUSE, current_language.language)),
                        TextFont {
                            font: ui_assets.font.clone(),
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });
        });

    // 左侧库存面板（调整位置避免与Tips面板重叠）
    commands
        .spawn((
            Node {
                width: Px(110.0),  // 稍微缩小
                height: Px(350.0), // 设置固定高度
                position_type: PositionType::Absolute,
                left: Px(10.0),
                top: Px(90.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Px(8.0)), // 减少内边距
                row_gap: Px(8.0),              // 减少间距
                ..default()
            },
            // 临时回退到背景色
            BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 0.7)),
            ZIndex(50),
            GameplayUI,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new(get_text(&ROUTE_SEGMENTS, current_language.language)),
                TextFont {
                    font: ui_assets.font.clone(),
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));

            let segment_types = [
                RouteSegmentType::Straight,
                RouteSegmentType::Curve,
                RouteSegmentType::TSplit,
                RouteSegmentType::Cross,
                RouteSegmentType::Bridge,
                RouteSegmentType::Tunnel,
            ];

            for (index, segment_type) in segment_types.iter().enumerate() {
                let available_count = game_state
                    .player_inventory
                    .get(segment_type)
                    .copied()
                    .unwrap_or(0);

                parent
                    .spawn((
                        Button,
                        Node {
                            width: Px(INVENTORY_SLOT_SIZE),
                            height: Px(INVENTORY_SLOT_SIZE),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border: UiRect::all(Px(2.0)),
                            ..default()
                        },
                        BackgroundColor(if available_count > 0 {
                            Color::srgb(0.4, 0.4, 0.4)
                        } else {
                            Color::srgb(0.2, 0.2, 0.2)
                        }),
                        BorderColor(Color::WHITE),
                        ButtonComponent {
                            button_type: ButtonType::InventorySlot(*segment_type),
                            is_hovered: false,
                            is_pressed: false,
                        },
                        InventorySlot {
                            segment_type: Some(*segment_type),
                            slot_index: index,
                            available_count,
                        },
                        UIElement,
                    ))
                    .with_children(|parent| {
                        if let Some(icon) = ui_assets.segment_icons.get(segment_type) {
                            parent.spawn((
                                ImageNode::new(icon.clone()),
                                Node {
                                    width: Px(35.0), // 缩小图标
                                    height: Px(35.0),
                                    ..default()
                                },
                            ));
                        }

                        parent.spawn((
                            Text::new(format!("{}", available_count)),
                            TextFont {
                                font: ui_assets.font.clone(),
                                font_size: 14.0,
                                ..default()
                            },
                            TextColor(Color::WHITE),
                            Node {
                                position_type: PositionType::Absolute,
                                bottom: Px(5.0),
                                right: Px(5.0),
                                ..default()
                            },
                            InventoryCountText {
                                segment_type: *segment_type,
                            },
                        ));
                    });
            }
        });

    // 右侧目标面板（调整位置适应1280x720）
    if let Some(level_data) = &game_state.current_level {
        commands
            .spawn((
                Node {
                    width: Px(280.0),  // 缩小宽度
                    height: Px(180.0), // 缩小高度
                    position_type: PositionType::Absolute,
                    right: Px(10.0),
                    top: Px(90.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Px(12.0)), // 减少内边距
                    row_gap: Px(8.0),               // 减少间距
                    ..default()
                },
                BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 0.7)),
                ZIndex(50),
                GameplayUI,
            ))
            .with_children(|parent| {
                parent.spawn((
                    Text::new(get_text(&OBJECTIVES, current_language.language)),
                    TextFont {
                        font: ui_assets.font.clone(),
                        font_size: 16.0, // 缩小字体
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));

                for (index, objective) in level_data.objectives.iter().enumerate() {
                    let is_completed = game_state
                        .objectives_completed
                        .get(index)
                        .copied()
                        .unwrap_or(false);

                    parent
                        .spawn((
                            Node {
                                flex_direction: FlexDirection::Row,
                                align_items: AlignItems::Center,
                                column_gap: Px(10.0),
                                ..default()
                            },
                            ObjectiveUI {
                                objective_index: index,
                            },
                        ))
                        .with_children(|parent| {
                            parent.spawn((
                                Node {
                                    width: Px(16.0),
                                    height: Px(16.0),
                                    ..default()
                                },
                                BackgroundColor(if is_completed {
                                    Color::srgb(0.0, 1.0, 0.0)
                                } else {
                                    Color::srgb(0.5, 0.5, 0.5)
                                }),
                            ));

                            // 如果目标涉及特定乘客，显示相应图标
                            if let Some(passenger_colors) =
                                get_objective_passenger_colors(objective)
                            {
                                for color in passenger_colors {
                                    if let Some(icon) = ui_assets.passenger_icons.get(&color) {
                                        parent.spawn((
                                            ImageNode::new(icon.clone()),
                                            Node {
                                                width: Px(16.0),
                                                height: Px(16.0),
                                                margin: UiRect::right(Px(4.0)),
                                                ..default()
                                            },
                                        ));
                                    }
                                }
                            }

                            parent.spawn((
                                Text::new(&objective.description),
                                TextFont {
                                    font: ui_assets.font.clone(),
                                    font_size: 14.0,
                                    ..default()
                                },
                                TextColor(if is_completed {
                                    Color::srgb(0.8, 1.0, 0.8)
                                } else {
                                    Color::WHITE
                                }),
                            ));
                        });
                }
            });

        // 新增：乘客统计面板（调整位置和大小）
        commands
            .spawn((
                Node {
                    width: Px(280.0),  // 缩小宽度
                    height: Px(160.0), // 缩小高度
                    position_type: PositionType::Absolute,
                    right: Px(10.0),
                    top: Px(280.0), // 调整位置，在目标面板下方
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Px(12.0)), // 减少内边距
                    row_gap: Px(6.0),               // 减少间距
                    ..default()
                },
                BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 0.7)),
                ZIndex(50),
                GameplayUI,
                PassengerStatsPanel,
            ))
            .with_children(|parent| {
                parent.spawn((
                    Text::new(get_text(&PASSENGER_STATUS, current_language.language)),
                    TextFont {
                        font: ui_assets.font.clone(),
                        font_size: 14.0, // 缩小字体
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));

                // 为每种在关卡中出现的乘客颜色创建状态行
                let passenger_colors = get_level_passenger_colors(level_data);
                for color in passenger_colors {
                    parent
                        .spawn((Node {
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            column_gap: Px(8.0),
                            ..default()
                        },))
                        .with_children(|parent| {
                            // 乘客图标
                            if let Some(icon) = ui_assets.passenger_icons.get(&color) {
                                parent.spawn((
                                    ImageNode::new(icon.clone()),
                                    Node {
                                        width: Px(20.0),
                                        height: Px(20.0),
                                        ..default()
                                    },
                                    PassengerColorIcon { color },
                                ));
                            }

                            // 状态标签
                            parent.spawn((
                                Text::new(format!("{:?}:", color)),
                                TextFont {
                                    font: ui_assets.font.clone(),
                                    font_size: 12.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.9, 0.9, 0.9)),
                                Node {
                                    width: Px(70.0),
                                    ..default()
                                },
                            ));

                            // 等待数量
                            parent.spawn((
                                Text::new("Waiting: 0"),
                                TextFont {
                                    font: ui_assets.font.clone(),
                                    font_size: 12.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(1.0, 1.0, 0.0)),
                                PassengerColorCountText { color },
                                Node {
                                    width: Px(70.0),
                                    ..default()
                                },
                            ));

                            // 到达数量
                            parent.spawn((
                                Text::new("Arrived:0"),
                                TextFont {
                                    font: ui_assets.font.clone(),
                                    font_size: 12.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.0, 1.0, 0.0)),
                                PassengerColorCountText { color },
                            ));
                        });
                }
            });
    }

    // 新增：Tips提示面板（左下角，调整为适合1280x720窗口）
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Px(10.0),
                bottom: Px(10.0),
                width: Px(280.0),  // 缩小宽度适应窗口
                height: Px(320.0), // 缩小高度适应窗口
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Px(12.0)), // 减少内边距
                row_gap: Px(6.0),               // 减少间距
                ..default()
            },
            BackgroundColor(Color::srgba(0.1, 0.1, 0.2, 0.9)),
            BorderColor(Color::srgb(0.3, 0.3, 0.4)),
            Outline::new(Val::Px(2.0), Val::ZERO, Color::srgb(0.3, 0.3, 0.4)),
            ZIndex(100),
            GameplayUI,
            TipsPanel, // 添加TipsPanel组件
            Name::new("Tips Panel"),
        ))
        .with_children(|parent| {
            create_tips_panel(parent, &ui_assets, &tips_manager);
        });
}

fn setup_pause_menu(
    mut commands: Commands,
    ui_assets: Res<UIAssets>,
    current_language: Res<CurrentLanguage>,
) {
    commands
        .spawn((
            Node {
                width: Percent(100.0),
                height: Percent(100.0),
                position_type: PositionType::Absolute,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                top: Px(0.0),
                left: Px(0.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
            ZIndex(3000), // 增加Z-index，确保在最上层
            PauseMenuUI,
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Node {
                        width: Px(300.0),
                        height: Px(400.0),
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        row_gap: Px(20.0),
                        padding: UiRect::all(Px(30.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.2, 0.2, 0.3)),
                    ZIndex(3001), // 面板在背景之上
                ))
                .with_children(|parent| {
                    spawn_title_text(
                        parent,
                        &ui_assets,
                        &get_text(&GAME_PAUSED, current_language.language),
                        30.0,
                    );

                    // 使用专用的暂停菜单按钮生成函数
                    spawn_pause_menu_button(
                        parent,
                        &ui_assets,
                        &get_text(&RESUME_GAME, current_language.language),
                        ButtonType::ResumeGame,
                        Color::srgb(0.2, 0.6, 0.2),
                    );
                    spawn_pause_menu_button(
                        parent,
                        &ui_assets,
                        &get_text(&RESTART_LEVEL, current_language.language),
                        ButtonType::RestartLevel,
                        Color::srgb(0.6, 0.6, 0.2),
                    );
                    spawn_pause_menu_button(
                        parent,
                        &ui_assets,
                        &get_text(&MAIN_MENU, current_language.language),
                        ButtonType::MainMenu,
                        Color::srgb(0.6, 0.2, 0.2),
                    );
                });
        });
}

// 新增：更新乘客统计UI的系统
fn update_passenger_stats_ui(
    passengers: Query<&PathfindingAgent>,
    mut passenger_count_texts: Query<(&PassengerColorCountText, &mut Text)>,
    current_language: Res<CurrentLanguage>,
) {
    // 统计每种颜色的乘客状态
    let mut waiting_counts = HashMap::new();
    let mut arrived_counts = HashMap::new();

    for agent in passengers.iter() {
        let waiting_entry = waiting_counts.entry(agent.color).or_insert(0);
        let arrived_entry = arrived_counts.entry(agent.color).or_insert(0);

        match agent.state {
            AgentState::WaitingAtStation | AgentState::Traveling | AgentState::Transferring => {
                *waiting_entry += 1;
            }
            AgentState::Arrived => {
                *arrived_entry += 1;
            }
            _ => {}
        }
    }

    // 更新UI文本
    for (count_component, mut text) in passenger_count_texts.iter_mut() {
        let waiting_count = waiting_counts.get(&count_component.color).unwrap_or(&0);
        let arrived_count = arrived_counts.get(&count_component.color).unwrap_or(&0);

        // 根据文本内容判断是等待还是到达的计数器
        if text.0.starts_with("等待:") || text.0.starts_with("Waiting:") {
            *text = Text::new(get_text_with_args(
                &WAITING,
                current_language.language,
                &[waiting_count.to_string().as_str()],
            ));
        } else if text.0.starts_with("到达:") || text.0.starts_with("Arrived:") {
            *text = Text::new(get_text_with_args(
                &ARRIVED,
                current_language.language,
                &[arrived_count.to_string().as_str()],
            ));
        }
    }
}

// 辅助函数：获取目标中涉及的乘客颜色
fn get_objective_passenger_colors(objective: &ObjectiveCondition) -> Option<Vec<PassengerColor>> {
    match &objective.condition_type {
        ObjectiveType::ConnectAllPassengers => {
            // 如果是连接所有乘客的目标，可以显示所有颜色
            // 这里返回None，表示不显示特定颜色图标
            None
        }
        // 可以扩展其他目标类型的颜色识别
        _ => None,
    }
}

// 辅助函数：获取关卡中出现的所有乘客颜色
fn get_level_passenger_colors(level_data: &LevelData) -> Vec<PassengerColor> {
    let mut colors = Vec::new();
    for demand in &level_data.passenger_demands {
        if !colors.contains(&demand.color) {
            colors.push(demand.color);
        }
    }
    colors
}
fn capture_level_complete_data(
    mut level_completed_events: EventReader<LevelCompletedEvent>,
    mut level_complete_data: ResMut<LevelCompleteData>,
) {
    for event in level_completed_events.read() {
        level_complete_data.final_score = event.final_score;
        level_complete_data.completion_time = event.completion_time;
        info!(
            "捕获关卡完成数据: 分数={}, 时间={:.1}s",
            event.final_score, event.completion_time
        );
    }
}

// ============ 辅助函数 ============

fn spawn_title_text(
    parent: &mut ChildSpawnerCommands<'_>,
    ui_assets: &UIAssets,
    text: &str,
    size: f32,
) {
    parent.spawn((
        Text::new(text),
        TextFont {
            font: ui_assets.font.clone(),
            font_size: size,
            ..default()
        },
        TextColor(Color::srgb(1.0, 1.0, 0.0)),
    ));
}

fn spawn_score_text(
    parent: &mut ChildSpawnerCommands<'_>,
    ui_assets: &UIAssets,
    text: &str,
    size: f32,
) {
    parent.spawn((
        Text::new(text),
        TextFont {
            font: ui_assets.font.clone(),
            font_size: size,
            ..default()
        },
        TextColor(Color::WHITE),
    ));
}

fn spawn_menu_button(
    parent: &mut ChildSpawnerCommands<'_>,
    ui_assets: &UIAssets,
    text: &str,
    button_type: ButtonType,
) {
    parent
        .spawn((
            Button,
            Node {
                width: Px(200.0),
                height: Px(50.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                margin: UiRect::all(Px(5.0)),
                ..default()
            },
            ImageNode::new(ui_assets.button_texture.clone()),
            ButtonComponent {
                button_type,
                is_hovered: false,
                is_pressed: false,
            },
            // 确保按钮可以接收交互
            ZIndex(10), // 高Z-index确保在其他元素之上
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new(text),
                TextFont {
                    font: ui_assets.font.clone(),
                    font_size: 18.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                // 确保文本不会阻挡按钮交互
                ZIndex(11),
            ));
        });
}

// 专用的暂停菜单按钮生成函数
fn spawn_pause_menu_button(
    parent: &mut ChildSpawnerCommands<'_>,
    ui_assets: &UIAssets,
    text: &str,
    button_type: ButtonType,
    color: Color,
) {
    parent
        .spawn((
            Button,
            Node {
                width: Px(200.0),
                height: Px(50.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                margin: UiRect::all(Px(5.0)),
                ..default()
            },
            BackgroundColor(color),
            ButtonComponent {
                button_type: button_type.clone(),
                is_hovered: false,
                is_pressed: false,
            },
            ZIndex(3002), // 按钮在面板之上
            // 添加名称以便调试
            Name::new(format!("PauseMenuButton_{:?}", button_type)),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new(text),
                TextFont {
                    font: ui_assets.font.clone(),
                    font_size: 18.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                ZIndex(3003), // 文本在按钮之上
            ));
        });
}

// ============ 清理系统 ============

fn cleanup_main_menu(mut commands: Commands, ui_query: Query<Entity, With<MainMenuUI>>) {
    for entity in ui_query.iter() {
        commands.entity(entity).despawn();
    }
}

fn cleanup_gameplay_ui(mut commands: Commands, ui_query: Query<Entity, With<GameplayUI>>) {
    for entity in ui_query.iter() {
        commands.entity(entity).despawn();
    }
}

fn cleanup_pause_menu(mut commands: Commands, ui_query: Query<Entity, With<PauseMenuUI>>) {
    for entity in ui_query.iter() {
        commands.entity(entity).despawn();
    }
}

fn cleanup_level_complete_ui(
    mut commands: Commands,
    ui_query: Query<Entity, With<LevelCompleteUI>>,
) {
    for entity in ui_query.iter() {
        commands.entity(entity).despawn();
    }
}

// ============ 交互处理系统 ============

fn handle_button_interactions(
    mut button_query: Query<
        (
            &Interaction,
            &mut ButtonComponent,
            Option<&mut BackgroundColor>,
            Option<&mut ImageNode>,
        ),
        (Changed<Interaction>, With<Button>),
    >,
    audio_assets: Res<AudioAssets>,
    audio_settings: Res<AudioSettings>,
    mut commands: Commands,
) {
    for (interaction, mut button_component, mut bg_color, mut image_node) in button_query.iter_mut()
    {
        match *interaction {
            Interaction::Pressed => {
                button_component.is_pressed = true;

                // 根据按钮类型（背景色或纹理）应用交互效果
                if let Some(ref mut color) = bg_color {
                    **color = Color::srgb(0.1, 0.1, 0.1).into();
                } else if let Some(ref mut image) = image_node {
                    image.color = Color::srgb(0.7, 0.7, 0.7); // 纹理按钮按下时变暗
                }

                if !audio_settings.is_muted {
                    commands.spawn((
                        AudioPlayer::new(audio_assets.button_click_sound.clone()),
                        PlaybackSettings {
                            mode: PlaybackMode::Despawn,
                            volume: Volume::Linear(
                                audio_settings.sfx_volume * audio_settings.master_volume,
                            ),
                            ..default()
                        },
                    ));
                }
            }
            Interaction::Hovered => {
                button_component.is_hovered = true;
                button_component.is_pressed = false;

                // 悬停效果
                if let Some(ref mut color) = bg_color {
                    **color = Color::srgb(0.5, 0.5, 0.7).into();
                } else if let Some(ref mut image) = image_node {
                    image.color = Color::srgb(1.1, 1.1, 1.1); // 纹理按钮悬停时稍微变亮
                }
            }
            Interaction::None => {
                button_component.is_hovered = false;
                button_component.is_pressed = false;

                // 重置为原始颜色
                if let Some(ref mut color) = bg_color {
                    // 根据按钮类型重置颜色
                    let original_color = match button_component.button_type {
                        ButtonType::ResumeGame => Color::srgb(0.2, 0.6, 0.2),
                        ButtonType::RestartLevel => Color::srgb(0.6, 0.6, 0.2),
                        ButtonType::MainMenu => Color::srgb(0.6, 0.2, 0.2),
                        ButtonType::StartGame => Color::srgb(0.2, 0.6, 0.2),
                        ButtonType::QuitGame => Color::srgb(0.6, 0.2, 0.2),
                        ButtonType::NextLevel => Color::srgb(0.2, 0.6, 0.2),
                        ButtonType::PauseGame => Color::srgb(0.3, 0.3, 0.3),
                        _ => Color::srgb(0.3, 0.3, 0.5),
                    };
                    **color = original_color.into();
                } else if let Some(ref mut image) = image_node {
                    image.color = Color::WHITE; // 纹理按钮重置为正常白色
                }
            }
        }
    }
}

fn handle_menu_buttons(
    button_query: Query<&ButtonComponent, (Changed<ButtonComponent>, With<Button>)>,
    mut next_state: ResMut<NextState<GameStateEnum>>,
    mut app_exit_events: EventWriter<AppExit>,
) {
    for button in button_query.iter() {
        if button.is_pressed {
            match button.button_type {
                ButtonType::StartGame => {
                    next_state.set(GameStateEnum::Playing);
                }
                ButtonType::QuitGame => {
                    app_exit_events.write(AppExit::Success);
                }
                _ => {}
            }
        }
    }
}

fn handle_pause_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    current_state: Res<State<GameStateEnum>>,
    mut next_state: ResMut<NextState<GameStateEnum>>,
) {
    if keyboard_input.just_pressed(KeyCode::Escape) {
        match current_state.get() {
            GameStateEnum::Playing => {
                next_state.set(GameStateEnum::Paused);
            }
            GameStateEnum::Paused => {
                next_state.set(GameStateEnum::Playing);
            }
            _ => {}
        }
    }
}

fn handle_pause_buttons(
    mut button_query: Query<(&Interaction, &ButtonComponent), (Changed<Interaction>, With<Button>)>,
    mut next_state: ResMut<NextState<GameStateEnum>>,
    mut level_manager: ResMut<LevelManager>,
) {
    for (interaction, button_component) in button_query.iter_mut() {
        // 修复：使用 Interaction::Pressed 而不是 button_component.is_pressed
        if matches!(*interaction, Interaction::Pressed) {
            info!("暂停菜单按钮被点击: {:?}", button_component.button_type);

            match button_component.button_type {
                ButtonType::ResumeGame => {
                    info!("继续游戏");
                    next_state.set(GameStateEnum::Playing);
                }
                ButtonType::RestartLevel => {
                    info!("重新开始关卡");
                    next_state.set(GameStateEnum::Loading);
                }
                ButtonType::MainMenu => {
                    info!("返回主菜单");
                    next_state.set(GameStateEnum::MainMenu);
                }
                ButtonType::NextLevel => {
                    level_manager.current_level_index += 1;
                    if level_manager.current_level_index < level_manager.available_levels.len() {
                        next_state.set(GameStateEnum::Loading);
                    } else {
                        next_state.set(GameStateEnum::MainMenu);
                    }
                }
                _ => {}
            }
        }
    }
}

fn handle_level_complete_buttons(
    button_query: Query<&ButtonComponent, (Changed<ButtonComponent>, With<Button>)>,
    mut next_state: ResMut<NextState<GameStateEnum>>,
    mut level_manager: ResMut<LevelManager>,
) {
    for button in button_query.iter() {
        if button.is_pressed {
            info!("关卡完成界面按钮被点击: {:?}", button.button_type);
            match button.button_type {
                ButtonType::NextLevel => {
                    // 解锁下一关
                    let next_level_index = level_manager.current_level_index + 1;
                    if next_level_index < level_manager.available_levels.len() {
                        // 确保下一关被解锁
                        if next_level_index < level_manager.unlocked_levels.len() {
                            level_manager.unlocked_levels[next_level_index] = true;
                            info!(
                                "解锁关卡: {} ({})",
                                next_level_index, level_manager.available_levels[next_level_index]
                            );
                        }

                        level_manager.current_level_index = next_level_index;
                        info!(
                            "进入下一关卡，索引: {} ({})",
                            level_manager.current_level_index,
                            level_manager.available_levels[level_manager.current_level_index]
                        );
                        next_state.set(GameStateEnum::Loading);
                    } else {
                        info!("🎉 所有关卡已完成！恭喜通关！");
                        // 可以在这里添加一个"游戏完成"状态，或者返回主菜单并显示成就
                        next_state.set(GameStateEnum::MainMenu);
                    }
                }
                ButtonType::RestartLevel => {
                    info!(
                        "重新挑战当前关卡，索引: {}",
                        level_manager.current_level_index
                    );
                    next_state.set(GameStateEnum::Loading);
                }
                ButtonType::MainMenu => {
                    info!("返回主菜单");
                    next_state.set(GameStateEnum::MainMenu);
                }
                _ => {}
            }
        }
    }
}

// ============ UI 更新系统 ============

fn update_ui_animations(
    mut commands: Commands,
    mut animated_ui_query: Query<(Entity, &mut AnimatedUI, &mut Transform), With<AnimatedUI>>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();

    for (entity, mut animation, mut transform) in animated_ui_query.iter_mut() {
        animation.elapsed += dt;
        let progress = (animation.elapsed / animation.duration).clamp(0.0, 1.0);

        match animation.animation_type {
            UIAnimation::FadeIn => {
                // 这里需要访问 Sprite 或 BackgroundColor 组件
                // 简化处理，仅作为示例
            }
            UIAnimation::ScaleUp => {
                let scale = animation.start_value
                    + (animation.target_value - animation.start_value) * ease_out_back(progress);
                transform.scale = Vec3::splat(scale);
            }
            UIAnimation::Bounce => {
                let bounce_offset =
                    (progress * std::f32::consts::PI * 4.0).sin() * (1.0 - progress) * 10.0;
                transform.translation.y += bounce_offset;
            }
            _ => {}
        }

        if progress >= 1.0 {
            commands.entity(entity).remove::<AnimatedUI>();
        }
    }
}

fn update_progress_bars(
    mut progress_bars: Query<(&mut ProgressBar, &mut Node)>,
    game_state: Res<GameState>,
) {
    for (mut progress_bar, mut node) in progress_bars.iter_mut() {
        let progress = match progress_bar.bar_type {
            ProgressBarType::ObjectiveProgress => {
                let completed_objectives = game_state
                    .objectives_completed
                    .iter()
                    .filter(|&&completed| completed)
                    .count();
                let total_objectives = game_state.objectives_completed.len();
                if total_objectives > 0 {
                    completed_objectives as f32 / total_objectives as f32
                } else {
                    0.0
                }
            }
            ProgressBarType::TimeRemaining => {
                if let Some(level_data) = &game_state.current_level {
                    if let Some(time_limit) =
                        level_data
                            .objectives
                            .iter()
                            .find_map(|obj| match &obj.condition_type {
                                ObjectiveType::TimeLimit(limit) => Some(*limit),
                                _ => None,
                            })
                    {
                        1.0 - (game_state.game_time / time_limit).clamp(0.0, 1.0)
                    } else {
                        1.0
                    }
                } else {
                    1.0
                }
            }
            ProgressBarType::BudgetUsed => {
                if let Some(level_data) = &game_state.current_level {
                    if let Some(budget_limit) =
                        level_data
                            .objectives
                            .iter()
                            .find_map(|obj| match &obj.condition_type {
                                ObjectiveType::MaxCost(limit) => Some(*limit as f32),
                                _ => None,
                            })
                    {
                        (game_state.total_cost as f32 / budget_limit).clamp(0.0, 1.0)
                    } else {
                        0.0
                    }
                } else {
                    0.0
                }
            }
        };

        progress_bar.current_value = progress * progress_bar.max_value;
        node.width = Percent(progress * 100.0);
    }
}

// ============ 音频系统 ============

fn handle_audio_events(
    mut commands: Commands,
    audio_assets: Res<AudioAssets>,
    audio_settings: Res<AudioSettings>,
    mut segment_placed_events: EventReader<SegmentPlacedEvent>,
    mut segment_removed_events: EventReader<SegmentRemovedEvent>,
    mut objective_completed_events: EventReader<ObjectiveCompletedEvent>,
    mut level_completed_events: EventReader<LevelCompletedEvent>,
    passengers: Query<&PathfindingAgent, Changed<PathfindingAgent>>,
) {
    if audio_settings.is_muted {
        return;
    }

    let base_volume = audio_settings.sfx_volume * audio_settings.master_volume;

    // 路线段放置音效
    for _event in segment_placed_events.read() {
        commands.spawn((
            AudioPlayer::new(audio_assets.segment_place_sound.clone()),
            PlaybackSettings {
                mode: PlaybackMode::Despawn,
                volume: Volume::Linear(base_volume),
                ..default()
            },
        ));
    }

    // 路线段移除音效
    for event in segment_removed_events.read() {
        info!("segment removed at: {:?}", event.position);
        commands.spawn((
            AudioPlayer::new(audio_assets.segment_remove_sound.clone()),
            PlaybackSettings {
                mode: PlaybackMode::Despawn,
                volume: Volume::Linear(base_volume),
                ..default()
            },
        ));
    }

    // 目标完成音效
    for _event in objective_completed_events.read() {
        commands.spawn((
            AudioPlayer::new(audio_assets.objective_complete_sound.clone()),
            PlaybackSettings {
                mode: PlaybackMode::Despawn,
                volume: Volume::Linear(base_volume * 1.2),
                ..default()
            },
        ));
    }

    // 关卡完成音效
    for _event in level_completed_events.read() {
        commands.spawn((
            AudioPlayer::new(audio_assets.level_complete_sound.clone()),
            PlaybackSettings {
                mode: PlaybackMode::Despawn,
                volume: Volume::Linear(base_volume * 1.5),
                ..default()
            },
        ));
    }

    // 乘客到达音效
    for agent in passengers.iter() {
        if matches!(agent.state, AgentState::Arrived) {
            commands.spawn((
                AudioPlayer::new(audio_assets.passenger_arrive_sound.clone()),
                PlaybackSettings {
                    mode: PlaybackMode::Despawn,
                    volume: Volume::Linear(base_volume * 0.8),
                    ..default()
                },
            ));
        }
    }
}

fn update_background_music(
    mut commands: Commands,
    audio_assets: Res<AudioAssets>,
    audio_settings: Res<AudioSettings>,
    current_state: Res<State<GameStateEnum>>,
    music_query: Query<Entity, With<AudioSink>>,
) {
    // 简化的背景音乐管理
    // 在实际实现中，你可能需要更复杂的音乐状态管理
    if music_query.is_empty()
        && matches!(current_state.get(), GameStateEnum::Playing)
        && !audio_settings.is_muted
    {
        commands.spawn((
            AudioPlayer::new(audio_assets.background_music.clone()),
            PlaybackSettings {
                mode: PlaybackMode::Despawn,
                volume: Volume::Linear(audio_settings.music_volume * audio_settings.master_volume),
                ..default()
            },
        ));
    }
}

fn setup_game_over_ui(
    mut commands: Commands,
    ui_assets: Res<UIAssets>,
    game_state: Res<GameState>,
    game_over_data: Res<GameOverData>,
    current_language: Res<CurrentLanguage>,
) {
    let game_over_entity = commands
        .spawn((
            Node {
                width: Percent(100.0),
                height: Percent(100.0),
                position_type: PositionType::Absolute,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
            ZIndex(2000),
            GameOverUI,
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Node {
                        width: Px(450.0),
                        height: Px(550.0),
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        row_gap: Px(20.0),
                        padding: UiRect::all(Px(40.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.3, 0.1, 0.1)), // 红色调表示失败
                    ZIndex(2001),
                ))
                .with_children(|parent| {
                    // 失败标题
                    spawn_title_text(
                        parent,
                        &ui_assets,
                        &get_text(&MISSION_FAILED, current_language.language),
                        36.0,
                    );

                    // 失败原因
                    spawn_score_text(
                        parent,
                        &ui_assets,
                        &get_text_with_args(
                            &FAILURE_REASON,
                            current_language.language,
                            &[game_over_data.reason.as_str()],
                        ),
                        20.0,
                    );

                    // 分隔线
                    parent.spawn((
                        Node {
                            width: Percent(80.0),
                            height: Px(2.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.5, 0.5, 0.5)),
                    ));

                    // 游戏统计
                    spawn_score_text(
                        parent,
                        &ui_assets,
                        &get_text(&GAME_STATISTICS, current_language.language),
                        18.0,
                    );

                    spawn_score_text(
                        parent,
                        &ui_assets,
                        &get_text_with_args(
                            &SCORE_EARNED,
                            current_language.language,
                            &[&game_over_data.final_score.to_string()],
                        ),
                        16.0,
                    );

                    spawn_score_text(
                        parent,
                        &ui_assets,
                        &get_text_with_args(
                            &GAME_DURATION,
                            current_language.language,
                            &[&format_time(game_over_data.game_time)],
                        ),
                        16.0,
                    );

                    spawn_score_text(
                        parent,
                        &ui_assets,
                        &get_text_with_args(
                            &TOTAL_COST,
                            current_language.language,
                            &[&game_state.total_cost.to_string()],
                        ),
                        16.0,
                    );

                    if game_over_data.passengers_gave_up > 0 {
                        spawn_score_text(
                            parent,
                            &ui_assets,
                            &get_text_with_args(
                                &PASSENGERS_GAVE_UP,
                                current_language.language,
                                &[&game_over_data.passengers_gave_up.to_string()],
                            ),
                            16.0,
                        );
                    }

                    // 分隔线
                    parent.spawn((
                        Node {
                            width: Percent(80.0),
                            height: Px(2.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.5, 0.5, 0.5)),
                    ));

                    // 鼓励文字和提示
                    spawn_score_text(
                        parent,
                        &ui_assets,
                        &get_text(&DONT_GIVE_UP, current_language.language),
                        18.0,
                    );

                    // 根据失败原因显示提示
                    let tip = get_failure_tip(&game_over_data.reason);
                    spawn_score_text(parent, &ui_assets, tip, 14.0);

                    // 按钮组
                    spawn_menu_button(
                        parent,
                        &ui_assets,
                        &get_text(&RETRY, current_language.language),
                        ButtonType::RestartLevel,
                    );
                    spawn_menu_button(
                        parent,
                        &ui_assets,
                        &get_text(&MAIN_MENU, current_language.language),
                        ButtonType::MainMenu,
                    );
                });
        })
        .id();

    // 添加动画效果
    commands.entity(game_over_entity).insert(AnimatedUI {
        animation_type: UIAnimation::FadeIn,
        duration: 0.5,
        elapsed: 0.0,
        start_value: 0.0,
        target_value: 1.0,
    });

    info!("游戏失败UI创建完毕: {}", game_over_data.reason);

    commands.send_event(LanguageChangedEvent {
        new_language: current_language.language,
    });
}

fn cleanup_game_over_ui(mut commands: Commands, ui_query: Query<Entity, With<GameOverUI>>) {
    for entity in ui_query.iter() {
        commands.entity(entity).despawn();
    }
}

fn handle_game_over_buttons(
    button_query: Query<&ButtonComponent, (Changed<ButtonComponent>, With<Button>)>,
    mut next_state: ResMut<NextState<GameStateEnum>>,
) {
    for button in button_query.iter() {
        if button.is_pressed {
            info!("游戏失败界面按钮被点击: {:?}", button.button_type);
            match button.button_type {
                ButtonType::RestartLevel => {
                    info!("重新挑战当前关卡");
                    next_state.set(GameStateEnum::Loading);
                }
                ButtonType::MainMenu => {
                    info!("返回主菜单");
                    next_state.set(GameStateEnum::MainMenu);
                }
                _ => {}
            }
        }
    }
}

// 辅助函数：根据失败原因提供建议
fn get_failure_tip(reason: &str) -> &'static str {
    if reason.contains("乘客放弃") {
        "💡 提示：尝试建设更短的路径，或者增加换乘站点来减少等待时间"
    } else if reason.contains("时间超限") {
        "💡 提示：优先连接最重要的站点，不要追求完美的网络设计"
    } else if reason.contains("预算超支") {
        "💡 提示：多使用便宜的直线段，减少昂贵的复杂路段"
    } else {
        "💡 提示：分析失败原因，调整策略后重新挑战"
    }
}

// 调试用：添加暂停菜单状态检查系统
fn debug_pause_menu_state(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    current_state: Res<State<GameStateEnum>>,
    buttons: Query<(Entity, &ButtonComponent, &Interaction), With<Button>>,
    ui_elements: Query<Entity, With<PauseMenuUI>>,
) {
    if keyboard_input.just_pressed(KeyCode::F10) {
        info!("=== 暂停菜单调试信息 ===");
        info!("当前游戏状态: {:?}", current_state.get());
        info!("暂停菜单UI实体数量: {}", ui_elements.iter().count());

        for (entity, button_component, interaction) in buttons.iter() {
            info!(
                "按钮 {:?}: 实体 {:?}, 交互状态 {:?}, 悬停: {}, 按下: {}",
                button_component.button_type,
                entity,
                interaction,
                button_component.is_hovered,
                button_component.is_pressed
            );
        }
    }
}

// ============ 关卡完成界面修改示例 ============

fn setup_level_complete_ui(
    mut commands: Commands,
    ui_assets: Res<UIAssets>,
    game_state: Res<GameState>,
    level_manager: Res<LevelManager>,
    level_complete_data: Res<LevelCompleteData>,
    current_language: Res<CurrentLanguage>,
) {
    let level_complete_entity = commands
        .spawn((
            Node {
                width: Percent(100.0),
                height: Percent(100.0),
                position_type: PositionType::Absolute,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
            ZIndex(2000),
            LevelCompleteUI,
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Node {
                        width: Px(400.0),
                        height: Px(500.0),
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        row_gap: Px(20.0),
                        padding: UiRect::all(Px(40.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.1, 0.3, 0.1)),
                    ZIndex(2001),
                ))
                .with_children(|parent| {
                    // 检查是否是最后一关
                    let is_final_level = level_manager.current_level_index + 1
                        >= level_manager.available_levels.len();

                    // 标题
                    let title_key = if is_final_level {
                        &CONGRATULATIONS
                    } else {
                        &LEVEL_COMPLETE
                    };
                    spawn_localized_title(parent, &ui_assets, title_key, 36.0);

                    if is_final_level {
                        spawn_localized_score_text(parent, &ui_assets, &ALL_LEVELS_COMPLETE, 18.0);
                    }

                    // 最终分数
                    let final_score = if level_complete_data.final_score > 0 {
                        level_complete_data.final_score
                    } else {
                        game_state.score.total_score
                    };

                    let score_args = vec![final_score.to_string()];
                    spawn_localized_score_text_with_args(
                        parent,
                        &ui_assets,
                        &FINAL_SCORE,
                        score_args,
                        24.0,
                    );

                    // 分数详细分解
                    let score = &game_state.score;
                    let breakdown_args = vec![
                        score.base_points.to_string(),
                        score.efficiency_bonus.to_string(),
                        score.speed_bonus.to_string(),
                        score.cost_bonus.to_string(),
                    ];
                    spawn_localized_score_text_with_args(
                        parent,
                        &ui_assets,
                        &SCORE_BREAKDOWN,
                        breakdown_args,
                        16.0,
                    );

                    // 完成时间
                    let completion_time = if level_complete_data.completion_time > 0.0 {
                        level_complete_data.completion_time
                    } else {
                        game_state.game_time
                    };

                    let time_args = vec![format_time(completion_time)];
                    spawn_localized_score_text_with_args(
                        parent,
                        &ui_assets,
                        &COMPLETION_TIME,
                        time_args,
                        20.0,
                    );

                    // 总成本
                    let cost_args = vec![game_state.total_cost.to_string()];
                    spawn_localized_score_text_with_args(
                        parent,
                        &ui_assets,
                        &TOTAL_COST,
                        cost_args,
                        20.0,
                    );

                    // 按钮
                    if !is_final_level {
                        spawn_localized_menu_button(
                            parent,
                            &ui_assets,
                            &NEXT_LEVEL,
                            ButtonType::NextLevel,
                        );
                    } else {
                        spawn_localized_score_text(parent, &ui_assets, &THANK_YOU, 18.0);
                    }

                    spawn_localized_menu_button(
                        parent,
                        &ui_assets,
                        &RETRY,
                        ButtonType::RestartLevel,
                    );
                    spawn_localized_menu_button(
                        parent,
                        &ui_assets,
                        &MAIN_MENU,
                        ButtonType::MainMenu,
                    );
                });
        })
        .id();

    // 添加动画
    commands.entity(level_complete_entity).insert(AnimatedUI {
        animation_type: UIAnimation::ScaleUp,
        duration: 0.3,
        elapsed: 0.0,
        start_value: 0.8,
        target_value: 1.0,
    });

    commands.send_event(LanguageChangedEvent {
        new_language: current_language.language,
    });
}

// ============ 语言切换处理 ============

#[derive(Component)]
pub struct LanguageToggleText;

fn handle_language_toggle_button(
    button_query: Query<(&Interaction, &ButtonComponent), (Changed<Interaction>, With<Button>)>,
    current_language: Res<CurrentLanguage>,
    mut language_events: EventWriter<LanguageChangedEvent>,
    mut toggle_texts: Query<&mut Text, With<LanguageToggleText>>,
) {
    for (interaction, button_component) in button_query.iter() {
        if matches!(*interaction, Interaction::Pressed) {
            if let ButtonType::ToggleLanguage = button_component.button_type {
                let new_language = match current_language.language {
                    Language::English => Language::Chinese,
                    Language::Chinese => Language::English,
                };

                // 发送语言切换事件
                language_events.write(LanguageChangedEvent { new_language });

                // 立即更新语言切换按钮的文本
                for mut text in toggle_texts.iter_mut() {
                    let next_language_text = match new_language {
                        Language::English => "中文",
                        Language::Chinese => "English",
                    };
                    *text = Text::new(next_language_text);
                }
            }
        }
    }
}

// ============ 动态文本更新系统 ============

fn update_gameplay_ui_values(
    game_state: Res<GameState>,
    current_language: Res<CurrentLanguage>,
    mut score_text: Query<
        (&mut LocalizedTextComponent, &mut Text),
        (
            With<ScoreText>,
            Without<TimerText>,
            Without<CostText>,
            Without<PassengerCountText>,
        ),
    >,
    mut timer_text: Query<
        (&mut LocalizedTextComponent, &mut Text),
        (
            With<TimerText>,
            Without<ScoreText>,
            Without<CostText>,
            Without<PassengerCountText>,
        ),
    >,
    mut cost_text: Query<
        (&mut LocalizedTextComponent, &mut Text),
        (
            With<CostText>,
            Without<ScoreText>,
            Without<TimerText>,
            Without<PassengerCountText>,
        ),
    >,
    mut passenger_text: Query<
        (&mut LocalizedTextComponent, &mut Text),
        (
            With<PassengerCountText>,
            Without<ScoreText>,
            Without<TimerText>,
            Without<CostText>,
        ),
    >,
) {
    // 更新分数文本
    if let Ok((mut localized, mut text)) = score_text.single_mut() {
        let score = &game_state.score;
        localized.format_args = Some(vec![
            score.total_score.to_string(),
            score.base_points.to_string(),
            score.efficiency_bonus.to_string(),
            score.speed_bonus.to_string(),
            score.cost_bonus.to_string(),
        ]);
        *text = Text::new(localized.get_text(current_language.language));
    }

    // 更新时间文本
    if let Ok((mut localized, mut text)) = timer_text.single_mut() {
        localized.format_args = Some(vec![format_time(game_state.game_time)]);
        *text = Text::new(localized.get_text(current_language.language));
    }

    // 更新成本文本
    if let Ok((mut localized, mut text)) = cost_text.single_mut() {
        localized.format_args = Some(vec![game_state.total_cost.to_string()]);
        *text = Text::new(localized.get_text(current_language.language));
    }

    // 更新乘客文本
    if let Ok((mut localized, mut text)) = passenger_text.single_mut() {
        let arrived_passengers = game_state.passenger_stats.total_arrived;
        let total_passengers = game_state.passenger_stats.total_spawned;
        localized.format_args = Some(vec![
            arrived_passengers.to_string(),
            total_passengers.to_string(),
        ]);
        *text = Text::new(localized.get_text(current_language.language));
    }
}

// ============ 辅助函数 ============

fn spawn_localized_title(
    parent: &mut ChildSpawnerCommands<'_>,
    ui_assets: &UIAssets,
    text_key: &'static LocalizedText,
    size: f32,
) {
    let (localized_text, text) = localized_text(text_key);
    parent.spawn((
        text,
        TextFont {
            font: ui_assets.font.clone(),
            font_size: size,
            ..default()
        },
        TextColor(Color::srgb(1.0, 1.0, 0.0)),
        localized_text,
    ));
}

fn spawn_localized_score_text(
    parent: &mut ChildSpawnerCommands<'_>,
    ui_assets: &UIAssets,
    text_key: &'static LocalizedText,
    size: f32,
) -> Entity {
    let (localized_text, text) = localized_text(text_key);
    parent
        .spawn((
            text,
            TextFont {
                font: ui_assets.font.clone(),
                font_size: size,
                ..default()
            },
            TextColor(Color::WHITE),
            localized_text,
        ))
        .id()
}

fn spawn_localized_score_text_with_args(
    parent: &mut ChildSpawnerCommands<'_>,
    ui_assets: &UIAssets,
    text_key: &'static LocalizedText,
    args: Vec<String>,
    size: f32,
) {
    let (localized_text, text) = localized_text_with_args(text_key, args);
    parent.spawn((
        text,
        TextFont {
            font: ui_assets.font.clone(),
            font_size: size,
            ..default()
        },
        TextColor(Color::WHITE),
        localized_text,
    ));
}

fn spawn_localized_menu_button(
    parent: &mut ChildSpawnerCommands<'_>,
    ui_assets: &UIAssets,
    text_key: &'static LocalizedText,
    button_type: ButtonType,
) {
    parent
        .spawn((
            Button,
            Node {
                width: Px(200.0),
                height: Px(50.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                margin: UiRect::all(Px(5.0)),
                ..default()
            },
            ImageNode::new(ui_assets.button_texture.clone()),
            ButtonComponent {
                button_type,
                is_hovered: false,
                is_pressed: false,
            },
            ZIndex(10),
        ))
        .with_children(|parent| {
            let (localized_text, text) = localized_text(text_key);
            parent.spawn((
                text,
                TextFont {
                    font: ui_assets.font.clone(),
                    font_size: 18.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                ZIndex(11),
                localized_text,
            ));
        });
}

// ============ 新的按钮类型 ============

#[derive(Clone, Debug, PartialEq)]
pub enum ButtonType {
    StartGame,
    PauseGame,
    ResumeGame,
    RestartLevel,
    NextLevel,
    MainMenu,
    QuitGame,
    ToggleLanguage, // 新增：语言切换按钮
    InventorySlot(RouteSegmentType),
}

// ============ 库存选中状态更新系统 ============

/// 更新库存槽位的选中状态视觉效果
fn update_inventory_selection_state(
    input_state: Res<crate::bus_puzzle::InputState>,
    mut inventory_slots: Query<(&InventorySlot, &mut BorderColor)>,
) {
    for (slot, mut border_color) in inventory_slots.iter_mut() {
        if let Some(slot_segment_type) = &slot.segment_type {
            let is_selected = input_state.selected_segment == Some(*slot_segment_type);

            if is_selected {
                // 选中状态：金黄色边框
                *border_color = Color::srgb(1.0, 0.8, 0.0).into();
            } else {
                // 未选中状态：正常白色边框
                *border_color = Color::WHITE.into();
            }
        }
    }
}
