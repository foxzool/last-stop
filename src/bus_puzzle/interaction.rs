// src/bus_puzzle/interaction.rs

// 使用相对路径引用同模块下的其他文件
use crate::bus_puzzle::{
    world_to_grid, AgentState, ButtonComponent, ButtonType, CameraController, DraggableSegment,
    GameState, GameStateEnum, GridPos, InputState, InventoryCountText, InventorySlot,
    InventoryUpdatedEvent, LevelCompletedEvent, LevelManager, ObjectiveCompletedEvent,
    ObjectiveCondition, ObjectiveTracker, ObjectiveType, PathNode, PathfindingAgent, PlacedSegment,
    RouteSegment, RouteSegmentType, SegmentPlacedEvent, SegmentPreview, SegmentRemovedEvent,
};
use bevy::{
    input::mouse::MouseWheel,
    prelude::{Val::Px, *},
    window::PrimaryWindow,
};

// 悬停提示组件
#[derive(Component)]
pub struct HoverTooltip;

// ============ 插件定义 ============

pub struct PuzzleInteractionPlugin;

impl Plugin for PuzzleInteractionPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(InputState::default())
            .insert_resource(CameraController::default())
            .add_systems(
                Update,
                (
                    handle_segment_placement,
                    handle_segment_rotation,
                    handle_segment_removal,
                    update_grid_preview,
                    update_objectives,
                    update_game_timer,
                    handle_level_completion,
                    handle_segment_hover_effects, // 新增：悬停效果
                    update_hover_tooltip,         // 新增：悬停提示
                )
                    .chain()
                    .run_if(in_state(GameStateEnum::Playing))
                    .run_if(not(is_paused)),
            )
            .add_systems(
                Update,
                (
                    handle_camera_controls,
                    update_mouse_world_position,
                    handle_button_interactions, // 统一的按钮交互处理
                )
                    .chain()
                    .run_if(in_state(GameStateEnum::Playing)),
            )
            .add_systems(
                PostUpdate,
                (
                    update_inventory_ui,
                    update_objectives_ui,
                    // update_score_display,
                )
                    .run_if(in_state(GameStateEnum::Playing)),
            );
    }
}

fn is_paused(game_state: Res<GameState>) -> bool {
    game_state.is_paused
}

// ============ 输入处理系统 ============

fn update_mouse_world_position(
    mut input_state: ResMut<InputState>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    game_state: Res<GameState>,
    level_manager: Res<LevelManager>,
    keyboard_input: Res<ButtonInput<KeyCode>>, // 添加键盘输入用于调试
) -> Result {
    let window = windows.single()?;
    let (camera, camera_transform) = camera_query.single()?;

    if let Some(cursor_pos) = window.cursor_position() {
        if let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) {
            input_state.mouse_world_pos = world_pos.extend(0.0);

            if let Some(level_data) = &game_state.current_level {
                let grid_pos = world_to_grid(
                    input_state.mouse_world_pos,
                    level_manager.tile_size,
                    level_data.grid_size.0,
                    level_data.grid_size.1,
                );
                input_state.grid_cursor_pos = Some(grid_pos);

                // F10 调试坐标转换精度
                if keyboard_input.just_pressed(KeyCode::F10) {
                    crate::bus_puzzle::debug_coordinate_conversion(
                        input_state.mouse_world_pos,
                        level_manager.tile_size,
                        level_data.grid_size.0,
                        level_data.grid_size.1,
                    );

                    // 显示当前鼠标位置信息
                    info!("鼠标调试信息:");
                    info!("  屏幕坐标: {:?}", cursor_pos);
                    info!("  世界坐标: {:?}", input_state.mouse_world_pos);
                    info!("  网格坐标: {:?}", grid_pos);

                    // 计算网格中心的世界坐标
                    let grid_center_world = grid_pos.to_world_pos(
                        level_manager.tile_size,
                        level_data.grid_size.0,
                        level_data.grid_size.1,
                    );
                    info!("  网格中心世界坐标: {:?}", grid_center_world);

                    let distance = input_state.mouse_world_pos.distance(grid_center_world);
                    info!("  鼠标距离网格中心: {:.2} 像素", distance);

                    if distance > level_manager.tile_size * 0.3 {
                        warn!("  ⚠️ 鼠标距离网格中心较远，可能存在坐标转换问题");
                    } else {
                        info!("  ✅ 坐标转换正常");
                    }
                }
            } else {
                let grid_pos =
                    world_to_grid(input_state.mouse_world_pos, level_manager.tile_size, 10, 8);
                input_state.grid_cursor_pos = Some(grid_pos);
            }
        }
    }

    Ok(())
}

fn handle_camera_controls(
    mut camera_controller: ResMut<CameraController>,
    mut camera_query: Query<&mut Transform, With<Camera>>,
    mut mouse_wheel_events: EventReader<MouseWheel>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) -> Result {
    let mut camera_transform = camera_query.single_mut()?;
    let dt = time.delta_secs();

    // 处理缩放
    for wheel_event in mouse_wheel_events.read() {
        camera_controller.zoom *= 1.0 - wheel_event.y * camera_controller.zoom_speed;
        camera_controller.zoom = camera_controller
            .zoom
            .clamp(camera_controller.min_zoom, camera_controller.max_zoom);

        camera_transform.scale = Vec3::splat(camera_controller.zoom);
    }

    // 处理平移
    let mut movement = Vec3::ZERO;
    if keyboard_input.pressed(KeyCode::KeyW) || keyboard_input.pressed(KeyCode::ArrowUp) {
        movement.y += 1.0;
    }
    if keyboard_input.pressed(KeyCode::KeyS) || keyboard_input.pressed(KeyCode::ArrowDown) {
        movement.y -= 1.0;
    }
    if keyboard_input.pressed(KeyCode::KeyA) || keyboard_input.pressed(KeyCode::ArrowLeft) {
        movement.x -= 1.0;
    }
    if keyboard_input.pressed(KeyCode::KeyD) || keyboard_input.pressed(KeyCode::ArrowRight) {
        movement.x += 1.0;
    }

    if movement != Vec3::ZERO {
        movement = movement.normalize();
        camera_transform.translation +=
            movement * camera_controller.pan_speed * dt * camera_controller.zoom;
    }

    Ok(())
}

// ============ 交互处理系统 ============

fn handle_button_interactions(
    button_query: Query<(&Interaction, &ButtonComponent), (Changed<Interaction>, With<Button>)>,
    mut input_state: ResMut<InputState>,
    mut next_state: ResMut<NextState<GameStateEnum>>,
    mut app_exit_events: EventWriter<AppExit>,
    mut level_manager: ResMut<LevelManager>,
    game_state: Res<GameState>,
) {
    for (interaction, button_component) in button_query.iter() {
        if matches!(*interaction, Interaction::Pressed) {
            // 处理按钮逻辑
            match &button_component.button_type {
                ButtonType::StartGame => {
                    next_state.set(GameStateEnum::Playing);
                }
                ButtonType::QuitGame => {
                    app_exit_events.write(AppExit::Success);
                }
                ButtonType::PauseGame => {
                    next_state.set(GameStateEnum::Paused);
                }
                ButtonType::ResumeGame => {
                    next_state.set(GameStateEnum::Playing);
                }
                ButtonType::RestartLevel => {
                    next_state.set(GameStateEnum::Loading);
                }
                ButtonType::MainMenu => {
                    next_state.set(GameStateEnum::MainMenu);
                }
                ButtonType::NextLevel => {
                    info!("next level");
                    level_manager.current_level_index += 1;
                    if level_manager.current_level_index < level_manager.available_levels.len() {
                        next_state.set(GameStateEnum::Loading);
                    } else {
                        next_state.set(GameStateEnum::MainMenu);
                    }
                }
                ButtonType::InventorySlot(segment_type) => {
                    let available_count = game_state
                        .player_inventory
                        .get(segment_type)
                        .copied()
                        .unwrap_or(0);

                    if available_count > 0 {
                        input_state.selected_segment = Some(*segment_type);
                        info!("Selected route segment: {:?}", segment_type);
                    } else {
                        warn!("Insufficient inventory: {:?}", segment_type);
                    }
                }
                _ => {}
            }
        }
    }
}

fn handle_segment_placement(
    mut commands: Commands,
    input_state: ResMut<InputState>,
    mut game_state: ResMut<GameState>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    asset_server: Res<AssetServer>,
    mut segment_placed_events: EventWriter<SegmentPlacedEvent>,
    mut inventory_updated_events: EventWriter<InventoryUpdatedEvent>,
    level_manager: Res<LevelManager>,
) {
    if mouse_button_input.just_released(MouseButton::Left) {
        if let (Some(segment_type), Some(grid_pos)) =
            (input_state.selected_segment, input_state.grid_cursor_pos)
        {
            if is_valid_placement(&game_state, grid_pos, &segment_type) {
                if let Some(&available_count) = game_state.player_inventory.get(&segment_type) {
                    if available_count > 0 {
                        let rotation = 0;
                        let cost = segment_type.get_cost();

                        let entity = spawn_route_segment(
                            &mut commands,
                            &asset_server,
                            grid_pos,
                            segment_type,
                            rotation,
                            &level_manager,
                        );

                        game_state.placed_segments.insert(
                            grid_pos,
                            PlacedSegment {
                                segment_type,
                                rotation,
                                entity,
                                cost,
                            },
                        );

                        game_state.total_cost += cost;
                        *game_state.player_inventory.get_mut(&segment_type).unwrap() -= 1;

                        segment_placed_events.write(SegmentPlacedEvent {
                            position: grid_pos,
                            segment_type,
                            rotation,
                        });

                        inventory_updated_events.write(InventoryUpdatedEvent {
                            segment_type,
                            new_count: game_state.player_inventory[&segment_type],
                        });

                        info!("在 {:?} 放置了 {:?}", grid_pos, segment_type);
                    } else {
                        warn!("库存不足：{:?}", segment_type);
                    }
                } else {
                    warn!("没有 {:?} 类型的路线段", segment_type);
                }
            } else {
                warn!("无法在 {:?} 放置 {:?}", grid_pos, segment_type);
            }
        }
    }
}

fn handle_segment_rotation(
    mut game_state: ResMut<GameState>,
    mut route_segments: Query<(&mut Transform, &mut RouteSegment)>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    input_state: Res<InputState>,
) {
    if mouse_button_input.just_pressed(MouseButton::Right) {
        if let Some(grid_pos) = input_state.grid_cursor_pos {
            if let Some(placed_segment) = game_state.placed_segments.get_mut(&grid_pos) {
                placed_segment.rotation = (placed_segment.rotation + 90) % 360;

                // 同时更新Transform和RouteSegment组件
                if let Ok((mut transform, mut route_segment)) =
                    route_segments.get_mut(placed_segment.entity)
                {
                    route_segment.rotation = placed_segment.rotation;
                    transform.rotation = Quat::from_rotation_z(
                        (placed_segment.rotation as f32) * std::f32::consts::PI / 180.0,
                    );
                }

                info!(
                    "Rotated route segment to {} degrees at {:?}",
                    placed_segment.rotation, grid_pos
                );
            }
        }
    }
}

fn handle_segment_removal(
    mut commands: Commands,
    mut game_state: ResMut<GameState>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    input_state: Res<InputState>,
    mut segment_removed_events: EventWriter<SegmentRemovedEvent>,
    mut inventory_updated_events: EventWriter<InventoryUpdatedEvent>,
) {
    if keyboard_input.just_pressed(KeyCode::Delete) || keyboard_input.just_pressed(KeyCode::KeyX) {
        if let Some(grid_pos) = input_state.grid_cursor_pos {
            if let Some(placed_segment) = game_state.placed_segments.remove(&grid_pos) {
                commands.entity(placed_segment.entity).despawn();

                *game_state
                    .player_inventory
                    .entry(placed_segment.segment_type)
                    .or_insert(0) += 1;
                game_state.total_cost -= placed_segment.cost;

                segment_removed_events.write(SegmentRemovedEvent { position: grid_pos });
                inventory_updated_events.write(InventoryUpdatedEvent {
                    segment_type: placed_segment.segment_type,
                    new_count: game_state.player_inventory[&placed_segment.segment_type],
                });

                info!("移除了在 {:?} 的路线段", grid_pos);
            }
        }
    }
}

fn update_grid_preview(
    mut commands: Commands,
    input_state: Res<InputState>,
    game_state: Res<GameState>,
    existing_previews: Query<Entity, With<SegmentPreview>>,
    asset_server: Res<AssetServer>,
    level_manager: Res<LevelManager>,
) {
    // 清除现有预览
    for entity in existing_previews.iter() {
        commands.entity(entity).despawn();
    }

    // 如果有选中的路线段和有效的网格位置，显示预览
    if let (Some(segment_type), Some(grid_pos)) =
        (input_state.selected_segment, input_state.grid_cursor_pos)
    {
        let is_valid = is_valid_placement(&game_state, grid_pos, &segment_type);

        // 获取世界坐标，需要网格尺寸信息
        let world_pos = if let Some(level_data) = &game_state.current_level {
            grid_pos.to_world_pos(
                level_manager.tile_size,
                level_data.grid_size.0,
                level_data.grid_size.1,
            )
        } else {
            grid_pos.to_world_pos(level_manager.tile_size, 10, 8) // 默认尺寸
        };

        // 选择预览材质颜色
        let color = if is_valid {
            Color::srgba(0.0, 1.0, 0.0, 0.7) // 绿色半透明
        } else {
            Color::srgba(1.0, 0.0, 0.0, 0.7) // 红色半透明
        };

        commands.spawn((
            Sprite {
                image: asset_server.load(segment_type.get_texture_path()),
                color,
                ..default()
            },
            Transform::from_translation(world_pos + Vec3::Z * 0.1),
            SegmentPreview {
                segment_type,
                rotation: 0,
                target_position: grid_pos,
            },
        ));
    }
}

// ============ 游戏逻辑系统 ============

fn update_objectives(
    mut game_state: ResMut<GameState>,
    passengers: Query<&PathfindingAgent>,
    mut objective_completed_events: EventWriter<ObjectiveCompletedEvent>,
) {
    let objectives_len = if let Some(level_data) = &game_state.current_level {
        level_data.objectives.len()
    } else {
        return;
    };

    if game_state.objectives_completed.len() < objectives_len {
        game_state
            .objectives_completed
            .resize(objectives_len, false);
    }

    let mut completed_objective_indices = Vec::new();

    // Phase 1: Check objectives and collect indices of newly completed ones.
    // This phase only reads from game_state related to objectives.
    if let Some(level_data) = &game_state.current_level {
        // Immutable borrow of game_state.current_level via level_data.
        // This borrow lasts for the scope of this if-let block.
        let objectives = &level_data.objectives;

        for (i, objective) in objectives.iter().enumerate() {
            // Read from game_state.objectives_completed (immutable).
            // This is fine alongside the immutable borrow of game_state.current_level.
            if !game_state.objectives_completed[i] {
                // Pass an immutable reference to game_state.
                // Bevy systems auto-deref ResMut<T> to &T or &mut T as needed.
                // Here, &*game_state explicitly gives &GameState.
                let is_completed = check_objective_completion(objective, &game_state, &passengers);

                if is_completed {
                    completed_objective_indices.push(i);
                }
            }
        }
    } // Immutable borrow of game_state.current_level (level_data) ends here.

    // Phase 2: Apply updates for completed objectives.
    // All conflicting immutable borrows from Phase 1 have ended.
    // We can now safely mutate game_state.
    for index in completed_objective_indices {
        // It's good practice to check again, though with ResMut it might be redundant
        // if this system is the sole writer to objectives_completed.
        if game_state.objectives_completed.get(index) == Some(&false) {
            game_state.objectives_completed[index] = true;
            objective_completed_events.write(ObjectiveCompletedEvent {
                objective_index: index,
            });

            // Log completion with description. This requires another short immutable borrow.
            if let Some(level_data) = &game_state.current_level {
                // Short immutable borrow
                if let Some(objective) = level_data.objectives.get(index) {
                    info!("目标完成: {}", objective.description);
                } else {
                    // This case should ideally not happen if index is valid.
                    info!("目标 {} 完成 (描述信息获取失败)", index);
                }
            } else {
                // This case should also ideally not happen if we passed phase 1.
                info!("目标 {} 完成 (关卡数据获取失败)", index);
            }
        }
    }
}

fn update_game_timer(mut game_state: ResMut<GameState>, time: Res<Time>) {
    if !game_state.is_paused {
        game_state.game_time += time.delta_secs();
    }
}

fn handle_level_completion(
    game_state: Res<GameState>,
    mut level_completed_events: EventWriter<LevelCompletedEvent>,
) {
    if let Some(_level_data) = &game_state.current_level {
        let all_completed = game_state
            .objectives_completed
            .iter()
            .all(|&completed| completed);

        if all_completed && !game_state.objectives_completed.is_empty() {
            // 在关卡完成时重新计算最终分数，确保使用最新的分数
            let final_score = game_state.score.total_score;

            // 发送关卡完成事件，使用计算好的最终分数
            level_completed_events.write(LevelCompletedEvent {
                final_score,
                completion_time: game_state.game_time,
            });

            info!(
                "关卡完成！最终分数: {}, 用时: {:.1}s",
                final_score, game_state.game_time
            );
        }
    }
}

// ============ UI 更新系统 ============

fn update_inventory_ui(
    // game_state: Res<GameState>,
    mut inventory_slots: Query<(&mut InventorySlot, &mut Sprite)>,
    mut inventory_count_text: Query<(&InventoryCountText, &mut Text)>,
    mut inventory_updated_events: EventReader<InventoryUpdatedEvent>,
) {
    for event in inventory_updated_events.read() {
        // 更新库存槽位
        for (mut slot, mut sprite) in inventory_slots.iter_mut() {
            if slot.segment_type.as_ref() == Some(&event.segment_type) {
                slot.available_count = event.new_count;

                sprite.color = if event.new_count > 0 {
                    Color::WHITE
                } else {
                    Color::srgb(0.5, 0.5, 0.5)
                };
            }
        }

        // 更新数量文本
        for (count_text, mut text) in inventory_count_text.iter_mut() {
            if count_text.segment_type == event.segment_type {
                *text = Text::new(format!("{}", event.new_count));
            }
        }
    }
}

fn update_objectives_ui(
    mut objective_trackers: Query<(&ObjectiveTracker, &mut Sprite)>,
    mut objective_completed_events: EventReader<ObjectiveCompletedEvent>,
) {
    for event in objective_completed_events.read() {
        for (tracker, mut sprite) in objective_trackers.iter_mut() {
            if tracker.objective_index == event.objective_index {
                sprite.color = Color::srgb(0.0, 1.0, 0.0);
            }
        }
    }
}

fn is_valid_placement(
    game_state: &GameState,
    position: GridPos,
    segment_type: &RouteSegmentType,
) -> bool {
    // 检查位置是否已被占用
    if game_state.placed_segments.contains_key(&position) {
        return false;
    }

    // 检查地形限制
    if let Some(level_data) = &game_state.current_level {
        if let Some(terrain_type) = level_data.terrain.get(&position) {
            match terrain_type {
                super::TerrainType::Building => return false,
                super::TerrainType::Water => {
                    return matches!(segment_type, RouteSegmentType::Bridge);
                }
                super::TerrainType::Mountain => {
                    return matches!(segment_type, RouteSegmentType::Tunnel);
                }
                _ => {}
            }
        }

        // 检查网格边界
        let (width, height) = level_data.grid_size;
        if position.x < 0
            || position.y < 0
            || position.x >= width as i32
            || position.y >= height as i32
        {
            return false;
        }
    }

    true
}

fn spawn_route_segment(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    position: GridPos,
    segment_type: RouteSegmentType,
    rotation: u32,
    level_manager: &LevelManager,
) -> Entity {
    let world_pos = if let Some(level_data) = &level_manager.current_level {
        position.to_world_pos(
            level_manager.tile_size,
            level_data.grid_size.0,
            level_data.grid_size.1,
        )
    } else {
        position.to_world_pos(level_manager.tile_size, 10, 8)
    };

    let texture_path = segment_type.get_texture_path();

    commands
        .spawn((
            Sprite::from_image(asset_server.load(texture_path)),
            Transform::from_translation(world_pos + Vec3::Z * 0.5).with_rotation(
                Quat::from_rotation_z((rotation as f32) * std::f32::consts::PI / 180.0),
            ),
            RouteSegment {
                grid_pos: position,
                segment_type,
                rotation,
                is_active: true,
            },
            DraggableSegment {
                segment_type,
                rotation,
                is_being_dragged: false,
                is_placed: true,
                cost: segment_type.get_cost(),
            },
        ))
        .id()
}

fn check_objective_completion(
    objective: &ObjectiveCondition,
    game_state: &GameState,
    passengers: &Query<&PathfindingAgent>,
) -> bool {
    match &objective.condition_type {
        ObjectiveType::ConnectAllPassengers => {
            let stats = &game_state.passenger_stats;
            stats.total_spawned > 0 && stats.total_arrived == stats.total_spawned
        }
        ObjectiveType::MaxTransfers(max_transfers) => passengers
            .iter()
            .filter(|agent| matches!(agent.state, AgentState::Arrived))
            .all(|agent| count_transfers_in_path(&agent.current_path) <= *max_transfers),
        ObjectiveType::MaxSegments(max_segments) => {
            game_state.placed_segments.len() <= (*max_segments as usize)
        }
        ObjectiveType::MaxCost(max_cost) => game_state.total_cost <= *max_cost,
        ObjectiveType::TimeLimit(time_limit) => game_state.game_time <= *time_limit,
        ObjectiveType::MinEfficiency(min_efficiency) => {
            calculate_network_efficiency(game_state, passengers) >= *min_efficiency
        }
        ObjectiveType::PassengerSatisfaction(min_satisfaction) => {
            calculate_passenger_satisfaction(passengers) >= *min_satisfaction
        }
    }
}

fn count_transfers_in_path(path: &[PathNode]) -> u32 {
    let mut transfers = 0;
    let mut current_route_id = None;

    for node in path {
        if let Some(route_id) = &node.route_id {
            if let Some(prev_route_id) = &current_route_id {
                if route_id != prev_route_id {
                    transfers += 1;
                }
            }
            current_route_id = Some(route_id.clone());
        }
    }

    transfers
}

pub fn calculate_network_efficiency(
    game_state: &GameState,
    passengers: &Query<&PathfindingAgent>,
) -> f32 {
    if passengers.is_empty() {
        return 0.0;
    }

    // 计算乘客完成率
    let total_passengers = passengers.iter().count() as f32;
    let arrived_passengers = passengers
        .iter()
        .filter(|agent| matches!(agent.state, AgentState::Arrived))
        .count() as f32;

    let completion_rate = if total_passengers > 0.0 {
        arrived_passengers / total_passengers
    } else {
        0.0
    };

    // 计算平均路径长度效率
    let path_efficiency = if arrived_passengers > 0.0 {
        let total_path_length: f32 = passengers
            .iter()
            .filter(|agent| matches!(agent.state, AgentState::Arrived))
            .map(|agent| agent.current_path.len() as f32)
            .sum();

        let average_path_length = total_path_length / arrived_passengers;
        // 路径长度越短，效率越高（最小长度设为2，避免除零）
        1.0 / (average_path_length.max(2.0) / 2.0)
    } else {
        0.0
    };

    // 计算成本效率
    let total_segments = game_state.placed_segments.len() as f32;
    let cost_efficiency = if total_segments > 0.0 {
        // 段数越少，效率越高
        1.0 / (total_segments / 10.0).max(0.1)
    } else {
        0.0
    };

    // 综合效率评分（权重：完成率60%，路径效率25%，成本效率15%）
    let overall_efficiency =
        completion_rate * 0.6 + path_efficiency * 0.25 + cost_efficiency * 0.15;

    // 返回0-10范围内的效率分数
    overall_efficiency * 10.0
}

fn calculate_passenger_satisfaction(passengers: &Query<&PathfindingAgent>) -> f32 {
    if passengers.is_empty() {
        return 1.0;
    }

    let satisfied_count = passengers
        .iter()
        .filter(|agent| matches!(agent.state, AgentState::Arrived))
        .count();

    satisfied_count as f32 / passengers.iter().count() as f32
}

/// 处理路线段的鼠标悬停高亮效果
fn handle_segment_hover_effects(
    input_state: Res<InputState>,
    game_state: Res<GameState>,
    mut route_segments: Query<(&mut Sprite, &RouteSegment)>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    // 获取鼠标当前网格位置
    let mouse_grid_pos = if let Some(grid_pos) = input_state.grid_cursor_pos {
        grid_pos
    } else {
        // 没有有效鼠标位置，重置所有路线段颜色
        for (mut sprite, _) in route_segments.iter_mut() {
            sprite.color = Color::WHITE;
        }
        return;
    };

    // 检测特殊键状态
    let is_delete_mode =
        keyboard_input.pressed(KeyCode::Delete) || keyboard_input.pressed(KeyCode::KeyX);
    let has_selected_segment = input_state.selected_segment.is_some();

    for (mut sprite, segment) in route_segments.iter_mut() {
        let is_hovered = segment.grid_pos == mouse_grid_pos;
        let is_placed_segment = game_state.placed_segments.contains_key(&segment.grid_pos);

        if is_hovered && is_placed_segment && !has_selected_segment {
            // 鼠标悬停在已放置的路线段上
            if is_delete_mode {
                // 删除模式 - 红色高亮
                sprite.color = Color::srgb(1.3, 0.6, 0.6);
            } else {
                // 正常悬停 - 黄色高亮
                sprite.color = Color::srgb(1.3, 1.3, 0.7);
            }
        } else {
            // 重置为正常颜色
            sprite.color = Color::WHITE;
        }
    }
}

/// 更新悬停提示工具栏
fn update_hover_tooltip(
    mut commands: Commands,
    input_state: Res<InputState>,
    game_state: Res<GameState>,
    existing_tooltips: Query<Entity, With<HoverTooltip>>,
    asset_server: Res<AssetServer>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    // 清除现有的提示
    for entity in existing_tooltips.iter() {
        commands.entity(entity).despawn();
    }

    // 检查是否悬停在已放置的路线段上
    if let Some(grid_pos) = input_state.grid_cursor_pos {
        if let Some(placed_segment) = game_state.placed_segments.get(&grid_pos) {
            // 如果没有选中其他路线段，显示操作提示
            if input_state.selected_segment.is_none() {
                spawn_hover_tooltip(
                    &mut commands,
                    &asset_server,
                    placed_segment,
                    &keyboard_input,
                );
            }
        }
    }
}

/// 生成悬停提示UI
fn spawn_hover_tooltip(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    placed_segment: &PlacedSegment,
    keyboard_input: &Res<ButtonInput<KeyCode>>,
) {
    // 确定提示文本和颜色
    let (tooltip_text, tooltip_color) =
        if keyboard_input.pressed(KeyCode::Delete) || keyboard_input.pressed(KeyCode::KeyX) {
            ("按 Delete 或 X 删除此路线段", Color::srgb(1.0, 0.6, 0.6))
        } else {
            ("右键旋转 | Delete/X 删除", Color::srgb(1.0, 1.0, 0.8))
        };

    // 创建提示框
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Px(120.0),
                left: Px(20.0),
                padding: UiRect::all(Px(8.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
            HoverTooltip,
            Name::new("Segment Hover Tooltip"),
        ))
        .with_children(|parent| {
            // 路线段信息
            parent.spawn((
                Text::new(format!(
                    "{:?} (旋转: {}°, 成本: {})",
                    placed_segment.segment_type, placed_segment.rotation, placed_segment.cost
                )),
                TextFont {
                    font: asset_server.load("fonts/quan.ttf"),
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.9, 0.9)),
            ));

            // 操作提示
            parent.spawn((
                Text::new(tooltip_text),
                TextFont {
                    font: asset_server.load("fonts/quan.ttf"),
                    font_size: 12.0,
                    ..default()
                },
                TextColor(tooltip_color),
            ));
        });
}
