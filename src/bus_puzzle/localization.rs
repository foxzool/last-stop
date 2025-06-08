#[allow(dead_code)]
// src/bus_puzzle/localization.rs - 本地化系统核心
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

// ============ 语言枚举 ============

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Language {
    #[default]
    English,
    Chinese,
}

impl Language {
    pub fn code(&self) -> &'static str {
        match self {
            Language::English => "en",
            Language::Chinese => "zh",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Language::English => "English",
            Language::Chinese => "中文",
        }
    }
}

// ============ 本地化文本结构 ============

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LocalizedText {
    pub en: &'static str,
    pub zh: &'static str,
}

impl LocalizedText {
    pub const fn new(en: &'static str, zh: &'static str) -> Self {
        Self { en, zh }
    }

    pub fn get(&self, language: Language) -> &'static str {
        match language {
            Language::English => self.en,
            Language::Chinese => self.zh,
        }
    }
}

// ============ 当前语言资源 ============

#[derive(Resource, Debug, Clone)]
pub struct CurrentLanguage {
    pub language: Language,
}

impl Default for CurrentLanguage {
    fn default() -> Self {
        Self {
            language: Language::Chinese, // 默认中文
        }
    }
}

// ============ 本地化组件 ============

#[derive(Component, Debug, Clone)]
pub struct LocalizedTextComponent {
    pub text_key: &'static LocalizedText,
    pub format_args: Option<Vec<String>>, // 支持格式化参数
}

impl LocalizedTextComponent {
    pub fn new(text_key: &'static LocalizedText) -> Self {
        Self {
            text_key,
            format_args: None,
        }
    }

    pub fn with_args(text_key: &'static LocalizedText, args: Vec<String>) -> Self {
        Self {
            text_key,
            format_args: Some(args),
        }
    }

    pub fn get_text(&self, language: Language) -> String {
        let base_text = self.text_key.get(language);

        if let Some(args) = &self.format_args {
            // 简单的参数替换：{0}, {1}, {2}...
            let mut result = base_text.to_string();
            for (i, arg) in args.iter().enumerate() {
                let placeholder = format!("{{{}}}", i);
                result = result.replace(&placeholder, arg);
            }
            result
        } else {
            base_text.to_string()
        }
    }
}

// ============ 事件定义 ============

#[derive(Event)]
pub struct LanguageChangedEvent {
    pub new_language: Language,
}

// ============ 文本常量定义 ============

// 主菜单
pub const GAME_TITLE: LocalizedText = LocalizedText::new("Last Stop", "下一站");
pub const START_GAME: LocalizedText = LocalizedText::new("Start Game", "开始游戏");
pub const QUIT_GAME: LocalizedText = LocalizedText::new("Quit Game", "退出游戏");
pub const LANGUAGE_SETTING: LocalizedText = LocalizedText::new("Language", "语言设置");

// 游戏界面
pub const SCORE: LocalizedText = LocalizedText::new("Score: {0}", "分数");
pub const TIME: LocalizedText = LocalizedText::new("Time: {0}", "时间");
pub const COST: LocalizedText = LocalizedText::new("Cost: {0}", "成本");
pub const PASSENGERS: LocalizedText = LocalizedText::new("Passengers: {0}", "乘客");
pub const PAUSE: LocalizedText = LocalizedText::new("Pause", "暂停");
pub const ROUTE_SEGMENTS: LocalizedText = LocalizedText::new("Route Segments", "路线段");
pub const OBJECTIVES: LocalizedText = LocalizedText::new("Objectives", "目标");
pub const PASSENGER_STATUS: LocalizedText = LocalizedText::new("Passenger Status", "乘客状态");

// 暂停菜单
pub const GAME_PAUSED: LocalizedText = LocalizedText::new("Game Paused", "游戏暂停");
pub const RESUME_GAME: LocalizedText = LocalizedText::new("Resume Game", "继续游戏");
pub const RESTART_LEVEL: LocalizedText = LocalizedText::new("Restart Level", "重新开始");
pub const MAIN_MENU: LocalizedText = LocalizedText::new("Main Menu", "主菜单");

// 关卡完成
pub const LEVEL_COMPLETE: LocalizedText = LocalizedText::new("Level Complete!", "关卡完成！");
pub const CONGRATULATIONS: LocalizedText =
    LocalizedText::new("🎉 Congratulations!", "🎉 恭喜通关！");
pub const FINAL_SCORE: LocalizedText = LocalizedText::new("Final Score: {0}", "最终得分: {0}");
pub const SCORE_BREAKDOWN: LocalizedText = LocalizedText::new(
    "Score Details: Base:{0} Efficiency:+{1} Speed:+{2} Cost:+{3}",
    "分数明细: 基础:{0} 效率:+{1} 速度:+{2} 成本:+{3}",
);
pub const COMPLETION_TIME: LocalizedText = LocalizedText::new("Time: {0}", "用时: {0}");
pub const TOTAL_COST: LocalizedText = LocalizedText::new("Total Cost: {0}", "总成本: {0}");
pub const NEXT_LEVEL: LocalizedText = LocalizedText::new("Next Level", "下一关");
pub const RETRY: LocalizedText = LocalizedText::new("Retry", "重新挑战");
pub const ALL_LEVELS_COMPLETE: LocalizedText =
    LocalizedText::new("You've completed all levels!", "您已完成所有关卡！");
pub const THANK_YOU: LocalizedText = LocalizedText::new("Thank you for playing!", "感谢游玩！");

// 游戏失败
pub const MISSION_FAILED: LocalizedText = LocalizedText::new("❌ Mission Failed", "❌ 任务失败");
pub const FAILURE_REASON: LocalizedText =
    LocalizedText::new("Failure Reason: {0}", "失败原因: {0}");
pub const GAME_STATISTICS: LocalizedText = LocalizedText::new("Game Statistics:", "本次游戏统计:");
pub const SCORE_EARNED: LocalizedText = LocalizedText::new("Score Earned: {0}", "获得分数: {0}");
pub const GAME_DURATION: LocalizedText = LocalizedText::new("Game Duration: {0}", "游戏时长: {0}");
pub const PASSENGERS_GAVE_UP: LocalizedText =
    LocalizedText::new("Passengers Gave Up: {0}", "放弃的乘客: {0}");
pub const DONT_GIVE_UP: LocalizedText =
    LocalizedText::new("Don't give up, try again!", "不要灰心，再试一次！");

// 乘客状态
pub const WAITING: LocalizedText = LocalizedText::new("Waiting: {0}", "等待: {0}");
pub const ARRIVED: LocalizedText = LocalizedText::new("Arrived: {0}", "到达: {0}");
pub const GAVE_UP: LocalizedText = LocalizedText::new("Gave Up: {0}", "放弃: {0}");

// 关卡信息
pub const LEVEL_TUTORIAL: LocalizedText = LocalizedText::new("First Connection", "第一次连接");
pub const LEVEL_TRANSFER: LocalizedText = LocalizedText::new("Learn Transfers", "学会换乘");
pub const LEVEL_MULTIPLE: LocalizedText = LocalizedText::new("Multiple Routes", "多条路线");
pub const LEVEL_TIME_PRESSURE: LocalizedText = LocalizedText::new("Time Challenge", "时间挑战");

// 站点名称
pub const STATION_A: LocalizedText = LocalizedText::new("Station A", "A站");
pub const STATION_B: LocalizedText = LocalizedText::new("Station B", "B站");
pub const STATION_C: LocalizedText = LocalizedText::new("Station C", "C站");
pub const TRANSFER_HUB: LocalizedText = LocalizedText::new("Transfer Hub", "中转站");
pub const NORTH_STATION: LocalizedText = LocalizedText::new("North Station", "北站");
pub const SOUTH_STATION: LocalizedText = LocalizedText::new("South Station", "南站");
pub const NORTHEAST_STATION: LocalizedText = LocalizedText::new("Northeast Station", "东北站");
pub const SOUTHEAST_STATION: LocalizedText = LocalizedText::new("Southeast Station", "东南站");
pub const CENTRAL_HUB: LocalizedText = LocalizedText::new("Central Hub", "中央枢纽");
pub const START_STATION: LocalizedText = LocalizedText::new("Start Station", "起点站");
pub const TARGET_STATION_A: LocalizedText = LocalizedText::new("Target Station A", "目标站A");
pub const TARGET_STATION_B: LocalizedText = LocalizedText::new("Target Station B", "目标站B");
pub const TARGET_STATION_C: LocalizedText = LocalizedText::new("Target Station C", "目标站C");

// 目标描述
pub const OBJECTIVE_CONNECT_ALL: LocalizedText = LocalizedText::new(
    "Connect all passengers to destinations",
    "连接所有乘客到目的地",
);
pub const OBJECTIVE_MAX_TRANSFERS: LocalizedText =
    LocalizedText::new("Maximum {0} transfers", "最多使用{0}次换乘");
pub const OBJECTIVE_MAX_SEGMENTS: LocalizedText =
    LocalizedText::new("Use at most {0} route segments", "最多使用{0}个路线段");
pub const OBJECTIVE_MAX_COST: LocalizedText =
    LocalizedText::new("Total cost ≤ {0}", "总成本不超过{0}");
pub const OBJECTIVE_TIME_LIMIT: LocalizedText =
    LocalizedText::new("Complete within {0} seconds", "在{0}秒内完成");
pub const OBJECTIVE_PASSENGER_SATISFACTION: LocalizedText =
    LocalizedText::new("Passenger satisfaction ≥ {0}%", "乘客满意度达到{0}%");

// 关卡描述
pub const TUTORIAL_DESCRIPTION: LocalizedText = LocalizedText::new(
    "Learn basic route connection by transporting red passengers from Station A to Station B",
    "学习基本的路线连接操作，将红色乘客从A站送到B站",
);
pub const TRANSFER_DESCRIPTION: LocalizedText = LocalizedText::new(
    "Learn to use the transfer system by connecting different routes through transfer stations",
    "学习使用换乘系统，通过中转站连接不同的路线",
);
pub const MULTIPLE_DESCRIPTION: LocalizedText = LocalizedText::new(
    "Manage multiple independent routes and optimize the entire transportation network",
    "管理多条独立路线，优化整个交通网络",
);
pub const TIME_PRESSURE_DESCRIPTION: LocalizedText = LocalizedText::new(
    "Quickly build an efficient transportation network within limited time",
    "在有限时间内快速建设高效的交通网络",
);

// 游戏提示和警告信息
pub const PASSENGERS_GAVE_UP_WARNING: LocalizedText = LocalizedText::new(
    "⚠️ {0} passengers have given up! Check route connections",
    "⚠️ 已有{0}位乘客放弃！检查路线连接",
);

pub const PASSENGERS_WAITING_HINT: LocalizedText = LocalizedText::new(
    "💡 Many passengers are waiting, press F4 to discover bus routes",
    "💡 很多乘客在等车，按F4发现公交路线",
);

pub const BUS_ROUTES_READY_INFO: LocalizedText = LocalizedText::new(
    "🚌 Routes are built, waiting for buses to start operating",
    "🚌 路线已建好，等待公交车开始运营",
);

pub const BUDGET_WARNING: LocalizedText =
    LocalizedText::new("💰 Budget Warning: {0}/{1}", "💰 预算警告: {0}/{1}");

// ============ 本地化系统插件 ============

pub struct LocalizationPlugin;

impl Plugin for LocalizationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CurrentLanguage>()
            .add_event::<LanguageChangedEvent>()
            .add_systems(
                Update,
                (update_localized_texts, handle_language_change_events),
            );
    }
}

// ============ 系统函数 ============

/// 更新所有本地化文本
fn update_localized_texts(
    current_language: Res<CurrentLanguage>,
    mut localized_texts: Query<
        (&LocalizedTextComponent, &mut Text),
        Changed<LocalizedTextComponent>,
    >,
) {
    if current_language.is_changed() {
        // 语言改变时，更新所有文本
        for (localized, mut text) in localized_texts.iter_mut() {
            *text = Text::new(localized.get_text(current_language.language));
        }
    }
}

/// 处理语言切换事件
fn handle_language_change_events(
    mut language_events: EventReader<LanguageChangedEvent>,
    mut current_language: ResMut<CurrentLanguage>,
    mut localized_texts: Query<(&LocalizedTextComponent, &mut Text)>,
) {
    for event in language_events.read() {
        current_language.language = event.new_language;

        // 立即更新所有文本
        for (localized, mut text) in localized_texts.iter_mut() {
            *text = Text::new(localized.get_text(event.new_language));
        }

        info!("Language changed to: {:?}", event.new_language);
    }
}

// ============ 辅助函数 ============

/// 获取本地化文本的便捷函数
pub fn get_text(text_key: &LocalizedText, language: Language) -> String {
    text_key.get(language).to_string()
}

/// 获取带参数的本地化文本
pub fn get_text_with_args(text_key: &LocalizedText, language: Language, args: &[&str]) -> String {
    let mut result = text_key.get(language).to_string();
    for (i, arg) in args.iter().enumerate() {
        let placeholder = format!("{{{}}}", i);
        result = result.replace(&placeholder, arg);
    }
    result
}

/// 创建本地化文本组件的便捷函数
pub fn localized_text(text_key: &'static LocalizedText) -> (LocalizedTextComponent, Text) {
    let component = LocalizedTextComponent::new(text_key);
    let text = Text::new(""); // 初始空文本，会在系统中更新
    (component, text)
}

/// 创建带参数的本地化文本组件
pub fn localized_text_with_args(
    text_key: &'static LocalizedText,
    args: Vec<String>,
) -> (LocalizedTextComponent, Text) {
    let component = LocalizedTextComponent::with_args(text_key, args);
    let text = Text::new(""); // 初始空文本，会在系统中更新
    (component, text)
}
