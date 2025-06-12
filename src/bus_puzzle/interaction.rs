// src/bus_puzzle/interaction.rs

// 使用相对路径引用同模块下的其他文件
use crate::bus_puzzle::{
    world_to_grid, AgentState, ButtonComponent, ButtonType, CameraController, CurrentLanguage,
    DraggableSegment, GameState, GameStateEnum, GridPos, InputState, InventoryCountText,
    InventorySlot, InventoryUpdatedEvent, Language, LevelCompletedEvent, LevelManager,
    ObjectiveCompletedEvent, ObjectiveCondition, ObjectiveTracker, ObjectiveType, PathNode,
    PathfindingAgent, PlacedSegment, RotationHintUI, RouteSegment, RouteSegmentType,
    SegmentPlacedEvent, SegmentPreview, SegmentRemovedEvent,
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
                    handle_segment_hover_effects,       // 新增：悬停效果
                    update_hover_tooltip,               // 新增：悬停提示
                    reset_preview_rotation_on_deselect, // 改进的取消选择
                    show_rotation_hint_ui,              // 中英文旋转提示UI
                    update_rotation_hint_text,          // 实时更新提示文本
                    update_rotation_angle_display,      // 更新角度显示
                    handle_inventory_selection,
                    handle_quick_rotation_keys,
                )
                    .chain()
                    .run_if(in_state(GameStateEnum::Playing))
                    .run_if(not(is_paused)),
            )
            // 新增：在状态变化时清理选择
            .add_systems(OnEnter(GameStateEnum::Paused), clear_segment_selection)
            .add_systems(OnEnter(GameStateEnum::MainMenu), clear_segment_selection)
            .add_systems(OnEnter(GameStateEnum::Loading), clear_segment_selection)
            .add_systems(
                Update,
                (
                    handle_camera_controls,
                    update_mouse_world_position,
                    // 将 handle_button_interactions 移动到全局，但添加状态检查
                    handle_button_interactions,
                )
                    .chain(),
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
                    debug_coordinate_conversion(
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
    current_state: Res<State<GameStateEnum>>,
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
                    // 重要：只在游戏进行状态下处理库存槽位按钮
                    if matches!(current_state.get(), GameStateEnum::Playing)
                        && !game_state.is_paused
                    {
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
                    } else {
                        // 在暂停状态下，不处理库存槽位按钮
                        trace!("Inventory slot button ignored - game is paused or not playing");
                    }
                }
                _ => {}
            }
        }
    }
}

fn handle_segment_placement(
    mut commands: Commands,
    mut input_state: ResMut<InputState>,
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
                        // 使用预览旋转角度
                        let rotation = input_state.preview_rotation;
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

                        info!(
                            "在 {:?} 放置了 {:?}，旋转角度: {}°",
                            grid_pos, segment_type, rotation
                        );

                        // 放置后重置预览旋转
                        input_state.preview_rotation = 0;
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
    mut input_state: ResMut<InputState>,
    mut game_state: ResMut<GameState>,
    mut route_segments: Query<(&mut Transform, &mut RouteSegment)>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    keyboard_input: Res<ButtonInput<KeyCode>>, // 新增：键盘输入
) {
    let should_rotate = mouse_button_input.just_pressed(MouseButton::Right)
        || keyboard_input.just_pressed(KeyCode::KeyR)
        || keyboard_input.just_pressed(KeyCode::Space); // 多种旋转方式

    if should_rotate {
        if let Some(grid_pos) = input_state.grid_cursor_pos {
            // 检查是否有已放置的路线段
            if let Some(placed_segment) = game_state.placed_segments.get_mut(&grid_pos) {
                // 旋转已放置的路线段
                placed_segment.rotation = (placed_segment.rotation + 90) % 360;

                if let Ok((mut transform, mut route_segment)) =
                    route_segments.get_mut(placed_segment.entity)
                {
                    route_segment.rotation = placed_segment.rotation;
                    transform.rotation = Quat::from_rotation_z(
                        (placed_segment.rotation as f32) * std::f32::consts::PI / 180.0,
                    );
                }

                info!(
                    "旋转已放置路线段到 {} 度，位置 {:?}",
                    placed_segment.rotation, grid_pos
                );
            } else if input_state.selected_segment.is_some() {
                // 旋转预览中的路线段
                input_state.preview_rotation = (input_state.preview_rotation + 90) % 360;
                info!("旋转预览路线段到 {} 度", input_state.preview_rotation);
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

// 改进的预览验证函数，支持旋转预览
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
        // 使用增强的验证函数，考虑旋转角度
        let is_valid = is_valid_placement_with_rotation(
            &game_state,
            grid_pos,
            &segment_type,
            input_state.preview_rotation,
        );

        // 获取世界坐标
        let world_pos = if let Some(level_data) = &game_state.current_level {
            grid_pos.to_world_pos(
                level_manager.tile_size,
                level_data.grid_size.0,
                level_data.grid_size.1,
            )
        } else {
            grid_pos.to_world_pos(level_manager.tile_size, 10, 8)
        };

        // 根据验证结果选择颜色，还可以添加连接点可视化
        let base_color = if is_valid {
            Color::srgba(0.0, 1.0, 0.0, 0.7) // 绿色半透明
        } else {
            Color::srgba(1.0, 0.0, 0.0, 0.7) // 红色半透明
        };

        // 应用预览旋转
        let rotation_quat = Quat::from_rotation_z(
            (input_state.preview_rotation as f32) * std::f32::consts::PI / 180.0,
        );

        // 生成主预览
        commands.spawn((
            Sprite {
                image: asset_server.load(segment_type.get_texture_path()),
                color: base_color,
                ..default()
            },
            Transform::from_translation(world_pos + Vec3::Z * 0.1).with_rotation(rotation_quat),
            SegmentPreview {
                segment_type,
                rotation: input_state.preview_rotation,
                target_position: grid_pos,
            },
        ));

        // 可选：显示连接点预览（小圆点）
        let connection_positions =
            segment_type.get_connection_positions(grid_pos, input_state.preview_rotation);

        for conn_pos in connection_positions {
            let conn_world_pos = if let Some(level_data) = &game_state.current_level {
                conn_pos.to_world_pos(
                    level_manager.tile_size,
                    level_data.grid_size.0,
                    level_data.grid_size.1,
                )
            } else {
                conn_pos.to_world_pos(level_manager.tile_size, 10, 8)
            };

            // 检查这个连接点是否有效
            let connection_valid = game_state
                .placed_segments
                .get(&conn_pos)
                .map(|seg| {
                    seg.segment_type
                        .has_connection_to(conn_pos, grid_pos, seg.rotation)
                })
                .unwrap_or(true); // 如果没有路线段则认为有效

            let connection_color = if connection_valid {
                Color::srgba(0.0, 1.0, 1.0, 0.8) // 青色：有效连接
            } else {
                Color::srgba(1.0, 1.0, 0.0, 0.8) // 黄色：可能的连接冲突
            };

            commands.spawn((
                Sprite {
                    color: connection_color,
                    custom_size: Some(Vec2::new(8.0, 8.0)), // 小方块
                    ..default()
                },
                Transform::from_translation(conn_world_pos + Vec3::Z * 0.2)
                    .with_rotation(rotation_quat),
                SegmentPreview {
                    segment_type,
                    rotation: input_state.preview_rotation,
                    target_position: conn_pos, // 连接点位置
                },
            ));
        }
    }
}

// 重置预览旋转的辅助函数（当取消选择时调用）
fn reset_preview_rotation_on_deselect(
    mut input_state: ResMut<InputState>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    // 按 ESC 键取消选择并重置旋转
    if keyboard_input.just_pressed(KeyCode::Escape) {
        input_state.selected_segment = None;
        input_state.preview_rotation = 0;
        info!("取消选择，重置预览旋转");
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
    input_state: Res<InputState>, // 新增：获取输入状态
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

    // 新增：更新选中状态的视觉反馈
    for (slot, mut sprite) in inventory_slots.iter_mut() {
        if let Some(slot_segment_type) = &slot.segment_type {
            let is_selected = input_state.selected_segment == Some(*slot_segment_type);

            if is_selected {
                // 选中状态：高亮边框效果
                sprite.color = Color::srgb(1.2, 1.2, 0.8); // 淡黄色高亮
            } else if slot.available_count > 0 {
                // 有库存：正常白色
                sprite.color = Color::WHITE;
            } else {
                // 无库存：灰色
                sprite.color = Color::srgb(0.5, 0.5, 0.5);
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

pub fn is_valid_placement(
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

/// 新增：坐标转换调试函数
fn debug_coordinate_conversion(world_pos: Vec3, tile_size: f32, grid_width: u32, grid_height: u32) {
    info!("=== 坐标转换调试 ===");
    info!("  世界坐标: {:?}", world_pos);
    info!("  瓦片大小: {}", tile_size);
    info!("  网格尺寸: {}x{}", grid_width, grid_height);

    let grid_pos = world_to_grid(world_pos, tile_size, grid_width, grid_height);
    info!("  计算网格坐标: {:?}", grid_pos);

    // 反向计算检验
    let back_world_pos = grid_pos.to_world_pos(tile_size, grid_width, grid_height);
    info!("  反向世界坐标: {:?}", back_world_pos);

    let distance = world_pos.distance(back_world_pos);
    info!("  坐标差异: {:.2} 像素", distance);

    if distance > tile_size * 0.1 {
        warn!("  ⚠️ 坐标转换精度可能有问题");
    } else {
        info!("  ✅ 坐标转换精度正常");
    }
}

/// 清理路线段选择状态 - 在暂停游戏或切换状态时调用
fn clear_segment_selection(
    mut commands: Commands,
    mut input_state: ResMut<InputState>,
    existing_previews: Query<Entity, With<SegmentPreview>>,
    existing_tooltips: Query<Entity, With<HoverTooltip>>,
    mut inventory_slots: Query<(&InventorySlot, &mut Sprite, &mut BorderColor)>, /* 更新：同时处理边框 */
) {
    // 清空选中的路线段
    if input_state.selected_segment.is_some() {
        info!("清理路线段选择状态: {:?}", input_state.selected_segment);
        input_state.selected_segment = None;
    }

    // 清空拖拽状态
    input_state.is_dragging = false;
    input_state.drag_entity = None;

    // 清除所有预览实体
    for entity in existing_previews.iter() {
        commands.entity(entity).despawn();
    }

    // 清除所有悬停提示
    for entity in existing_tooltips.iter() {
        commands.entity(entity).despawn();
    }

    // 更新：重置库存槽位的视觉状态（包括边框）
    for (slot, mut sprite, mut border_color) in inventory_slots.iter_mut() {
        // 重置边框颜色为正常白色
        *border_color = Color::WHITE.into();

        // 重置背景颜色
        if slot.available_count > 0 {
            sprite.color = Color::WHITE; // 重置为正常白色
        } else {
            sprite.color = Color::srgb(0.5, 0.5, 0.5); // 无库存保持灰色
        }
    }

    info!("已清理所有路线段选择状态、预览和库存UI状态（包括边框）");
}

// 增强的预览连接验证函数
fn is_valid_placement_with_rotation(
    game_state: &GameState,
    position: GridPos,
    segment_type: &RouteSegmentType,
    rotation: u32,
) -> bool {
    // 基础验证
    if !is_valid_placement(game_state, position, segment_type) {
        return false;
    }

    // 可选：验证旋转后的连接是否合理
    // 这里可以添加更复杂的连接验证逻辑
    // 比如检查旋转后的路线段是否能与周围的路线段正确连接

    let connection_positions = segment_type.get_connection_positions(position, rotation);

    // 检查连接点是否与现有路线段匹配
    for conn_pos in connection_positions {
        if let Some(existing_segment) = game_state.placed_segments.get(&conn_pos) {
            // 检查现有路线段是否有朝向当前位置的连接口
            if !existing_segment.segment_type.has_connection_to(
                conn_pos,
                position,
                existing_segment.rotation,
            ) {
                // 有冲突的连接，但这可能是用户想要的，所以只是警告而不阻止
                trace!(
                    "警告：{:?} 在 {:?} (旋转{}°) 与 {:?} 在 {:?} 的连接可能不匹配",
                    segment_type,
                    position,
                    rotation,
                    existing_segment.segment_type,
                    conn_pos
                );
            }
        }
    }

    true
}

// 数字键快速旋转系统（可选的高级功能）
fn handle_quick_rotation_keys(
    mut input_state: ResMut<InputState>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    if input_state.selected_segment.is_some() {
        // 数字键1-4对应0°, 90°, 180°, 270°
        if keyboard_input.just_pressed(KeyCode::Digit1) {
            input_state.preview_rotation = 0;
            info!("快速旋转到 0°");
        } else if keyboard_input.just_pressed(KeyCode::Digit2) {
            input_state.preview_rotation = 90;
            info!("快速旋转到 90°");
        } else if keyboard_input.just_pressed(KeyCode::Digit3) {
            input_state.preview_rotation = 180;
            info!("快速旋转到 180°");
        } else if keyboard_input.just_pressed(KeyCode::Digit4) {
            input_state.preview_rotation = 270;
            info!("快速旋转到 270°");
        }
    }
}

// 改进的库存槽位处理，重置旋转当切换路线段类型时
fn handle_inventory_selection(
    mut input_state: ResMut<InputState>,
    button_query: Query<(&Interaction, &ButtonComponent), (Changed<Interaction>, With<Button>)>,
    game_state: Res<GameState>,
) {
    for (interaction, button_component) in button_query.iter() {
        if matches!(*interaction, Interaction::Pressed) {
            if let ButtonType::InventorySlot(segment_type) = &button_component.button_type {
                let available_count = game_state
                    .player_inventory
                    .get(segment_type)
                    .copied()
                    .unwrap_or(0);

                if available_count > 0 {
                    // 如果选择了不同的路线段类型，重置旋转
                    if input_state.selected_segment != Some(*segment_type) {
                        input_state.preview_rotation = 0;
                        info!("选择新路线段类型，重置旋转角度");
                    }

                    input_state.selected_segment = Some(*segment_type);
                    info!("选择路线段: {:?}", segment_type);
                } else {
                    warn!("库存不足: {:?}", segment_type);
                }
            }
        }
    }
}

// 最终推荐实现：直接集成的版本
fn show_rotation_hint_ui(
    mut commands: Commands,
    input_state: Res<InputState>,
    ui_assets: Res<crate::bus_puzzle::UIAssets>,
    current_language: Res<CurrentLanguage>,
    existing_hints: Query<Entity, With<RotationHintUI>>,
) {
    // 清除现有提示
    for entity in existing_hints.iter() {
        commands.entity(entity).despawn();
    }

    // 如果有选中的路线段，显示旋转提示
    if input_state.selected_segment.is_some() {
        commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    bottom: Px(20.0),
                    left: Px(50.0),
                    padding: UiRect::all(Px(12.0)),
                    border: UiRect::all(Px(2.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.85)),
                BorderColor(Color::srgb(0.4, 0.6, 0.8)),
                ZIndex(100),
                RotationHintUI,
                Name::new("Rotation Hint UI"),
            ))
            .with_children(|parent| {
                // 主要操作提示行
                let main_hint = match current_language.language {
                    Language::English => format!(
                        "🔄 Right/R/Space to Rotate (Current: {}°) | 📍 Left Click to Place | ❌ ESC to Cancel",
                        input_state.preview_rotation
                    ),
                    Language::Chinese => format!(
                        "🔄 右键/R键/空格旋转 (当前: {}°) | 📍 左键放置 | ❌ ESC取消",
                        input_state.preview_rotation
                    ),
                };

                parent.spawn((
                    Text::new(main_hint),
                    TextFont {
                        font: ui_assets.font.clone(),
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                    Node {
                        margin: UiRect::bottom(Px(4.0)),
                        ..default()
                    },
                ));

                // 快速旋转提示行
                let quick_hint = match current_language.language {
                    Language::English => "💡 Tip: Press 1-4 for quick rotation (0°/90°/180°/270°)",
                    Language::Chinese => "💡 提示: 按数字键1-4快速旋转 (0°/90°/180°/270°)",
                };

                parent.spawn((
                    Text::new(quick_hint),
                    TextFont {
                        font: ui_assets.font.clone(),
                        font_size: 11.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.8, 0.8, 1.0)),
                ));
            });
    }
}

// 实时更新提示文本的系统（处理语言切换）
fn update_rotation_hint_text(
    mut hint_texts: Query<&mut Text, With<RotationHintUI>>,
    input_state: Res<InputState>,
    current_language: Res<CurrentLanguage>,
) {
    // 如果语言发生变化，更新提示文本
    if current_language.is_changed() && input_state.selected_segment.is_some() {
        for mut text in hint_texts.iter_mut() {
            // 根据文本内容判断是主提示还是次提示
            if text.0.contains("🔄") {
                // 主要操作提示
                let new_text = match current_language.language {
                    Language::English => format!(
                        "🔄 Right/R/Space to Rotate (Current: {}°) | 📍 Left Click to Place | ❌ ESC to Cancel",
                        input_state.preview_rotation
                    ),
                    Language::Chinese => format!(
                        "🔄 右键/R键/空格旋转 (当前: {}°) | 📍 左键放置 | ❌ ESC取消",
                        input_state.preview_rotation
                    ),
                };
                *text = Text::new(new_text);
            } else if text.0.contains("💡") {
                // 快速旋转提示
                let new_text = match current_language.language {
                    Language::English => "💡 Tip: Press 1-4 for quick rotation (0°/90°/180°/270°)",
                    Language::Chinese => "💡 提示: 按数字键1-4快速旋转 (0°/90°/180°/270°)",
                };
                *text = Text::new(new_text);
            }
        }
    }
}

// 更新角度显示的系统
fn update_rotation_angle_display(
    mut hint_texts: Query<&mut Text, With<RotationHintUI>>,
    input_state: Res<InputState>,
    current_language: Res<CurrentLanguage>,
) {
    // 如果预览旋转角度发生变化，更新显示
    if input_state.is_changed() && input_state.selected_segment.is_some() {
        for mut text in hint_texts.iter_mut() {
            if text.0.contains("🔄") {
                // 更新主要操作提示中的角度显示
                let new_text = match current_language.language {
                    Language::English => format!(
                        "🔄 Right/R/Space to Rotate (Current: {}°) | 📍 Left Click to Place | ❌ ESC to Cancel",
                        input_state.preview_rotation
                    ),
                    Language::Chinese => format!(
                        "🔄 右键/R键/空格旋转 (当前: {}°) | 📍 左键放置 | ❌ ESC取消",
                        input_state.preview_rotation
                    ),
                };
                *text = Text::new(new_text);
            }
        }
    }
}
