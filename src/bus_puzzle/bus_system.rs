// src/bus_puzzle/bus_system.rs - 公交车系统核心实现

use crate::bus_puzzle::GridPos;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============ 公交车实体组件 ============

#[derive(Component, Debug, Clone)]
#[allow(dead_code)]
pub struct BusVehicle {
    pub vehicle_id: String,
    pub route_id: String,
    pub capacity: u32,
    pub current_passengers: Vec<Entity>,
    pub current_stop_index: usize,
    pub direction: BusDirection,
    pub state: BusState,
    pub speed: f32,
    pub dwell_time: f32,
    pub remaining_dwell: f32,
    pub target_position: Option<Vec3>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BusDirection {
    Forward,  // 正向运行
    Backward, // 反向运行
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BusState {
    Traveling,     // 行驶中
    AtStop,        // 停站中
    Loading,       // 上下客
    TurningAround, // 终点调头
    Idle,          // 空闲状态
}

impl Default for BusVehicle {
    fn default() -> Self {
        Self {
            vehicle_id: "bus_001".to_string(),
            route_id: "route_001".to_string(),
            capacity: 30,
            current_passengers: Vec::new(),
            current_stop_index: 0,
            direction: BusDirection::Forward,
            state: BusState::Idle,
            speed: 80.0,     // 像素/秒
            dwell_time: 3.0, // 停站3秒
            remaining_dwell: 0.0,
            target_position: None,
        }
    }
}

// ============ 公交路线定义 ============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusRoute {
    pub route_id: String,
    pub route_name: String,
    pub stops: Vec<BusStop>,
    pub segments: Vec<GridPos>,
    pub frequency: f32, // 发车间隔(秒)
    pub is_circular: bool,
    pub vehicles: Vec<Entity>,
    pub max_vehicles: u32,
    pub color: Color,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusStop {
    pub position: GridPos,
    pub name: String,
    pub waiting_passengers: Vec<Entity>,
    pub platform_capacity: u32,
}

// ============ 路线发现和管理 ============

#[derive(Resource, Default)]
#[allow(dead_code)]
pub struct BusRoutesManager {
    pub routes: HashMap<String, BusRoute>,
    pub route_counter: u32,
    pub vehicle_counter: u32,
}

#[allow(dead_code)]
impl BusRoutesManager {
    #[allow(dead_code)]
    pub fn generate_route_id(&mut self) -> String {
        self.route_counter += 1;
        format!("route_{:03}", self.route_counter)
    }

    #[allow(dead_code)]
    pub fn generate_vehicle_id(&mut self) -> String {
        self.vehicle_counter += 1;
        format!("bus_{:03}", self.vehicle_counter)
    }

    pub fn add_route(&mut self, route: BusRoute) {
        self.routes.insert(route.route_id.clone(), route);
    }

    pub fn get_route(&self, route_id: &str) -> Option<&BusRoute> {
        self.routes.get(route_id)
    }

    #[allow(dead_code)]
    pub fn get_route_mut(&mut self, route_id: &str) -> Option<&mut BusRoute> {
        self.routes.get_mut(route_id)
    }
}
