use bevy::prelude::*;

mod grid;
mod interaction;
pub mod level;
mod passenger;
mod route;
mod validation;
pub(super) fn plugin(app: &mut App) {
    app.add_plugins((
        level::plugin,
        route::PathfindingPlugin,
        // grid::GridPlugin,
        // interaction::MouseInteractionPlugin,
        // validation::ConnectionValidationPlugin,
        // passenger::PassengerPlugin,
    ));
}
