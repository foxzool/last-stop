use bevy::{audio::Volume, input::common_conditions::input_just_pressed, prelude::*, ui::Val::*};

use crate::{
    game::level::WantLevel,
    menus::{
        Menu,
        common::{go_back, go_back_on_click},
    },
    screens::Screen,
    theme::prelude::*,
};

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Menu::LevelSelect), spawn_level_menu);
    app.add_systems(
        Update,
        go_back.run_if(in_state(Menu::LevelSelect).and(input_just_pressed(KeyCode::Escape))),
    );
}

fn spawn_level_menu(mut commands: Commands) {
    commands.spawn((
        widget::ui_root("Levels Menu"),
        GlobalZIndex(2),
        StateScoped(Menu::LevelSelect),
        children![
            widget::header("Level Select"),
            widget::button("Level 1", spawn_level_1),
            widget::button("Level 2", spawn_level_2),
            widget::button("Level 3", spawn_level_3),
            widget::button("Level 4", spawn_level_4),
            widget::button("Level 5", spawn_level_5),
            widget::button("Back", go_back_on_click),
        ],
    ));
}

fn spawn_level_1(
    _: Trigger<Pointer<Click>>,
    mut next_screen: ResMut<NextState<Screen>>,
    mut commands: Commands,
) {
    next_screen.set(Screen::Gameplay);
    commands.insert_resource(WantLevel(1));
}

fn spawn_level_2(
    _: Trigger<Pointer<Click>>,
    mut next_screen: ResMut<NextState<Screen>>,
    mut commands: Commands,
) {
    next_screen.set(Screen::Gameplay);
    commands.insert_resource(WantLevel(2));
}

fn spawn_level_3(
    _: Trigger<Pointer<Click>>,
    mut next_screen: ResMut<NextState<Screen>>,
    mut commands: Commands,
) {
    next_screen.set(Screen::Gameplay);
    commands.insert_resource(WantLevel(3));
}

fn spawn_level_4(
    _: Trigger<Pointer<Click>>,
    mut next_screen: ResMut<NextState<Screen>>,
    mut commands: Commands,
) {
    next_screen.set(Screen::Gameplay);
    commands.insert_resource(WantLevel(4));
}

fn spawn_level_5(
    _: Trigger<Pointer<Click>>,
    mut next_screen: ResMut<NextState<Screen>>,
    mut commands: Commands,
) {
    next_screen.set(Screen::Gameplay);
    commands.insert_resource(WantLevel(5));
}

#[derive(Component, Reflect)]
#[reflect(Component)]
struct GlobalVolumeLabel;
