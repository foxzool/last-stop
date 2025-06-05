// src/screens/title.rs
// level select menu

use crate::{menus::Menu, screens::Screen};
use bevy::prelude::*;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Screen::LevelSelect), open_level_select);
    app.add_systems(OnExit(Screen::LevelSelect), close_level_select);
}

fn open_level_select(mut next_menu: ResMut<NextState<Menu>>) {
    next_menu.set(Menu::LevelSelect);
}

fn close_level_select(mut next_menu: ResMut<NextState<Menu>>) {
    next_menu.set(Menu::Main);
}
