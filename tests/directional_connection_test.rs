use last_stop::bus_puzzle::{GridPos, RouteSegmentType};

#[test]
fn test_tsplit_directional_connections() {
    // 测试 TSplit 不应该与不兼容的路线段连接
    
    let tsplit_pos = GridPos::new(8, 5);
    let tsplit = RouteSegmentType::TSplit;
    let tsplit_rotation = 0; // TSplit 0度：上、下、右连接
    
    let straight_pos = GridPos::new(7, 5); // TSplit 的左侧
    let straight = RouteSegmentType::Straight;
    let straight_rotation = 0; // Straight 0度：左、右连接
    
    // TSplit 0度时的连接位置
    let tsplit_connections = tsplit.get_connection_positions(tsplit_pos, tsplit_rotation);
    println!("TSplit 连接位置: {:?}", tsplit_connections);
    
    // 验证 TSplit 不连接到左方 (7,5)
    assert!(!tsplit_connections.contains(&straight_pos), 
            "TSplit 0度不应该连接到左方的直线段 (7,5)");
    
    // Straight 0度时的连接位置
    let straight_connections = straight.get_connection_positions(straight_pos, straight_rotation);
    println!("Straight 连接位置: {:?}", straight_connections);
    
    // 验证 Straight 连接到右方 (8,5)
    assert!(straight_connections.contains(&tsplit_pos), 
            "Straight 0度应该连接到右方的 TSplit (8,5)");
    
    // 关键测试：检查双向连接兼容性
    let tsplit_to_straight = tsplit.has_connection_to(tsplit_pos, straight_pos, tsplit_rotation);
    let straight_to_tsplit = straight.has_connection_to(straight_pos, tsplit_pos, straight_rotation);
    
    println!("TSplit -> Straight: {}", tsplit_to_straight);
    println!("Straight -> TSplit: {}", straight_to_tsplit);
    
    // TSplit 0度没有左连接，所以不应该连接到 Straight
    assert!(!tsplit_to_straight, "TSplit 0度不应该有到 Straight 的连接");
    
    // Straight 0度有右连接，可以连接到 TSplit
    assert!(straight_to_tsplit, "Straight 0度应该有到 TSplit 的连接");
    
    // 但是由于连接需要双向兼容，这两个路线段不应该实际连接
    assert!(!(tsplit_to_straight && straight_to_tsplit), 
            "由于 TSplit 没有左连接，这两个路线段不应该能够连接");
}

#[test]
fn test_valid_tsplit_connections() {
    // 测试 TSplit 与兼容路线段的正确连接
    
    let tsplit_pos = GridPos::new(5, 5);
    let tsplit = RouteSegmentType::TSplit;
    let tsplit_rotation = 0; // TSplit 0度：上、下、右连接
    
    // 测试上方连接
    let straight_above = RouteSegmentType::Straight;
    let straight_above_pos = GridPos::new(5, 4);
    let straight_above_rotation = 90; // 垂直方向：上、下连接
    
    assert!(tsplit.has_connection_to(tsplit_pos, straight_above_pos, tsplit_rotation), 
            "TSplit 应该连接到上方");
    assert!(straight_above.has_connection_to(straight_above_pos, tsplit_pos, straight_above_rotation), 
            "上方的垂直直线段应该连接到 TSplit");
    
    // 测试下方连接
    let straight_below_pos = GridPos::new(5, 6);
    assert!(tsplit.has_connection_to(tsplit_pos, straight_below_pos, tsplit_rotation), 
            "TSplit 应该连接到下方");
    assert!(straight_above.has_connection_to(straight_below_pos, tsplit_pos, straight_above_rotation), 
            "下方的垂直直线段应该连接到 TSplit");
    
    // 测试右方连接
    let straight_right = RouteSegmentType::Straight;
    let straight_right_pos = GridPos::new(6, 5);
    let straight_right_rotation = 0; // 水平方向：左、右连接
    
    assert!(tsplit.has_connection_to(tsplit_pos, straight_right_pos, tsplit_rotation), 
            "TSplit 应该连接到右方");
    assert!(straight_right.has_connection_to(straight_right_pos, tsplit_pos, straight_right_rotation), 
            "右方的水平直线段应该连接到 TSplit");
}

#[test]
fn test_curve_directional_connections() {
    // 测试 Curve 的方向性连接
    
    let curve_pos = GridPos::new(5, 5);
    let curve = RouteSegmentType::Curve;
    let curve_rotation = 90; // Curve 90度：上、右连接
    
    // 测试不应该连接的方向
    let straight_left_pos = GridPos::new(4, 5);
    let straight_below_pos = GridPos::new(5, 6);
    
    assert!(!curve.has_connection_to(curve_pos, straight_left_pos, curve_rotation), 
            "Curve 90度不应该连接到左方");
    assert!(!curve.has_connection_to(curve_pos, straight_below_pos, curve_rotation), 
            "Curve 90度不应该连接到下方");
    
    // 测试应该连接的方向
    let straight_above_pos = GridPos::new(5, 4);
    let straight_right_pos = GridPos::new(6, 5);
    
    assert!(curve.has_connection_to(curve_pos, straight_above_pos, curve_rotation), 
            "Curve 90度应该连接到上方");
    assert!(curve.has_connection_to(curve_pos, straight_right_pos, curve_rotation), 
            "Curve 90度应该连接到右方");
}

#[test]
fn test_cross_connections() {
    // 测试十字路口的全方向连接
    
    let cross_pos = GridPos::new(5, 5);
    let cross = RouteSegmentType::Cross;
    let cross_rotation = 0; // Cross 任何角度都是四方向连接
    
    // Cross 应该连接到所有四个方向
    assert!(cross.has_connection_to(cross_pos, GridPos::new(5, 4), cross_rotation), 
            "Cross 应该连接到上方");
    assert!(cross.has_connection_to(cross_pos, GridPos::new(5, 6), cross_rotation), 
            "Cross 应该连接到下方");
    assert!(cross.has_connection_to(cross_pos, GridPos::new(4, 5), cross_rotation), 
            "Cross 应该连接到左方");
    assert!(cross.has_connection_to(cross_pos, GridPos::new(6, 5), cross_rotation), 
            "Cross 应该连接到右方");
}
