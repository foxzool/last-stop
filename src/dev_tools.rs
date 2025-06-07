//! Development tools for the bus_puzzle. This plugin is only enabled in dev builds.

use crate::bus_puzzle::GameStateEnum;
use bevy::{dev_tools::states::log_transitions, prelude::*};

pub(super) fn plugin(app: &mut App) {
    // Log `GameStateEnum` state transitions.
    app.add_systems(Update, log_transitions::<GameStateEnum>);
}
