//! 空间索引
//!
//! 使用R-tree实现高效的空间查询，支持：
//! - 范围查询
//! - 点击测试
//! - 最近邻查询

use crate::entity::EntityId;
use crate::math::{BoundingBox2, Point2};
use std::collections::HashMap;

/// 空间索引条目（预留给R-tree实现）
#[derive(Debug, Clone)]
struct _SpatialEntry {
    _id: EntityId,
    _bbox: BoundingBox2,
}

/// 简单的空间索引（基于网格）
///
/// 对于更复杂的场景，可以替换为R-tree实现
#[derive(Debug)]
pub struct SpatialIndex {
    /// 网格单元大小
    cell_size: f64,

    /// 网格映射：网格坐标 -> 实体列表
    grid: HashMap<(i64, i64), Vec<EntityId>>,

    /// 实体的包围盒缓存
    bboxes: HashMap<EntityId, BoundingBox2>,
}

impl SpatialIndex {
    /// 创建新的空间索引
    pub fn new(cell_size: f64) -> Self {
        Self {
            cell_size,
            grid: HashMap::new(),
            bboxes: HashMap::new(),
        }
    }

    /// 使用默认网格大小创建
    pub fn default_grid() -> Self {
        Self::new(100.0) // 100单位的网格
    }

    /// 将世界坐标转换为网格坐标
    fn to_grid_coord(&self, x: f64, y: f64) -> (i64, i64) {
        (
            (x / self.cell_size).floor() as i64,
            (y / self.cell_size).floor() as i64,
        )
    }

    /// 获取包围盒覆盖的所有网格单元
    fn cells_for_bbox(&self, bbox: &BoundingBox2) -> Vec<(i64, i64)> {
        let (min_gx, min_gy) = self.to_grid_coord(bbox.min.x, bbox.min.y);
        let (max_gx, max_gy) = self.to_grid_coord(bbox.max.x, bbox.max.y);

        let mut cells = Vec::new();
        for gx in min_gx..=max_gx {
            for gy in min_gy..=max_gy {
                cells.push((gx, gy));
            }
        }
        cells
    }

    /// 插入实体
    pub fn insert(&mut self, id: EntityId, bbox: BoundingBox2) {
        // 先移除旧的（如果存在）
        self.remove(&id);

        // 添加到所有覆盖的网格单元
        for cell in self.cells_for_bbox(&bbox) {
            self.grid.entry(cell).or_default().push(id);
        }

        // 缓存包围盒
        self.bboxes.insert(id, bbox);
    }

    /// 移除实体
    pub fn remove(&mut self, id: &EntityId) -> bool {
        if let Some(bbox) = self.bboxes.remove(id) {
            for cell in self.cells_for_bbox(&bbox) {
                if let Some(entities) = self.grid.get_mut(&cell) {
                    entities.retain(|e| e != id);
                }
            }
            true
        } else {
            false
        }
    }

    /// 更新实体的包围盒
    pub fn update(&mut self, id: EntityId, new_bbox: BoundingBox2) {
        self.insert(id, new_bbox);
    }

    /// 范围查询：查找与指定矩形相交的所有实体
    pub fn query_rect(&self, rect: &BoundingBox2) -> Vec<EntityId> {
        let mut result = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for cell in self.cells_for_bbox(rect) {
            if let Some(entities) = self.grid.get(&cell) {
                for id in entities {
                    if seen.insert(*id) {
                        // 精确检查包围盒相交
                        if let Some(bbox) = self.bboxes.get(id) {
                            if bbox.intersects(rect) {
                                result.push(*id);
                            }
                        }
                    }
                }
            }
        }

        result
    }

    /// 点击测试：查找包含指定点的所有实体
    pub fn query_point(&self, point: &Point2) -> Vec<EntityId> {
        let cell = self.to_grid_coord(point.x, point.y);

        let mut result = Vec::new();

        if let Some(entities) = self.grid.get(&cell) {
            for id in entities {
                if let Some(bbox) = self.bboxes.get(id) {
                    if bbox.contains(point) {
                        result.push(*id);
                    }
                }
            }
        }

        result
    }

    /// 查找最近的实体
    pub fn query_nearest(&self, point: &Point2, max_distance: f64) -> Option<EntityId> {
        let search_bbox = BoundingBox2::new(
            Point2::new(point.x - max_distance, point.y - max_distance),
            Point2::new(point.x + max_distance, point.y + max_distance),
        );

        let candidates = self.query_rect(&search_bbox);

        candidates
            .into_iter()
            .filter_map(|id| {
                self.bboxes.get(&id).map(|bbox| {
                    let center = bbox.center();
                    let dist = (center - point).norm();
                    (id, dist)
                })
            })
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .map(|(id, _)| id)
    }

    /// 清空索引
    pub fn clear(&mut self) {
        self.grid.clear();
        self.bboxes.clear();
    }

    /// 获取实体数量
    pub fn len(&self) -> usize {
        self.bboxes.len()
    }

    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        self.bboxes.is_empty()
    }

    /// 获取实体的包围盒
    pub fn get_bbox(&self, id: &EntityId) -> Option<&BoundingBox2> {
        self.bboxes.get(id)
    }

    /// 重建索引（当大量更新后优化性能）
    pub fn rebuild(&mut self) {
        let entries: Vec<_> = self
            .bboxes
            .iter()
            .map(|(id, bbox)| (*id, *bbox))
            .collect();

        self.grid.clear();

        for (id, bbox) in entries {
            for cell in self.cells_for_bbox(&bbox) {
                self.grid.entry(cell).or_default().push(id);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spatial_index() {
        let mut index = SpatialIndex::new(10.0);

        let id1 = EntityId::new();
        let id2 = EntityId::new();
        let id3 = EntityId::new();

        index.insert(
            id1,
            BoundingBox2::new(Point2::new(0.0, 0.0), Point2::new(5.0, 5.0)),
        );
        index.insert(
            id2,
            BoundingBox2::new(Point2::new(10.0, 10.0), Point2::new(15.0, 15.0)),
        );
        index.insert(
            id3,
            BoundingBox2::new(Point2::new(100.0, 100.0), Point2::new(105.0, 105.0)),
        );

        // 查询与 (0,0)-(20,20) 相交的实体
        let result = index.query_rect(&BoundingBox2::new(
            Point2::new(0.0, 0.0),
            Point2::new(20.0, 20.0),
        ));

        assert_eq!(result.len(), 2);
        assert!(result.contains(&id1));
        assert!(result.contains(&id2));
        assert!(!result.contains(&id3));
    }
}

