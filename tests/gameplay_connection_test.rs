use last_stop::bus_puzzle::{GridPos, RouteSegmentType};

/// 测试教学关卡中的实际连接情况
#[test]
fn test_tutorial_level_connections() {
    // 模拟教学关卡的路线段配置
    
    // A站 (1,4) 附近的 Curve 在 (1,5)，旋转90度
    let curve_segment = RouteSegmentType::Curve;
    let curve_pos = GridPos::new(1, 5);
    let curve_rotation = 90;
    
    // Curve 旋转90度后的连接位置
    let curve_connections = curve_segment.get_connection_positions(curve_pos, curve_rotation);
    
    // 验证 Curve 是否连接到 A站 (1,4)
    assert!(curve_connections.contains(&GridPos::new(1, 4)), 
            "Curve 旋转90度后应该连接到 A站 (1,4)");
    
    // 验证 Curve 是否向右连接到 (2,5)
    assert!(curve_connections.contains(&GridPos::new(2, 5)), 
            "Curve 旋转90度后应该向右连接到 (2,5)");
    
    // 验证连接数量正确
    assert_eq!(curve_connections.len(), 2, "Curve 应该有 2 个连接");
    
    // B站 (8,4) 附近的 TSplit 在 (8,5)，旋转0度
    let tsplit_segment = RouteSegmentType::TSplit;
    let tsplit_pos = GridPos::new(8, 5);
    let tsplit_rotation = 0;
    
    // TSplit 0度时的连接位置
    let tsplit_connections = tsplit_segment.get_connection_positions(tsplit_pos, tsplit_rotation);
    
    // 验证 TSplit 是否连接到 B站 (8,4) - 上方连接
    assert!(tsplit_connections.contains(&GridPos::new(8, 4)), 
            "TSplit 0度应该连接到 B站 (8,4)");
    
    // 验证 TSplit 的其他连接
    assert!(tsplit_connections.contains(&GridPos::new(8, 6)), 
            "TSplit 0度应该有下方连接 (8,6)");
    assert!(tsplit_connections.contains(&GridPos::new(9, 5)), 
            "TSplit 0度应该有右方连接 (9,5)");
    
    // 验证 TSplit 不应该有左方连接
    assert!(!tsplit_connections.contains(&GridPos::new(7, 5)), 
            "TSplit 0度不应该有左方连接 (7,5)");
    
    // 验证连接数量正确
    assert_eq!(tsplit_connections.len(), 3, "TSplit 应该有 3 个连接");
}

/// 测试路线段之间的相互连接
#[test]
fn test_segment_to_segment_connections() {
    // 测试水平直线段连接
    let straight1_pos = GridPos::new(2, 5);
    let straight2_pos = GridPos::new(3, 5);
    
    let straight_segment = RouteSegmentType::Straight;
    
    // 第一个直线段（水平，0度）的连接
    let straight1_connections = straight_segment.get_connection_positions(straight1_pos, 0);
    
    // 验证第一个直线段连接到第二个直线段
    assert!(straight1_connections.contains(&straight2_pos), 
            "水平直线段应该连接到右侧的直线段");
    
    // 第二个直线段的连接
    let straight2_connections = straight_segment.get_connection_positions(straight2_pos, 0);
    
    // 验证第二个直线段连接到第一个直线段
    assert!(straight2_connections.contains(&straight1_pos), 
            "水平直线段应该连接到左侧的直线段");
}

/// 测试旋转后的连接
#[test]
fn test_rotated_connections() {
    let segment_pos = GridPos::new(5, 5);
    
    // 测试 Curve 的不同旋转角度
    let curve = RouteSegmentType::Curve;
    
    // 0度：左(-1,0)和上(0,-1)
    let connections_0 = curve.get_connection_offsets(0);
    assert!(connections_0.contains(&(-1, 0)));
    assert!(connections_0.contains(&(0, -1)));
    
    // 90度：上(0,-1)和右(1,0)
    let connections_90 = curve.get_connection_offsets(90);
    assert!(connections_90.contains(&(0, -1)));
    assert!(connections_90.contains(&(1, 0)));
    
    // 180度：右(1,0)和下(0,1)
    let connections_180 = curve.get_connection_offsets(180);
    assert!(connections_180.contains(&(1, 0)));
    assert!(connections_180.contains(&(0, 1)));
    
    // 270度：下(0,1)和左(-1,0)
    let connections_270 = curve.get_connection_offsets(270);
    assert!(connections_270.contains(&(0, 1)));
    assert!(connections_270.contains(&(-1, 0)));
}

/// 测试 has_connection_to 方法的正确性
#[test]
fn test_has_connection_to_method() {
    let segment_pos = GridPos::new(5, 5);
    let tsplit = RouteSegmentType::TSplit;
    
    // TSplit 0度时的连接测试
    assert!(tsplit.has_connection_to(segment_pos, GridPos::new(5, 4), 0), 
            "TSplit 0度应该连接到上方");
    assert!(tsplit.has_connection_to(segment_pos, GridPos::new(5, 6), 0), 
            "TSplit 0度应该连接到下方");
    assert!(tsplit.has_connection_to(segment_pos, GridPos::new(6, 5), 0), 
            "TSplit 0度应该连接到右方");
    assert!(!tsplit.has_connection_to(segment_pos, GridPos::new(4, 5), 0), 
            "TSplit 0度不应该连接到左方");
    
    // 测试距离大于1的情况
    assert!(!tsplit.has_connection_to(segment_pos, GridPos::new(3, 5), 0), 
            "TSplit 不应该连接到距离大于1的位置");
    assert!(!tsplit.has_connection_to(segment_pos, GridPos::new(5, 7), 0), 
            "TSplit 不应该连接到距离大于1的位置");
}
