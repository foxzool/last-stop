use crate::{game::level::spawn_level, screens::Screen};
use bevy::prelude::*;

pub mod level;
mod grid;

pub(super) fn plugin(app: &mut App) {
    app.add_plugins((level::plugin, grid::plugin));
    app.add_systems(OnEnter(Screen::Gameplay), spawn_level);
}
