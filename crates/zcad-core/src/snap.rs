//! 对象捕捉系统
//!
//! 参考 LibreCAD 的设计，实现 CAD 标准的对象捕捉功能。
//!
//! 支持的捕捉类型：
//! - 端点 (Endpoint)
//! - 中点 (Midpoint)
//! - 圆心 (Center)
//! - 交点 (Intersection)
//! - 垂足 (Perpendicular)
//! - 切点 (Tangent)
//! - 最近点 (Nearest)
//! - 网格点 (Grid)

use crate::entity::{Entity, EntityId};
use crate::geometry::{Arc, Circle, Geometry, Line, Polyline};
use crate::math::{Point2, Vector2, EPSILON};
use serde::{Deserialize, Serialize};

/// 捕捉类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SnapType {
    /// 端点捕捉
    Endpoint,
    /// 中点捕捉
    Midpoint,
    /// 圆心捕捉
    Center,
    /// 交点捕捉
    Intersection,
    /// 垂足捕捉
    Perpendicular,
    /// 切点捕捉
    Tangent,
    /// 最近点捕捉
    Nearest,
    /// 网格点捕捉
    Grid,
    /// 象限点（圆/弧的0°, 90°, 180°, 270°位置）
    Quadrant,
}

impl SnapType {
    /// 获取捕捉类型的名称
    pub fn name(&self) -> &'static str {
        match self {
            SnapType::Endpoint => "端点",
            SnapType::Midpoint => "中点",
            SnapType::Center => "圆心",
            SnapType::Intersection => "交点",
            SnapType::Perpendicular => "垂足",
            SnapType::Tangent => "切点",
            SnapType::Nearest => "最近点",
            SnapType::Grid => "网格点",
            SnapType::Quadrant => "象限点",
        }
    }

    /// 获取捕捉类型的快捷键
    pub fn shortcut(&self) -> &'static str {
        match self {
            SnapType::Endpoint => "END",
            SnapType::Midpoint => "MID",
            SnapType::Center => "CEN",
            SnapType::Intersection => "INT",
            SnapType::Perpendicular => "PER",
            SnapType::Tangent => "TAN",
            SnapType::Nearest => "NEA",
            SnapType::Grid => "GRI",
            SnapType::Quadrant => "QUA",
        }
    }
}

/// 捕捉点
#[derive(Debug, Clone)]
pub struct SnapPoint {
    /// 捕捉到的世界坐标
    pub point: Point2,
    /// 捕捉类型
    pub snap_type: SnapType,
    /// 关联的实体ID（如果有）
    pub entity_id: Option<EntityId>,
    /// 距离鼠标的屏幕距离（用于排序）
    pub distance: f64,
}

impl SnapPoint {
    pub fn new(point: Point2, snap_type: SnapType, entity_id: Option<EntityId>, distance: f64) -> Self {
        Self {
            point,
            snap_type,
            entity_id,
            distance,
        }
    }
}

/// 捕捉配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapConfig {
    /// 捕捉容差（屏幕像素）
    pub tolerance: f64,
    /// 启用的捕捉类型
    pub enabled_types: SnapMask,
    /// 网格间距
    pub grid_spacing: f64,
    /// 是否显示捕捉标记
    pub show_markers: bool,
    /// 是否显示捕捉提示
    pub show_tooltips: bool,
}

impl Default for SnapConfig {
    fn default() -> Self {
        Self {
            tolerance: 10.0, // 10像素
            enabled_types: SnapMask::default(),
            grid_spacing: 10.0,
            show_markers: true,
            show_tooltips: true,
        }
    }
}

/// 捕捉掩码（位域，用于快速启用/禁用捕捉类型）
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SnapMask {
    bits: u16,
}

impl SnapMask {
    pub const ENDPOINT: u16 = 1 << 0;
    pub const MIDPOINT: u16 = 1 << 1;
    pub const CENTER: u16 = 1 << 2;
    pub const INTERSECTION: u16 = 1 << 3;
    pub const PERPENDICULAR: u16 = 1 << 4;
    pub const TANGENT: u16 = 1 << 5;
    pub const NEAREST: u16 = 1 << 6;
    pub const GRID: u16 = 1 << 7;
    pub const QUADRANT: u16 = 1 << 8;

    pub const NONE: SnapMask = SnapMask { bits: 0 };
    pub const ALL: SnapMask = SnapMask { bits: 0xFFFF };

    pub fn new(bits: u16) -> Self {
        Self { bits }
    }

    pub fn is_enabled(&self, snap_type: SnapType) -> bool {
        let bit = match snap_type {
            SnapType::Endpoint => Self::ENDPOINT,
            SnapType::Midpoint => Self::MIDPOINT,
            SnapType::Center => Self::CENTER,
            SnapType::Intersection => Self::INTERSECTION,
            SnapType::Perpendicular => Self::PERPENDICULAR,
            SnapType::Tangent => Self::TANGENT,
            SnapType::Nearest => Self::NEAREST,
            SnapType::Grid => Self::GRID,
            SnapType::Quadrant => Self::QUADRANT,
        };
        self.bits & bit != 0
    }

    pub fn set(&mut self, snap_type: SnapType, enabled: bool) {
        let bit = match snap_type {
            SnapType::Endpoint => Self::ENDPOINT,
            SnapType::Midpoint => Self::MIDPOINT,
            SnapType::Center => Self::CENTER,
            SnapType::Intersection => Self::INTERSECTION,
            SnapType::Perpendicular => Self::PERPENDICULAR,
            SnapType::Tangent => Self::TANGENT,
            SnapType::Nearest => Self::NEAREST,
            SnapType::Grid => Self::GRID,
            SnapType::Quadrant => Self::QUADRANT,
        };
        if enabled {
            self.bits |= bit;
        } else {
            self.bits &= !bit;
        }
    }

    pub fn toggle(&mut self, snap_type: SnapType) {
        let enabled = self.is_enabled(snap_type);
        self.set(snap_type, !enabled);
    }
}

impl Default for SnapMask {
    fn default() -> Self {
        // 默认启用常用的捕捉类型
        Self {
            bits: Self::ENDPOINT | Self::MIDPOINT | Self::CENTER | Self::INTERSECTION,
        }
    }
}

/// 捕捉引擎
///
/// 负责计算和管理对象捕捉
#[derive(Debug, Clone)]
pub struct SnapEngine {
    config: SnapConfig,
    /// 缓存的候选捕捉点
    candidates: Vec<SnapPoint>,
}

impl SnapEngine {
    pub fn new(config: SnapConfig) -> Self {
        Self {
            config,
            candidates: Vec::with_capacity(64),
        }
    }

    /// 获取配置
    pub fn config(&self) -> &SnapConfig {
        &self.config
    }

    /// 获取配置（可变）
    pub fn config_mut(&mut self) -> &mut SnapConfig {
        &mut self.config
    }

    /// 寻找最佳捕捉点
    ///
    /// # 参数
    /// - `mouse_world`: 鼠标的世界坐标
    /// - `entities`: 要搜索的实体列表
    /// - `zoom`: 当前缩放级别（用于计算屏幕距离）
    /// - `reference_point`: 参考点（用于垂足、切点等计算）
    pub fn find_snap_point(
        &mut self,
        mouse_world: Point2,
        entities: &[&Entity],
        zoom: f64,
        reference_point: Option<Point2>,
    ) -> Option<SnapPoint> {
        self.candidates.clear();

        // 世界坐标容差
        let world_tolerance = self.config.tolerance / zoom;

        // 1. 网格捕捉
        if self.config.enabled_types.is_enabled(SnapType::Grid) {
            if let Some(snap) = self.snap_to_grid(mouse_world, world_tolerance) {
                self.candidates.push(snap);
            }
        }

        // 2. 收集所有实体的捕捉点
        for entity in entities {
            self.collect_entity_snap_points(
                entity,
                mouse_world,
                world_tolerance,
                reference_point,
            );
        }

        // 3. 交点捕捉（需要成对的实体）
        if self.config.enabled_types.is_enabled(SnapType::Intersection) {
            self.collect_intersection_points(entities, mouse_world, world_tolerance);
        }

        // 4. 找到最近的捕捉点
        self.candidates
            .iter()
            .filter(|p| p.distance <= world_tolerance)
            .min_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap_or(std::cmp::Ordering::Equal))
            .cloned()
    }

    /// 收集单个实体的捕捉点
    fn collect_entity_snap_points(
        &mut self,
        entity: &Entity,
        mouse: Point2,
        tolerance: f64,
        reference_point: Option<Point2>,
    ) {
        match &entity.geometry {
            Geometry::Point(p) => {
                if self.config.enabled_types.is_enabled(SnapType::Endpoint) {
                    let dist = (p.position - mouse).norm();
                    if dist <= tolerance {
                        self.candidates.push(SnapPoint::new(
                            p.position,
                            SnapType::Endpoint,
                            Some(entity.id),
                            dist,
                        ));
                    }
                }
            }
            Geometry::Line(line) => {
                self.collect_line_snap_points(line, entity.id, mouse, tolerance, reference_point);
            }
            Geometry::Circle(circle) => {
                self.collect_circle_snap_points(circle, entity.id, mouse, tolerance, reference_point);
            }
            Geometry::Arc(arc) => {
                self.collect_arc_snap_points(arc, entity.id, mouse, tolerance, reference_point);
            }
            Geometry::Polyline(polyline) => {
                self.collect_polyline_snap_points(polyline, entity.id, mouse, tolerance, reference_point);
            }
            Geometry::Text(text) => {
                // 文本只捕捉插入点
                if self.config.enabled_types.is_enabled(SnapType::Endpoint) {
                    let dist = (text.position - mouse).norm();
                    if dist <= tolerance {
                        self.candidates.push(SnapPoint::new(
                            text.position,
                            SnapType::Endpoint,
                            Some(entity.id),
                            dist,
                        ));
                    }
                }
            }
            Geometry::Dimension(dim) => {
                // 标注捕捉定义点
                if self.config.enabled_types.is_enabled(SnapType::Endpoint) {
                    for &pt in &[dim.definition_point1, dim.definition_point2] {
                        let dist = (pt - mouse).norm();
                        if dist <= tolerance {
                            self.candidates.push(SnapPoint::new(
                                pt,
                                SnapType::Endpoint,
                                Some(entity.id),
                                dist,
                            ));
                        }
                    }
                }
            }
        }
    }

    /// 线段的捕捉点
    fn collect_line_snap_points(
        &mut self,
        line: &Line,
        entity_id: EntityId,
        mouse: Point2,
        tolerance: f64,
        reference_point: Option<Point2>,
    ) {
        let enabled = &self.config.enabled_types;

        // 端点
        if enabled.is_enabled(SnapType::Endpoint) {
            let dist_start = (line.start - mouse).norm();
            if dist_start <= tolerance {
                self.candidates.push(SnapPoint::new(
                    line.start,
                    SnapType::Endpoint,
                    Some(entity_id),
                    dist_start,
                ));
            }

            let dist_end = (line.end - mouse).norm();
            if dist_end <= tolerance {
                self.candidates.push(SnapPoint::new(
                    line.end,
                    SnapType::Endpoint,
                    Some(entity_id),
                    dist_end,
                ));
            }
        }

        // 中点
        if enabled.is_enabled(SnapType::Midpoint) {
            let midpoint = line.midpoint();
            let dist = (midpoint - mouse).norm();
            if dist <= tolerance {
                self.candidates.push(SnapPoint::new(
                    midpoint,
                    SnapType::Midpoint,
                    Some(entity_id),
                    dist,
                ));
            }
        }

        // 垂足
        if enabled.is_enabled(SnapType::Perpendicular) {
            if let Some(ref_point) = reference_point {
                if let Some(perp) = self.perpendicular_to_line(line, ref_point) {
                    let dist = (perp - mouse).norm();
                    if dist <= tolerance {
                        self.candidates.push(SnapPoint::new(
                            perp,
                            SnapType::Perpendicular,
                            Some(entity_id),
                            dist,
                        ));
                    }
                }
            }
        }

        // 最近点
        if enabled.is_enabled(SnapType::Nearest) {
            let nearest = self.nearest_point_on_line(line, mouse);
            let dist = (nearest - mouse).norm();
            if dist <= tolerance {
                self.candidates.push(SnapPoint::new(
                    nearest,
                    SnapType::Nearest,
                    Some(entity_id),
                    dist,
                ));
            }
        }
    }

    /// 圆的捕捉点
    fn collect_circle_snap_points(
        &mut self,
        circle: &Circle,
        entity_id: EntityId,
        mouse: Point2,
        tolerance: f64,
        reference_point: Option<Point2>,
    ) {
        let enabled = &self.config.enabled_types;

        // 圆心
        if enabled.is_enabled(SnapType::Center) {
            let dist = (circle.center - mouse).norm();
            if dist <= tolerance {
                self.candidates.push(SnapPoint::new(
                    circle.center,
                    SnapType::Center,
                    Some(entity_id),
                    dist,
                ));
            }
        }

        // 象限点
        if enabled.is_enabled(SnapType::Quadrant) {
            let quadrant_angles = [0.0, std::f64::consts::FRAC_PI_2, std::f64::consts::PI, 3.0 * std::f64::consts::FRAC_PI_2];
            for angle in quadrant_angles {
                let point = circle.point_at_angle(angle);
                let dist = (point - mouse).norm();
                if dist <= tolerance {
                    self.candidates.push(SnapPoint::new(
                        point,
                        SnapType::Quadrant,
                        Some(entity_id),
                        dist,
                    ));
                }
            }
        }

        // 切点
        if enabled.is_enabled(SnapType::Tangent) {
            if let Some(ref_point) = reference_point {
                for tangent in self.tangent_points_to_circle(circle, ref_point) {
                    let dist = (tangent - mouse).norm();
                    if dist <= tolerance {
                        self.candidates.push(SnapPoint::new(
                            tangent,
                            SnapType::Tangent,
                            Some(entity_id),
                            dist,
                        ));
                    }
                }
            }
        }

        // 最近点（圆上）
        if enabled.is_enabled(SnapType::Nearest) {
            let dir = (mouse - circle.center).normalize();
            let nearest = circle.center + dir * circle.radius;
            let dist = (nearest - mouse).norm();
            if dist <= tolerance {
                self.candidates.push(SnapPoint::new(
                    nearest,
                    SnapType::Nearest,
                    Some(entity_id),
                    dist,
                ));
            }
        }
    }

    /// 圆弧的捕捉点
    fn collect_arc_snap_points(
        &mut self,
        arc: &Arc,
        entity_id: EntityId,
        mouse: Point2,
        tolerance: f64,
        _reference_point: Option<Point2>,
    ) {
        let enabled = &self.config.enabled_types;

        // 端点
        if enabled.is_enabled(SnapType::Endpoint) {
            let start = arc.start_point();
            let dist_start = (start - mouse).norm();
            if dist_start <= tolerance {
                self.candidates.push(SnapPoint::new(
                    start,
                    SnapType::Endpoint,
                    Some(entity_id),
                    dist_start,
                ));
            }

            let end = arc.end_point();
            let dist_end = (end - mouse).norm();
            if dist_end <= tolerance {
                self.candidates.push(SnapPoint::new(
                    end,
                    SnapType::Endpoint,
                    Some(entity_id),
                    dist_end,
                ));
            }
        }

        // 圆心
        if enabled.is_enabled(SnapType::Center) {
            let dist = (arc.center - mouse).norm();
            if dist <= tolerance {
                self.candidates.push(SnapPoint::new(
                    arc.center,
                    SnapType::Center,
                    Some(entity_id),
                    dist,
                ));
            }
        }

        // 中点（弧的中点）
        if enabled.is_enabled(SnapType::Midpoint) {
            let mid_angle = arc.start_angle + arc.sweep_angle() / 2.0;
            let midpoint = Point2::new(
                arc.center.x + arc.radius * mid_angle.cos(),
                arc.center.y + arc.radius * mid_angle.sin(),
            );
            let dist = (midpoint - mouse).norm();
            if dist <= tolerance {
                self.candidates.push(SnapPoint::new(
                    midpoint,
                    SnapType::Midpoint,
                    Some(entity_id),
                    dist,
                ));
            }
        }
    }

    /// 多段线的捕捉点
    fn collect_polyline_snap_points(
        &mut self,
        polyline: &Polyline,
        entity_id: EntityId,
        mouse: Point2,
        tolerance: f64,
        reference_point: Option<Point2>,
    ) {
        let enabled = &self.config.enabled_types;

        // 顶点（端点）
        if enabled.is_enabled(SnapType::Endpoint) {
            for vertex in &polyline.vertices {
                let dist = (vertex.point - mouse).norm();
                if dist <= tolerance {
                    self.candidates.push(SnapPoint::new(
                        vertex.point,
                        SnapType::Endpoint,
                        Some(entity_id),
                        dist,
                    ));
                }
            }
        }

        // 线段中点
        if enabled.is_enabled(SnapType::Midpoint) {
            for i in 0..polyline.segment_count() {
                let v1 = &polyline.vertices[i];
                let v2 = &polyline.vertices[(i + 1) % polyline.vertices.len()];

                // 只处理直线段的中点
                if v1.bulge.abs() < EPSILON {
                    let midpoint = Point2::new(
                        (v1.point.x + v2.point.x) / 2.0,
                        (v1.point.y + v2.point.y) / 2.0,
                    );
                    let dist = (midpoint - mouse).norm();
                    if dist <= tolerance {
                        self.candidates.push(SnapPoint::new(
                            midpoint,
                            SnapType::Midpoint,
                            Some(entity_id),
                            dist,
                        ));
                    }
                }
            }
        }

        // 最近点和垂足需要遍历所有线段
        if enabled.is_enabled(SnapType::Nearest) || enabled.is_enabled(SnapType::Perpendicular) {
            for i in 0..polyline.segment_count() {
                let v1 = &polyline.vertices[i];
                let v2 = &polyline.vertices[(i + 1) % polyline.vertices.len()];

                // 只处理直线段
                if v1.bulge.abs() < EPSILON {
                    let line = Line::new(v1.point, v2.point);

                    if enabled.is_enabled(SnapType::Nearest) {
                        let nearest = self.nearest_point_on_line(&line, mouse);
                        let dist = (nearest - mouse).norm();
                        if dist <= tolerance {
                            self.candidates.push(SnapPoint::new(
                                nearest,
                                SnapType::Nearest,
                                Some(entity_id),
                                dist,
                            ));
                        }
                    }

                    if enabled.is_enabled(SnapType::Perpendicular) {
                        if let Some(ref_point) = reference_point {
                            if let Some(perp) = self.perpendicular_to_line(&line, ref_point) {
                                let dist = (perp - mouse).norm();
                                if dist <= tolerance {
                                    self.candidates.push(SnapPoint::new(
                                        perp,
                                        SnapType::Perpendicular,
                                        Some(entity_id),
                                        dist,
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// 收集交点
    fn collect_intersection_points(
        &mut self,
        entities: &[&Entity],
        mouse: Point2,
        tolerance: f64,
    ) {
        // 双重循环检查所有实体对
        for i in 0..entities.len() {
            for j in (i + 1)..entities.len() {
                let intersections = self.find_intersections(&entities[i].geometry, &entities[j].geometry);
                
                for point in intersections {
                    let dist = (point - mouse).norm();
                    if dist <= tolerance {
                        self.candidates.push(SnapPoint::new(
                            point,
                            SnapType::Intersection,
                            None, // 交点涉及两个实体
                            dist,
                        ));
                    }
                }
            }
        }
    }

    /// 网格捕捉
    fn snap_to_grid(&self, mouse: Point2, tolerance: f64) -> Option<SnapPoint> {
        let spacing = self.config.grid_spacing;
        
        let grid_x = (mouse.x / spacing).round() * spacing;
        let grid_y = (mouse.y / spacing).round() * spacing;
        let grid_point = Point2::new(grid_x, grid_y);
        
        let dist = (grid_point - mouse).norm();
        if dist <= tolerance {
            Some(SnapPoint::new(grid_point, SnapType::Grid, None, dist))
        } else {
            None
        }
    }

    // ========== 几何计算辅助方法 ==========

    /// 计算点到线段的最近点
    fn nearest_point_on_line(&self, line: &Line, point: Point2) -> Point2 {
        let v = line.end - line.start;
        let w = point - line.start;

        let c1 = w.dot(&v);
        if c1 <= 0.0 {
            return line.start;
        }

        let c2 = v.dot(&v);
        if c2 <= c1 {
            return line.end;
        }

        let b = c1 / c2;
        line.start + v * b
    }

    /// 计算从参考点到线段的垂足
    fn perpendicular_to_line(&self, line: &Line, ref_point: Point2) -> Option<Point2> {
        let v = line.end - line.start;
        let w = ref_point - line.start;

        let c1 = w.dot(&v);
        let c2 = v.dot(&v);

        if c2 < EPSILON {
            return None;
        }

        let b = c1 / c2;
        
        // 垂足必须在线段上
        if b >= 0.0 && b <= 1.0 {
            Some(line.start + v * b)
        } else {
            None
        }
    }

    /// 计算从点到圆的切点
    fn tangent_points_to_circle(&self, circle: &Circle, point: Point2) -> Vec<Point2> {
        let d = (point - circle.center).norm();
        
        // 点在圆内，没有切点
        if d <= circle.radius {
            return vec![];
        }

        // 切线长度
        let _tangent_len = (d * d - circle.radius * circle.radius).sqrt();
        
        // 从圆心到点的方向
        let dir = (point - circle.center) / d;
        
        // 切点角度
        let angle = (circle.radius / d).asin();
        
        // 计算两个切点
        let base_angle = dir.y.atan2(dir.x);
        
        vec![
            Point2::new(
                circle.center.x + circle.radius * (base_angle + angle).cos(),
                circle.center.y + circle.radius * (base_angle + angle).sin(),
            ),
            Point2::new(
                circle.center.x + circle.radius * (base_angle - angle).cos(),
                circle.center.y + circle.radius * (base_angle - angle).sin(),
            ),
        ]
    }

    /// 计算两个几何体的交点
    fn find_intersections(&self, geom1: &Geometry, geom2: &Geometry) -> Vec<Point2> {
        match (geom1, geom2) {
            (Geometry::Line(l1), Geometry::Line(l2)) => {
                self.line_line_intersection(l1, l2).into_iter().collect()
            }
            (Geometry::Line(line), Geometry::Circle(circle)) 
            | (Geometry::Circle(circle), Geometry::Line(line)) => {
                self.line_circle_intersection(line, circle)
            }
            (Geometry::Circle(c1), Geometry::Circle(c2)) => {
                self.circle_circle_intersection(c1, c2)
            }
            (Geometry::Line(line), Geometry::Arc(arc))
            | (Geometry::Arc(arc), Geometry::Line(line)) => {
                self.line_arc_intersection(line, arc)
            }
            // 多段线：展开为线段/弧后计算
            (Geometry::Line(line), Geometry::Polyline(poly))
            | (Geometry::Polyline(poly), Geometry::Line(line)) => {
                self.line_polyline_intersection(line, poly)
            }
            // 其他情况暂不处理
            _ => vec![],
        }
    }

    /// 线段-线段交点
    fn line_line_intersection(&self, l1: &Line, l2: &Line) -> Option<Point2> {
        let d1 = l1.end - l1.start;
        let d2 = l2.end - l2.start;

        let cross = d1.x * d2.y - d1.y * d2.x;
        
        // 平行
        if cross.abs() < EPSILON {
            return None;
        }

        let d = l2.start - l1.start;
        let t1 = (d.x * d2.y - d.y * d2.x) / cross;
        let t2 = (d.x * d1.y - d.y * d1.x) / cross;

        // 检查交点是否在两条线段上
        if t1 >= 0.0 && t1 <= 1.0 && t2 >= 0.0 && t2 <= 1.0 {
            Some(l1.start + d1 * t1)
        } else {
            None
        }
    }

    /// 线段-圆交点
    fn line_circle_intersection(&self, line: &Line, circle: &Circle) -> Vec<Point2> {
        let d = line.end - line.start;
        let f = line.start - circle.center;

        let a = d.dot(&d);
        let b = 2.0 * f.dot(&d);
        let c = f.dot(&f) - circle.radius * circle.radius;

        let discriminant = b * b - 4.0 * a * c;

        if discriminant < 0.0 {
            return vec![];
        }

        let mut intersections = Vec::new();

        if discriminant.abs() < EPSILON {
            // 一个交点（相切）
            let t = -b / (2.0 * a);
            if t >= 0.0 && t <= 1.0 {
                intersections.push(line.start + d * t);
            }
        } else {
            // 两个交点
            let sqrt_disc = discriminant.sqrt();
            let t1 = (-b - sqrt_disc) / (2.0 * a);
            let t2 = (-b + sqrt_disc) / (2.0 * a);

            if t1 >= 0.0 && t1 <= 1.0 {
                intersections.push(line.start + d * t1);
            }
            if t2 >= 0.0 && t2 <= 1.0 {
                intersections.push(line.start + d * t2);
            }
        }

        intersections
    }

    /// 圆-圆交点
    fn circle_circle_intersection(&self, c1: &Circle, c2: &Circle) -> Vec<Point2> {
        let d = (c2.center - c1.center).norm();

        // 不相交情况
        if d > c1.radius + c2.radius || d < (c1.radius - c2.radius).abs() || d < EPSILON {
            return vec![];
        }

        let a = (c1.radius * c1.radius - c2.radius * c2.radius + d * d) / (2.0 * d);
        let h = (c1.radius * c1.radius - a * a).sqrt();

        let p = c1.center + (c2.center - c1.center) * (a / d);

        let dir = (c2.center - c1.center) / d;
        let perp = Vector2::new(-dir.y, dir.x);

        if h < EPSILON {
            // 一个交点（相切）
            vec![p]
        } else {
            // 两个交点
            vec![
                p + perp * h,
                p - perp * h,
            ]
        }
    }

    /// 线段-圆弧交点
    fn line_arc_intersection(&self, line: &Line, arc: &Arc) -> Vec<Point2> {
        // 先求线段-完整圆的交点，再过滤在弧范围内的
        let circle = Circle::new(arc.center, arc.radius);
        let circle_intersections = self.line_circle_intersection(line, &circle);

        circle_intersections
            .into_iter()
            .filter(|p| arc.contains_point(p))
            .collect()
    }

    /// 线段-多段线交点
    fn line_polyline_intersection(&self, line: &Line, polyline: &Polyline) -> Vec<Point2> {
        let mut intersections = Vec::new();

        for i in 0..polyline.segment_count() {
            let v1 = &polyline.vertices[i];
            let v2 = &polyline.vertices[(i + 1) % polyline.vertices.len()];

            if v1.bulge.abs() < EPSILON {
                // 直线段
                let seg = Line::new(v1.point, v2.point);
                if let Some(p) = self.line_line_intersection(line, &seg) {
                    intersections.push(p);
                }
            }
            // TODO: 处理弧线段
        }

        intersections
    }
}

impl Default for SnapEngine {
    fn default() -> Self {
        Self::new(SnapConfig::default())
    }
}

// 为Arc添加辅助方法
impl Arc {
    /// 检查点是否在弧上（角度范围内）
    fn contains_point(&self, point: &Point2) -> bool {
        let angle = (point.y - self.center.y).atan2(point.x - self.center.x);
        
        let mut a = angle;
        let mut start = self.start_angle;
        let mut end = self.end_angle;

        // 归一化到 [0, 2π)
        let two_pi = 2.0 * std::f64::consts::PI;
        while a < 0.0 { a += two_pi; }
        while start < 0.0 { start += two_pi; }
        while end < 0.0 { end += two_pi; }
        a %= two_pi;
        start %= two_pi;
        end %= two_pi;

        if start <= end {
            a >= start && a <= end
        } else {
            a >= start || a <= end
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snap_mask() {
        let mut mask = SnapMask::default();
        assert!(mask.is_enabled(SnapType::Endpoint));
        assert!(mask.is_enabled(SnapType::Midpoint));
        assert!(!mask.is_enabled(SnapType::Nearest));

        mask.set(SnapType::Nearest, true);
        assert!(mask.is_enabled(SnapType::Nearest));

        mask.toggle(SnapType::Endpoint);
        assert!(!mask.is_enabled(SnapType::Endpoint));
    }

    #[test]
    fn test_line_intersection() {
        let engine = SnapEngine::default();

        let l1 = Line::new(Point2::new(0.0, 0.0), Point2::new(10.0, 10.0));
        let l2 = Line::new(Point2::new(0.0, 10.0), Point2::new(10.0, 0.0));

        let intersection = engine.line_line_intersection(&l1, &l2);
        assert!(intersection.is_some());

        let p = intersection.unwrap();
        assert!((p.x - 5.0).abs() < EPSILON);
        assert!((p.y - 5.0).abs() < EPSILON);
    }

    #[test]
    fn test_nearest_point_on_line() {
        let engine = SnapEngine::default();
        let line = Line::new(Point2::new(0.0, 0.0), Point2::new(10.0, 0.0));

        // 中间点
        let nearest = engine.nearest_point_on_line(&line, Point2::new(5.0, 5.0));
        assert!((nearest.x - 5.0).abs() < EPSILON);
        assert!((nearest.y).abs() < EPSILON);

        // 线段外的点
        let nearest = engine.nearest_point_on_line(&line, Point2::new(-5.0, 0.0));
        assert!((nearest.x).abs() < EPSILON); // 应该返回起点
    }
}

