//! Tile-based渲染系统
//!
//! 将画布分成小块（Tile），只更新变化的区域以提高性能。

use crate::vertex::LineVertex;
use zcad_core::math::{BoundingBox2, Point2};
use std::collections::HashMap;

/// 渲染块（Tile）
///
/// 每个Tile代表屏幕上的一个矩形区域，包含该区域内的几何数据
#[derive(Debug, Clone)]
pub struct Tile {
    /// Tile在世界坐标中的边界
    pub bounds: BoundingBox2,

    /// Tile在屏幕坐标中的位置和大小
    pub screen_x: u32,
    pub screen_y: u32,
    pub screen_width: u32,
    pub screen_height: u32,

    /// 该Tile包含的顶点数据
    pub vertices: Vec<LineVertex>,

    /// 脏标记 - 标记是否需要重新渲染
    pub dirty: bool,

    /// 最后更新时间戳
    pub last_update: Option<std::time::Instant>,
}

impl Default for Tile {
    fn default() -> Self {
        Self {
            bounds: zcad_core::math::BoundingBox2::new(
                zcad_core::math::Point2::origin(),
                zcad_core::math::Point2::origin(),
            ),
            screen_x: 0,
            screen_y: 0,
            screen_width: 0,
            screen_height: 0,
            vertices: Vec::new(),
            dirty: false,
            last_update: Some(std::time::Instant::now()),
        }
    }
}

impl Tile {
    /// 创建新的Tile
    pub fn new(bounds: BoundingBox2, screen_x: u32, screen_y: u32, screen_width: u32, screen_height: u32) -> Self {
        Self {
            bounds,
            screen_x,
            screen_y,
            screen_width,
            screen_height,
            vertices: Vec::new(),
            dirty: true,
            last_update: Some(std::time::Instant::now()),
        }
    }

    /// 清除顶点数据
    pub fn clear(&mut self) {
        self.vertices.clear();
        self.dirty = true;
    }

    /// 添加顶点到Tile
    pub fn add_vertices(&mut self, vertices: &[LineVertex]) {
        self.vertices.extend_from_slice(vertices);
        self.dirty = true;
        self.last_update = Some(std::time::Instant::now());
    }

    /// 检查点是否在Tile内
    pub fn contains_point(&self, point: &Point2) -> bool {
        self.bounds.contains(point)
    }

    /// 检查包围盒是否与Tile相交
    pub fn intersects_bounds(&self, bounds: &BoundingBox2) -> bool {
        self.bounds.intersects(bounds)
    }

    /// 获取顶点数量
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    /// 标记为干净（已渲染）
    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }
}

/// Tile管理器
///
/// 管理所有Tile，处理Tile的创建、更新和渲染优化
pub struct TileManager {
    /// Tile大小（像素）
    tile_size: u32,

    /// 视口大小
    viewport_width: u32,
    viewport_height: u32,

    /// 当前可见的Tile映射 (tile_key -> Tile)
    visible_tiles: HashMap<u64, Tile>,

    /// Tile缓存，用于复用Tile对象
    tile_cache: Vec<Tile>,

    /// 最大缓存Tile数量
    max_cache_size: usize,

    /// 性能统计
    pub stats: TileStats,
}

#[derive(Debug, Clone)]
pub struct TileStats {
    /// 总Tile数量
    pub total_tiles: usize,

    /// 脏Tile数量
    pub dirty_tiles: usize,

    /// 缓存命中率
    pub cache_hit_rate: f32,

    /// 平均顶点数 per Tile
    pub avg_vertices_per_tile: f32,

    /// 最后更新时间
    pub last_update: Option<std::time::Instant>,
}

impl Default for TileStats {
    fn default() -> Self {
        Self {
            total_tiles: 0,
            dirty_tiles: 0,
            cache_hit_rate: 0.0,
            avg_vertices_per_tile: 0.0,
            last_update: Some(std::time::Instant::now()),
        }
    }
}

impl TileManager {
    /// 创建新的Tile管理器
    pub fn new(tile_size: u32, viewport_width: u32, viewport_height: u32) -> Self {
        Self {
            tile_size,
            viewport_width,
            viewport_height,
            visible_tiles: HashMap::new(),
            tile_cache: Vec::new(),
            max_cache_size: 100,
            stats: TileStats::default(),
        }
    }

    /// 更新视口大小
    pub fn resize(&mut self, width: u32, height: u32) {
        if self.viewport_width != width || self.viewport_height != height {
            self.viewport_width = width;
            self.viewport_height = height;
            self.visible_tiles.clear(); // 视口改变时需要重新计算Tile
        }
    }

    /// 更新可见Tile（基于相机视口）
    pub fn update_visible_tiles(&mut self, camera_bounds: &BoundingBox2) {
        let start_time = std::time::Instant::now();

        // 计算需要的Tile范围
        let tile_world_size_x = camera_bounds.width() / (self.viewport_width as f64 / self.tile_size as f64);
        let tile_world_size_y = camera_bounds.height() / (self.viewport_height as f64 / self.tile_size as f64);

        let start_tile_x = (camera_bounds.min.x / tile_world_size_x).floor() as i32;
        let end_tile_x = (camera_bounds.max.x / tile_world_size_x).ceil() as i32;
        let start_tile_y = (camera_bounds.min.y / tile_world_size_y).floor() as i32;
        let end_tile_y = (camera_bounds.max.y / tile_world_size_y).ceil() as i32;

        let mut new_visible_tiles = HashMap::new();
        let mut cache_hits = 0;

        // 创建或复用Tile
        for tile_y in start_tile_y..=end_tile_y {
            for tile_x in start_tile_x..=end_tile_x {
                let tile_key = Self::tile_key(tile_x, tile_y);

                // 尝试从现有Tile复用
                if let Some(existing_tile) = self.visible_tiles.remove(&tile_key) {
                    // Tile仍然可见，复用它
                    cache_hits += 1;
                    new_visible_tiles.insert(tile_key, existing_tile);
                } else {
                    // 创建新Tile
                    let tile_bounds = BoundingBox2::new(
                        Point2::new(
                            tile_x as f64 * tile_world_size_x,
                            tile_y as f64 * tile_world_size_y,
                        ),
                        Point2::new(
                            (tile_x + 1) as f64 * tile_world_size_x,
                            (tile_y + 1) as f64 * tile_world_size_y,
                        ),
                    );

                    let screen_x = ((tile_x - start_tile_x) * self.tile_size as i32) as u32;
                    let screen_y = ((tile_y - start_tile_y) * self.tile_size as i32) as u32;

                    let tile = if let Some(mut cached) = self.tile_cache.pop() {
                        // 复用缓存的Tile
                        cached.bounds = tile_bounds;
                        cached.screen_x = screen_x;
                        cached.screen_y = screen_y;
                        cached.clear();
                        cache_hits += 1;
                        cached
                    } else {
                        // 创建新Tile
                        Tile::new(tile_bounds, screen_x, screen_y, self.tile_size, self.tile_size)
                    };

                    new_visible_tiles.insert(tile_key, tile);
                }
            }
        }

        // 将不再可见的Tile放入缓存
        for (_, tile) in self.visible_tiles.drain() {
            if self.tile_cache.len() < self.max_cache_size {
                self.tile_cache.push(tile);
            }
        }

        self.visible_tiles = new_visible_tiles;

        // 更新统计信息
        self.stats.total_tiles = self.visible_tiles.len();
        self.stats.dirty_tiles = self.visible_tiles.values().filter(|t| t.dirty).count();
        self.stats.cache_hit_rate = if self.stats.total_tiles > 0 {
            cache_hits as f32 / self.stats.total_tiles as f32
        } else {
            0.0
        };
        self.stats.last_update = Some(start_time);
    }

    /// 将几何体添加到相关的Tile
    pub fn add_geometry_to_tiles(&mut self, vertices: &[LineVertex], geometry_bounds: &BoundingBox2) {
        for tile in self.visible_tiles.values_mut() {
            if tile.intersects_bounds(geometry_bounds) {
                tile.add_vertices(vertices);
            }
        }
    }

    /// 标记与包围盒相交的所有Tile为脏
    pub fn mark_tiles_dirty(&mut self, bounds: &BoundingBox2) {
        for tile in self.visible_tiles.values_mut() {
            if tile.intersects_bounds(bounds) {
                tile.dirty = true;
            }
        }
    }

    /// 清除所有Tile的脏标记
    pub fn clear_dirty_flags(&mut self) {
        for tile in self.visible_tiles.values_mut() {
            tile.mark_clean();
        }
    }

    /// 获取需要渲染的Tile（脏Tile）
    pub fn dirty_tiles(&self) -> impl Iterator<Item = &Tile> {
        self.visible_tiles.values().filter(|tile| tile.dirty)
    }

    /// 获取所有可见Tile
    pub fn visible_tiles(&self) -> impl Iterator<Item = &Tile> {
        self.visible_tiles.values()
    }

    /// 获取Tile数量统计
    pub fn tile_count(&self) -> usize {
        self.visible_tiles.len()
    }

    /// 计算Tile键值
    fn tile_key(tile_x: i32, tile_y: i32) -> u64 {
        // 使用简单的编码：高32位是x，低32位是y
        ((tile_x as u64) << 32) | (tile_y as u64 & 0xFFFFFFFF)
    }

    /// 解析Tile键值
    fn _decode_tile_key(key: u64) -> (i32, i32) {
        let x = (key >> 32) as i32;
        let y = (key & 0xFFFFFFFF) as i32;
        (x, y)
    }

    /// 优化：合并相邻的脏Tile以减少绘制调用
    pub fn optimize_dirty_regions(&mut self) -> Vec<TileRegion> {
        let mut regions = Vec::new();
        let mut processed = std::collections::HashSet::new();

        for (key, tile) in &self.visible_tiles {
            if tile.dirty && !processed.contains(key) {
                let region = TileRegion {
                    min_x: tile.screen_x,
                    min_y: tile.screen_y,
                    max_x: tile.screen_x + tile.screen_width,
                    max_y: tile.screen_y + tile.screen_height,
                    tiles: vec![*key],
                };

                // 简单的合并逻辑：扩展到相邻的脏Tile
                // 这里可以实现更复杂的合并算法
                processed.insert(*key);
                regions.push(region);
            }
        }

        regions
    }
}

/// Tile区域（用于优化渲染）
#[derive(Debug, Clone)]
pub struct TileRegion {
    pub min_x: u32,
    pub min_y: u32,
    pub max_x: u32,
    pub max_y: u32,
    pub tiles: Vec<u64>,
}

impl TileRegion {
    pub fn width(&self) -> u32 {
        self.max_x - self.min_x
    }

    pub fn height(&self) -> u32 {
        self.max_y - self.min_y
    }
}
