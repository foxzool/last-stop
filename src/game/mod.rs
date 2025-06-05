use bevy::prelude::*;

mod grid;
mod interaction;
pub mod level;
mod passenger;
mod validation;

pub(super) fn plugin(app: &mut App) {
    app.add_plugins((
        level::plugin,
        // grid::GridPlugin,
        // interaction::MouseInteractionPlugin,
        // validation::ConnectionValidationPlugin,
        // passenger::PassengerPlugin,
    ));
}
