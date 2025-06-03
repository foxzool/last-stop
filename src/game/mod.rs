use crate::{game::level::spawn_level, screens::Screen};
use bevy::prelude::*;

mod grid;
mod interaction;
pub mod level;
mod validation;

pub(super) fn plugin(app: &mut App) {
    app.add_plugins((
        level::plugin,
        grid::GridPlugin,
        interaction::MouseInteractionPlugin,
        validation::ConnectionValidationPlugin,
    ));
    app.add_systems(OnEnter(Screen::Gameplay), spawn_level);
}
