use last_stop::bus_puzzle::{GridPos, RouteSegmentType};

#[test]
fn test_tsplit_connection_consistency() {
    let segment_type = RouteSegmentType::TSplit;
    let segment_pos = GridPos::new(5, 5);
    
    // TSplit 在 0 度时应该有上、下、右三个连接
    let connections_0deg = segment_type.get_connection_offsets(0);
    assert_eq!(connections_0deg.len(), 3, "TSplit 应该有 3 个连接");
    
    // 验证具体连接方向
    assert!(connections_0deg.contains(&(0, -1)), "TSplit 0度应该有上方连接");
    assert!(connections_0deg.contains(&(0, 1)), "TSplit 0度应该有下方连接");
    assert!(connections_0deg.contains(&(1, 0)), "TSplit 0度应该有右方连接");
    assert!(!connections_0deg.contains(&(-1, 0)), "TSplit 0度不应该有左方连接");
    
    // 测试 has_connection_to 方法
    assert!(segment_type.has_connection_to(segment_pos, GridPos::new(5, 4), 0), "应该能连接到上方");
    assert!(segment_type.has_connection_to(segment_pos, GridPos::new(5, 6), 0), "应该能连接到下方");
    assert!(segment_type.has_connection_to(segment_pos, GridPos::new(6, 5), 0), "应该能连接到右方");
    assert!(!segment_type.has_connection_to(segment_pos, GridPos::new(4, 5), 0), "不应该能连接到左方");
}

#[test]
fn test_curve_connection_consistency() {
    let segment_type = RouteSegmentType::Curve;
    let segment_pos = GridPos::new(5, 5);
    
    // Curve 在 0 度时应该有左、上两个连接
    let connections_0deg = segment_type.get_connection_offsets(0);
    assert_eq!(connections_0deg.len(), 2, "Curve 应该有 2 个连接");
    
    // 验证具体连接方向
    assert!(connections_0deg.contains(&(-1, 0)), "Curve 0度应该有左方连接");
    assert!(connections_0deg.contains(&(0, -1)), "Curve 0度应该有上方连接");
    assert!(!connections_0deg.contains(&(1, 0)), "Curve 0度不应该有右方连接");
    assert!(!connections_0deg.contains(&(0, 1)), "Curve 0度不应该有下方连接");
    
    // 测试 has_connection_to 方法
    assert!(segment_type.has_connection_to(segment_pos, GridPos::new(4, 5), 0), "应该能连接到左方");
    assert!(segment_type.has_connection_to(segment_pos, GridPos::new(5, 4), 0), "应该能连接到上方");
    assert!(!segment_type.has_connection_to(segment_pos, GridPos::new(6, 5), 0), "不应该能连接到右方");
    assert!(!segment_type.has_connection_to(segment_pos, GridPos::new(5, 6), 0), "不应该能连接到下方");
}

#[test]
fn test_rotation_consistency() {
    let segment_type = RouteSegmentType::TSplit;
    let segment_pos = GridPos::new(5, 5);
    
    // 测试旋转 90 度
    let connections_90deg = segment_type.get_connection_offsets(90);
    assert_eq!(connections_90deg.len(), 3, "旋转后仍应该有 3 个连接");
    
    // TSplit 旋转 90 度后：应该是左、右、下
    assert!(connections_90deg.contains(&(-1, 0)), "TSplit 90度应该有左方连接");
    assert!(connections_90deg.contains(&(1, 0)), "TSplit 90度应该有右方连接");
    assert!(connections_90deg.contains(&(0, 1)), "TSplit 90度应该有下方连接");
    assert!(!connections_90deg.contains(&(0, -1)), "TSplit 90度不应该有上方连接");
    
    // 验证 has_connection_to 方法在旋转后的正确性
    assert!(segment_type.has_connection_to(segment_pos, GridPos::new(4, 5), 90), "90度时应该能连接到左方");
    assert!(segment_type.has_connection_to(segment_pos, GridPos::new(6, 5), 90), "90度时应该能连接到右方");
    assert!(segment_type.has_connection_to(segment_pos, GridPos::new(5, 6), 90), "90度时应该能连接到下方");
    assert!(!segment_type.has_connection_to(segment_pos, GridPos::new(5, 4), 90), "90度时不应该能连接到上方");
}

#[test]
fn test_all_segment_types() {
    // 确保所有路线段类型都有正确的连接数量
    assert_eq!(RouteSegmentType::Straight.get_connection_offsets(0).len(), 2);
    assert_eq!(RouteSegmentType::Curve.get_connection_offsets(0).len(), 2);
    assert_eq!(RouteSegmentType::TSplit.get_connection_offsets(0).len(), 3);
    assert_eq!(RouteSegmentType::Cross.get_connection_offsets(0).len(), 4);
    assert_eq!(RouteSegmentType::Bridge.get_connection_offsets(0).len(), 2);
    assert_eq!(RouteSegmentType::Tunnel.get_connection_offsets(0).len(), 2);
}
