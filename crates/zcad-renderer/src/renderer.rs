//! 主渲染器
//! 
//! 这个模块提供基于wgpu的GPU渲染能力。
//! 当前版本主要用于未来扩展，egui渲染由eframe处理。

use crate::camera::Camera2D;
use crate::compute::{ComputeShader, BooleanOp};
use crate::pipeline::LinePipeline;
use crate::tile::TileManager;
use crate::vertex::{CameraUniform, LineVertex};
use thiserror::Error;
use zcad_core::math::BoundingBox2;
use wgpu::util::DeviceExt;
use zcad_core::geometry::{Arc, Circle, Geometry, Line, Polyline};
use zcad_core::math::Point2;
use zcad_core::properties::Color;

/// 渲染器错误
#[derive(Error, Debug)]
pub enum RenderError {
    #[error("Failed to create surface: {0}")]
    SurfaceError(#[from] wgpu::CreateSurfaceError),

    #[error("Failed to request adapter: {0}")]
    AdapterError(#[from] wgpu::RequestAdapterError),

    #[error("Failed to request device: {0}")]
    DeviceError(#[from] wgpu::RequestDeviceError),

    #[error("Surface error: {0}")]
    Surface(#[from] wgpu::SurfaceError),

    #[error("Compute error: {0}")]
    Compute(String),
}

/// GPU渲染器（保留用于未来的高性能渲染需求）
pub struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,

    line_pipeline: LinePipeline,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,

    // 计算着色器
    compute_shader: ComputeShader,

    // Tile-based渲染系统
    tile_manager: TileManager,

    // 渲染缓冲区
    line_vertices: Vec<LineVertex>,

    // 网格设置
    grid_visible: bool,
    grid_spacing: f64,
    grid_color: Color,
}

impl Renderer {
    /// 创建新的渲染器
    pub async fn new(window: std::sync::Arc<winit::window::Window>) -> Result<Self, RenderError> {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let surface = instance.create_surface(window)?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("ZCAD Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: Default::default(),
                experimental_features: Default::default(),
                trace: Default::default(),
            })
            .await?;

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surface_config);

        // 创建管线
        let line_pipeline = LinePipeline::new(&device, surface_format);

        // 创建计算着色器
        let compute_shader = ComputeShader::new(&device, &queue).map_err(|e| RenderError::Compute(e.to_string()))?;

        // 创建Tile管理器
        let tile_manager = TileManager::new(256, size.width, size.height); // 256x256像素的Tile

        // 创建相机缓冲区
        let camera_uniform = CameraUniform::new();
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera Bind Group"),
            layout: &line_pipeline.camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        Ok(Self {
            device,
            queue,
            surface,
            surface_config,
            line_pipeline,
            camera_buffer,
            camera_bind_group,
            compute_shader,
            tile_manager,
            line_vertices: Vec::new(),
            grid_visible: true,
            grid_spacing: 50.0,
            grid_color: Color::new(60, 60, 70),
        })
    }

    /// 调整视口大小
    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.surface_config.width = width;
            self.surface_config.height = height;
            self.surface.configure(&self.device, &self.surface_config);
            self.tile_manager.resize(width, height);
        }
    }

    /// 更新相机
    pub fn update_camera(&mut self, camera: &Camera2D) {
        let uniform = camera.to_uniform();
        self.queue
            .write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[uniform]));
    }

    /// 设置网格可见性
    pub fn set_grid_visible(&mut self, visible: bool) {
        self.grid_visible = visible;
    }

    /// 设置网格间距
    pub fn set_grid_spacing(&mut self, spacing: f64) {
        self.grid_spacing = spacing;
    }

    /// 清空渲染缓冲区
    pub fn begin_frame(&mut self) {
        self.line_vertices.clear();
    }

    /// 绘制网格
    pub fn draw_grid(&mut self, camera: &Camera2D) {
        if !self.grid_visible {
            return;
        }

        let bounds = camera.visible_bounds();
        let color = self.grid_color.to_f32_array();
        
        // 根据缩放级别调整网格间距
        let mut spacing = self.grid_spacing;
        while spacing * camera.zoom < 20.0 {
            spacing *= 5.0;
        }
        while spacing * camera.zoom > 200.0 {
            spacing /= 5.0;
        }

        // 计算网格范围
        let start_x = (bounds.min.x / spacing).floor() * spacing;
        let end_x = (bounds.max.x / spacing).ceil() * spacing;
        let start_y = (bounds.min.y / spacing).floor() * spacing;
        let end_y = (bounds.max.y / spacing).ceil() * spacing;

        // 绘制垂直线
        let mut x = start_x;
        while x <= end_x {
            self.line_vertices.push(LineVertex::new(x as f32, start_y as f32, color));
            self.line_vertices.push(LineVertex::new(x as f32, end_y as f32, color));
            x += spacing;
        }

        // 绘制水平线
        let mut y = start_y;
        while y <= end_y {
            self.line_vertices.push(LineVertex::new(start_x as f32, y as f32, color));
            self.line_vertices.push(LineVertex::new(end_x as f32, y as f32, color));
            y += spacing;
        }

        // 绘制坐标轴（更明显的颜色）
        let axis_color = Color::new(100, 100, 120).to_f32_array();
        
        // X轴
        if bounds.min.y <= 0.0 && bounds.max.y >= 0.0 {
            self.line_vertices.push(LineVertex::new(start_x as f32, 0.0, axis_color));
            self.line_vertices.push(LineVertex::new(end_x as f32, 0.0, axis_color));
        }
        
        // Y轴
        if bounds.min.x <= 0.0 && bounds.max.x >= 0.0 {
            self.line_vertices.push(LineVertex::new(0.0, start_y as f32, axis_color));
            self.line_vertices.push(LineVertex::new(0.0, end_y as f32, axis_color));
        }
    }

    /// 绘制十字光标
    pub fn draw_crosshair(&mut self, pos: Point2, camera: &Camera2D) {
        let size = 15.0 / camera.zoom; // 固定屏幕大小
        let color = Color::WHITE.to_f32_array();
        
        // 水平线
        self.line_vertices.push(LineVertex::new((pos.x - size) as f32, pos.y as f32, color));
        self.line_vertices.push(LineVertex::new((pos.x + size) as f32, pos.y as f32, color));
        
        // 垂直线
        self.line_vertices.push(LineVertex::new(pos.x as f32, (pos.y - size) as f32, color));
        self.line_vertices.push(LineVertex::new(pos.x as f32, (pos.y + size) as f32, color));
    }

    /// 添加几何体到渲染批次
    pub fn draw_geometry(&mut self, geometry: &Geometry, color: Color) {
        let color_arr = color.to_f32_array();

        match geometry {
            Geometry::Point(p) => {
                let size = 3.0;
                let x = p.position.x as f32;
                let y = p.position.y as f32;
                self.line_vertices.push(LineVertex::new(x - size, y, color_arr));
                self.line_vertices.push(LineVertex::new(x + size, y, color_arr));
                self.line_vertices.push(LineVertex::new(x, y - size, color_arr));
                self.line_vertices.push(LineVertex::new(x, y + size, color_arr));
            }
            Geometry::Line(line) => {
                self.draw_line(line, color_arr);
            }
            Geometry::Circle(circle) => {
                self.draw_circle(circle, color_arr);
            }
            Geometry::Arc(arc) => {
                self.draw_arc(arc, color_arr);
            }
            Geometry::Polyline(polyline) => {
                self.draw_polyline(polyline, color_arr);
            }
        }
    }

    fn draw_line(&mut self, line: &Line, color: [f32; 4]) {
        self.line_vertices.push(LineVertex::new(
            line.start.x as f32,
            line.start.y as f32,
            color,
        ));
        self.line_vertices.push(LineVertex::new(
            line.end.x as f32,
            line.end.y as f32,
            color,
        ));
    }

    fn draw_circle(&mut self, circle: &Circle, color: [f32; 4]) {
        let segments = (circle.radius * 2.0).clamp(32.0, 256.0) as usize;
        let angle_step = 2.0 * std::f64::consts::PI / segments as f64;

        for i in 0..segments {
            let a1 = i as f64 * angle_step;
            let a2 = (i + 1) as f64 * angle_step;

            let p1 = circle.point_at_angle(a1);
            let p2 = circle.point_at_angle(a2);

            self.line_vertices
                .push(LineVertex::new(p1.x as f32, p1.y as f32, color));
            self.line_vertices
                .push(LineVertex::new(p2.x as f32, p2.y as f32, color));
        }
    }

    fn draw_arc(&mut self, arc: &Arc, color: [f32; 4]) {
        let sweep = arc.sweep_angle();
        let segments = ((arc.radius * sweep.abs()).clamp(8.0, 128.0)) as usize;
        let angle_step = sweep / segments as f64;

        for i in 0..segments {
            let a1 = arc.start_angle + i as f64 * angle_step;
            let a2 = arc.start_angle + (i + 1) as f64 * angle_step;

            let p1 = Point2::new(
                arc.center.x + arc.radius * a1.cos(),
                arc.center.y + arc.radius * a1.sin(),
            );
            let p2 = Point2::new(
                arc.center.x + arc.radius * a2.cos(),
                arc.center.y + arc.radius * a2.sin(),
            );

            self.line_vertices
                .push(LineVertex::new(p1.x as f32, p1.y as f32, color));
            self.line_vertices
                .push(LineVertex::new(p2.x as f32, p2.y as f32, color));
        }
    }

    fn draw_polyline(&mut self, polyline: &Polyline, color: [f32; 4]) {
        if polyline.vertices.len() < 2 {
            return;
        }

        for i in 0..polyline.segment_count() {
            let v1 = &polyline.vertices[i];
            let v2 = &polyline.vertices[(i + 1) % polyline.vertices.len()];

            if v1.bulge.abs() < 0.001 {
                self.line_vertices.push(LineVertex::new(
                    v1.point.x as f32,
                    v1.point.y as f32,
                    color,
                ));
                self.line_vertices.push(LineVertex::new(
                    v2.point.x as f32,
                    v2.point.y as f32,
                    color,
                ));
            } else {
                // 弧线段细分
                self.draw_bulge_segment(v1.point, v2.point, v1.bulge, color);
            }
        }
    }

    fn draw_bulge_segment(&mut self, p1: Point2, p2: Point2, bulge: f64, color: [f32; 4]) {
        let chord = p2 - p1;
        let chord_len = chord.norm();
        
        if chord_len < 0.001 {
            return;
        }

        let s = chord_len / 2.0;
        let h = s * bulge.abs();
        
        if h < 0.001 {
            self.line_vertices.push(LineVertex::new(p1.x as f32, p1.y as f32, color));
            self.line_vertices.push(LineVertex::new(p2.x as f32, p2.y as f32, color));
            return;
        }

        let radius = (s * s + h * h) / (2.0 * h);
        let angle = 4.0 * bulge.abs().atan();
        
        let mid = Point2::new((p1.x + p2.x) / 2.0, (p1.y + p2.y) / 2.0);
        let d = radius - h;
        
        let perp = if bulge > 0.0 {
            zcad_core::math::Vector2::new(-chord.y, chord.x).normalize()
        } else {
            zcad_core::math::Vector2::new(chord.y, -chord.x).normalize()
        };
        
        let center = mid + perp * d;
        let start_angle = (p1.y - center.y).atan2(p1.x - center.x);
        
        let segments = (radius * angle.abs()).clamp(4.0, 64.0) as usize;
        let angle_step = if bulge > 0.0 { angle / segments as f64 } else { -angle / segments as f64 };

        for i in 0..segments {
            let a1 = start_angle + i as f64 * angle_step;
            let a2 = start_angle + (i + 1) as f64 * angle_step;

            let pt1 = Point2::new(center.x + radius * a1.cos(), center.y + radius * a1.sin());
            let pt2 = Point2::new(center.x + radius * a2.cos(), center.y + radius * a2.sin());

            self.line_vertices.push(LineVertex::new(pt1.x as f32, pt1.y as f32, color));
            self.line_vertices.push(LineVertex::new(pt2.x as f32, pt2.y as f32, color));
        }
    }

    /// 执行Tile-based渲染
    pub fn render(&mut self, clear_color: Color) -> Result<(), RenderError> {
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Tile-based Render Encoder"),
        });

        // Tile-based渲染：只渲染脏Tile
        let _dirty_regions = self.tile_manager.optimize_dirty_regions();

        {
            let clear = clear_color.to_f32_array();
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Tile-based Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: clear[0] as f64,
                            g: clear[1] as f64,
                            b: clear[2] as f64,
                            a: clear[3] as f64,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.line_pipeline.pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);

            // 渲染所有可见Tile（包括非脏的，用于完整画面）
            for tile in self.tile_manager.visible_tiles() {
                if !tile.vertices.is_empty() {
                    // 为每个Tile创建顶点缓冲区
                    let vertex_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some(&format!("Tile Vertex Buffer ({}, {})", tile.screen_x, tile.screen_y)),
                        contents: bytemuck::cast_slice(&tile.vertices),
                        usage: wgpu::BufferUsages::VERTEX,
                    });

                    render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                    render_pass.draw(0..tile.vertex_count() as u32, 0..1);
                }
            }

            // 同时渲染全局几何体（如十字光标等）
            if !self.line_vertices.is_empty() {
                let global_vertex_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Global Line Vertex Buffer"),
                    contents: bytemuck::cast_slice(&self.line_vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });

                render_pass.set_vertex_buffer(0, global_vertex_buffer.slice(..));
                render_pass.draw(0..self.line_vertices.len() as u32, 0..1);
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        // 清除脏标记
        self.tile_manager.clear_dirty_flags();

        Ok(())
    }

    /// 获取设备引用
    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    /// 获取队列引用
    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    /// 获取表面格式
    pub fn surface_format(&self) -> wgpu::TextureFormat {
        self.surface_config.format
    }

    /// 获取视口尺寸
    pub fn size(&self) -> (u32, u32) {
        (self.surface_config.width, self.surface_config.height)
    }

    /// 执行布尔运算（异步）
    pub async fn boolean_operation(
        &self,
        geom1: &Geometry,
        geom2: &Geometry,
        operation: BooleanOp,
        tolerance: f64,
    ) -> Result<Vec<Geometry>, RenderError> {
        self.compute_shader.boolean_operation(geom1, geom2, operation, tolerance)
            .await
            .map_err(|e| RenderError::Compute(e.to_string()))
    }

    /// 执行几何偏移（异步）
    pub async fn offset_geometry(
        &self,
        geometry: &Geometry,
        distance: f64,
        tolerance: f64,
    ) -> Result<Vec<Geometry>, RenderError> {
        self.compute_shader.offset_geometry(geometry, distance, tolerance)
            .await
            .map_err(|e| RenderError::Compute(e.to_string()))
    }

    /// 更新Tile系统（基于当前相机）
    pub fn update_tiles(&mut self, camera: &Camera2D) {
        let bounds = camera.visible_bounds();
        self.tile_manager.update_visible_tiles(&bounds);
    }

    /// 添加几何体到Tile系统
    pub fn add_geometry_to_tiles(&mut self, geometry: &Geometry, color: Color) {
        let mut temp_vertices = Vec::new();
        self.draw_geometry_to_buffer(geometry, color, &mut temp_vertices);

        if !temp_vertices.is_empty() {
            let bounds = geometry.bounding_box();
            self.tile_manager.add_geometry_to_tiles(&temp_vertices, &bounds);
        }
    }

    /// 标记区域为脏（需要重新渲染）
    pub fn mark_region_dirty(&mut self, bounds: &BoundingBox2) {
        self.tile_manager.mark_tiles_dirty(bounds);
    }

    /// 获取Tile统计信息
    pub fn tile_stats(&self) -> &crate::tile::TileStats {
        &self.tile_manager.stats
    }

    /// 辅助方法：将几何体绘制到顶点缓冲区
    fn draw_geometry_to_buffer(&self, geometry: &Geometry, color: Color, vertices: &mut Vec<LineVertex>) {
        let color_arr = color.to_f32_array();

        match geometry {
            Geometry::Point(p) => {
                let size = 3.0;
                let x = p.position.x as f32;
                let y = p.position.y as f32;
                vertices.push(LineVertex::new(x - size, y, color_arr));
                vertices.push(LineVertex::new(x + size, y, color_arr));
                vertices.push(LineVertex::new(x, y - size, color_arr));
                vertices.push(LineVertex::new(x, y + size, color_arr));
            }
            Geometry::Line(line) => {
                vertices.push(LineVertex::new(
                    line.start.x as f32,
                    line.start.y as f32,
                    color_arr,
                ));
                vertices.push(LineVertex::new(
                    line.end.x as f32,
                    line.end.y as f32,
                    color_arr,
                ));
            }
            Geometry::Circle(circle) => {
                let segments = (circle.radius * 2.0).clamp(32.0, 256.0) as usize;
                let angle_step = 2.0 * std::f64::consts::PI / segments as f64;

                for i in 0..segments {
                    let a1 = i as f64 * angle_step;
                    let a2 = (i + 1) as f64 * angle_step;

                    let p1 = circle.point_at_angle(a1);
                    let p2 = circle.point_at_angle(a2);

                    vertices.push(LineVertex::new(p1.x as f32, p1.y as f32, color_arr));
                    vertices.push(LineVertex::new(p2.x as f32, p2.y as f32, color_arr));
                }
            }
            Geometry::Arc(arc) => {
                let sweep = arc.sweep_angle();
                let segments = ((arc.radius * sweep.abs()).clamp(8.0, 128.0)) as usize;
                let angle_step = sweep / segments as f64;

                for i in 0..segments {
                    let a1 = arc.start_angle + i as f64 * angle_step;
                    let a2 = arc.start_angle + (i + 1) as f64 * angle_step;

                    let p1 = Point2::new(
                        arc.center.x + arc.radius * a1.cos(),
                        arc.center.y + arc.radius * a1.sin(),
                    );
                    let p2 = Point2::new(
                        arc.center.x + arc.radius * a2.cos(),
                        arc.center.y + arc.radius * a2.sin(),
                    );

                    vertices.push(LineVertex::new(p1.x as f32, p1.y as f32, color_arr));
                    vertices.push(LineVertex::new(p2.x as f32, p2.y as f32, color_arr));
                }
            }
            Geometry::Polyline(polyline) => {
                if polyline.vertices.len() < 2 {
                    return;
                }

                for i in 0..polyline.segment_count() {
                    let v1 = &polyline.vertices[i];
                    let v2 = &polyline.vertices[(i + 1) % polyline.vertices.len()];

                    if v1.bulge.abs() < 0.001 {
                        vertices.push(LineVertex::new(
                            v1.point.x as f32,
                            v1.point.y as f32,
                            color_arr,
                        ));
                        vertices.push(LineVertex::new(
                            v2.point.x as f32,
                            v2.point.y as f32,
                            color_arr,
                        ));
                    } else {
                        // 简化的弧线处理（实际应该细分）
                        vertices.push(LineVertex::new(
                            v1.point.x as f32,
                            v1.point.y as f32,
                            color_arr,
                        ));
                        vertices.push(LineVertex::new(
                            v2.point.x as f32,
                            v2.point.y as f32,
                            color_arr,
                        ));
                    }
                }
            }
        }
    }
}
