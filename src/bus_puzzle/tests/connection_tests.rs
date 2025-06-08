#[cfg(test)]
mod tests {
    use super::super::{GridPos, RouteSegmentType};

    #[test]
    fn test_tsplit_connections_consistency() {
        let segment_type = RouteSegmentType::TSplit;
        let segment_pos = GridPos::new(5, 5);

        // TSplit 在 0 度时应该有上、下、右三个连接
        let connections_0deg = segment_type.get_connection_offsets(0);
        assert_eq!(connections_0deg.len(), 3);
        assert!(connections_0deg.contains(&(0, -1))); // 上
        assert!(connections_0deg.contains(&(0, 1)));  // 下
        assert!(connections_0deg.contains(&(1, 0)));  // 右

        // 验证连接位置
        let connection_positions = segment_type.get_connection_positions(segment_pos, 0);
        assert!(connection_positions.contains(&GridPos::new(5, 4))); // 上方
        assert!(connection_positions.contains(&GridPos::new(5, 6))); // 下方
        assert!(connection_positions.contains(&GridPos::new(6, 5))); // 右方

        // 验证 has_connection_to 方法
        assert!(segment_type.has_connection_to(segment_pos, GridPos::new(5, 4), 0)); // 上
        assert!(segment_type.has_connection_to(segment_pos, GridPos::new(5, 6), 0)); // 下
        assert!(segment_type.has_connection_to(segment_pos, GridPos::new(6, 5), 0)); // 右
        assert!(!segment_type.has_connection_to(segment_pos, GridPos::new(4, 5), 0)); // 左 - 不应该有连接
    }

    #[test]
    fn test_tsplit_rotations() {
        let segment_type = RouteSegmentType::TSplit;
        let segment_pos = GridPos::new(5, 5);

        // TSplit 旋转 90 度后：左、右、下
        let connections_90deg = segment_type.get_connection_offsets(90);
        assert_eq!(connections_90deg.len(), 3);
        assert!(connections_90deg.contains(&(-1, 0))); // 左
        assert!(connections_90deg.contains(&(1, 0)));  // 右
        assert!(connections_90deg.contains(&(0, 1)));  // 下

        // TSplit 旋转 180 度后：下、上、左
        let connections_180deg = segment_type.get_connection_offsets(180);
        assert_eq!(connections_180deg.len(), 3);
        assert!(connections_180deg.contains(&(0, 1)));  // 下
        assert!(connections_180deg.contains(&(0, -1))); // 上
        assert!(connections_180deg.contains(&(-1, 0))); // 左

        // TSplit 旋转 270 度后：右、左、上
        let connections_270deg = segment_type.get_connection_offsets(270);
        assert_eq!(connections_270deg.len(), 3);
        assert!(connections_270deg.contains(&(1, 0)));  // 右
        assert!(connections_270deg.contains(&(-1, 0))); // 左
        assert!(connections_270deg.contains(&(0, -1))); // 上
    }

    #[test]
    fn test_all_segment_types_consistency() {
        let _segment_pos = GridPos::new(5, 5);

        // 测试所有路线段类型在 0 度时的连接数量
        let straight = RouteSegmentType::Straight.get_connection_offsets(0);
        assert_eq!(straight.len(), 2); // 直线：2个连接

        let curve = RouteSegmentType::Curve.get_connection_offsets(0);
        assert_eq!(curve.len(), 2); // 转弯：2个连接

        let tsplit = RouteSegmentType::TSplit.get_connection_offsets(0);
        assert_eq!(tsplit.len(), 3); // T型：3个连接

        let cross = RouteSegmentType::Cross.get_connection_offsets(0);
        assert_eq!(cross.len(), 4); // 十字：4个连接

        let bridge = RouteSegmentType::Bridge.get_connection_offsets(0);
        assert_eq!(bridge.len(), 2); // 桥梁：2个连接

        let tunnel = RouteSegmentType::Tunnel.get_connection_offsets(0);
        assert_eq!(tunnel.len(), 2); // 隧道：2个连接
    }

    #[test]
    fn test_curve_connections() {
        let segment_type = RouteSegmentType::Curve;
        let segment_pos = GridPos::new(5, 5);

        // Curve 在 0 度时应该有左、上两个连接
        let connections_0deg = segment_type.get_connection_offsets(0);
        assert_eq!(connections_0deg.len(), 2);
        assert!(connections_0deg.contains(&(-1, 0))); // 左
        assert!(connections_0deg.contains(&(0, -1))); // 上

        // 验证连接位置
        assert!(segment_type.has_connection_to(segment_pos, GridPos::new(4, 5), 0)); // 左
        assert!(segment_type.has_connection_to(segment_pos, GridPos::new(5, 4), 0)); // 上
        assert!(!segment_type.has_connection_to(segment_pos, GridPos::new(6, 5), 0)); // 右 - 不应该有连接
        assert!(!segment_type.has_connection_to(segment_pos, GridPos::new(5, 6), 0)); // 下 - 不应该有连接
    }
}
