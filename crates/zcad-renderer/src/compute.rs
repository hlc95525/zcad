//! GPU计算着色器模块
//!
//! 提供GPU加速的几何运算功能：
//! - 布尔运算（并集、交集、差集）
//! - 偏移/膨胀/腐蚀
//! - 路径简化
//! - 几何变换

use thiserror::Error;
use wgpu::util::DeviceExt;
use zcad_core::geometry::{Circle, Geometry, Line, Point};
use zcad_core::math::Point2;

/// 计算着色器错误
#[derive(Error, Debug)]
pub enum ComputeError {
    #[error("Failed to create compute pipeline: {0}")]
    PipelineError(#[from] wgpu::Error),

    #[error("Buffer creation failed: {0}")]
    BufferError(String),

    #[error("Compute operation failed: {0}")]
    OperationError(String),
}

/// GPU计算着色器
pub struct ComputeShader {
    device: wgpu::Device,
    queue: wgpu::Queue,

    // 布尔运算管线
    boolean_pipeline: wgpu::ComputePipeline,
    boolean_bind_group_layout: wgpu::BindGroupLayout,

    // 偏移管线
    offset_pipeline: wgpu::ComputePipeline,
    offset_bind_group_layout: wgpu::BindGroupLayout,

    // 几何变换管线（预留）
    _transform_pipeline: wgpu::ComputePipeline,
    _transform_bind_group_layout: wgpu::BindGroupLayout,
}

impl ComputeShader {
    /// 创建新的计算着色器实例
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Result<Self, ComputeError> {
        let boolean_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Boolean Compute Bind Group Layout"),
                entries: &[
                    // 输入几何体1
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // 输入几何体2
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // 输出结果
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // 参数缓冲区（操作类型、容差等）
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let offset_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Offset Compute Bind Group Layout"),
                entries: &[
                    // 输入几何体
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // 输出结果
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // 参数缓冲区（偏移距离、细分度等）
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let transform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Transform Compute Bind Group Layout"),
                entries: &[
                    // 输入几何体
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // 输出结果
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // 变换矩阵
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        // 创建计算管线
        let boolean_pipeline = Self::create_compute_pipeline(
            device,
            &boolean_bind_group_layout,
            include_str!("shaders/boolean_compute.wgsl"),
            "Boolean Compute Pipeline",
        )?;

        let offset_pipeline = Self::create_compute_pipeline(
            device,
            &offset_bind_group_layout,
            include_str!("shaders/offset_compute.wgsl"),
            "Offset Compute Pipeline",
        )?;

        let transform_pipeline = Self::create_compute_pipeline(
            device,
            &transform_bind_group_layout,
            include_str!("shaders/transform_compute.wgsl"),
            "Transform Compute Pipeline",
        )?;

        Ok(Self {
            device: device.clone(),
            queue: queue.clone(),
            boolean_pipeline,
            boolean_bind_group_layout,
            offset_pipeline,
            offset_bind_group_layout,
            _transform_pipeline: transform_pipeline,
            _transform_bind_group_layout: transform_bind_group_layout,
        })
    }

    /// 创建计算管线
    fn create_compute_pipeline(
        device: &wgpu::Device,
        bind_group_layout: &wgpu::BindGroupLayout,
        shader_source: &str,
        label: &str,
    ) -> Result<wgpu::ComputePipeline, ComputeError> {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(label),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(&format!("{} Layout", label)),
            bind_group_layouts: &[bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some(label),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        Ok(pipeline)
    }

    /// 执行布尔运算
    pub async fn boolean_operation(
        &self,
        geom1: &Geometry,
        geom2: &Geometry,
        operation: BooleanOp,
        tolerance: f64,
    ) -> Result<Vec<Geometry>, ComputeError> {
        // 将几何体转换为GPU缓冲区格式
        let geom1_data = self.geometry_to_gpu_data(geom1);
        let geom2_data = self.geometry_to_gpu_data(geom2);

        // 创建输入缓冲区
        let input1_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Boolean Input 1"),
            contents: bytemuck::cast_slice(&geom1_data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let input2_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Boolean Input 2"),
            contents: bytemuck::cast_slice(&geom2_data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // 创建输出缓冲区（预分配足够空间）
        let output_size = (geom1_data.len() + geom2_data.len()) * 2;
        let output_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Boolean Output"),
            size: (output_size * std::mem::size_of::<GpuGeometryData>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // 参数缓冲区
        let params = BooleanParams {
            operation: operation as u32,
            tolerance: tolerance as f32,
            input1_count: geom1_data.len() as u32,
            input2_count: geom2_data.len() as u32,
        };
        let params_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Boolean Params"),
            contents: bytemuck::cast_slice(&[params]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // 创建绑定组
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Boolean Compute Bind Group"),
            layout: &self.boolean_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: input1_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: input2_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: output_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        // 执行计算
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Boolean Compute Encoder"),
        });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Boolean Compute Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.boolean_pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);

            // 计算工作组数量（根据输入几何体的复杂度）
            let workgroups = ((geom1_data.len() + geom2_data.len()) / 64).max(1);
            compute_pass.dispatch_workgroups(workgroups as u32, 1, 1);
        }

        // 复制结果到可读缓冲区
        let staging_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Boolean Staging Buffer"),
            size: output_buffer.size(),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        encoder.copy_buffer_to_buffer(&output_buffer, 0, &staging_buffer, 0, output_buffer.size());

        self.queue.submit(std::iter::once(encoder.finish()));

        // 读取结果
        let buffer_slice = staging_buffer.slice(..);
        let (sender, receiver) = futures::channel::oneshot::channel();

        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });

        // self.device.poll(wgpu::PollType::WaitForSubmissionIndex {
        //     submission_index: 0,
        //     timeout: None,
        // });

        match receiver.await {
            Ok(Ok(())) => {
                let data = buffer_slice.get_mapped_range();
                let result_data: &[GpuGeometryData] = bytemuck::cast_slice(&data);

                // 转换回Geometry
                let mut result = Vec::new();
                for gpu_data in result_data {
                    if gpu_data.geometry_type != 0 { // 非空几何体
                        if let Some(geom) = self.gpu_data_to_geometry(gpu_data) {
                            result.push(geom);
                        }
                    }
                }

                drop(data);
                staging_buffer.unmap();

                Ok(result)
            }
            _ => Err(ComputeError::OperationError("Failed to read compute results".to_string())),
        }
    }

    /// 执行偏移运算
    pub async fn offset_geometry(
        &self,
        geometry: &Geometry,
        distance: f64,
        tolerance: f64,
    ) -> Result<Vec<Geometry>, ComputeError> {
        let geom_data = self.geometry_to_gpu_data(geometry);

        let input_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Offset Input"),
            contents: bytemuck::cast_slice(&geom_data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let output_size = geom_data.len() * 4; // 偏移可能产生更多顶点
        let output_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Offset Output"),
            size: (output_size * std::mem::size_of::<GpuGeometryData>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let params = OffsetParams {
            distance: distance as f32,
            tolerance: tolerance as f32,
            input_count: geom_data.len() as u32,
        };
        let params_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Offset Params"),
            contents: bytemuck::cast_slice(&[params]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Offset Compute Bind Group"),
            layout: &self.offset_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: input_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: output_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Offset Compute Encoder"),
        });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Offset Compute Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.offset_pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);

            let workgroups = (geom_data.len() / 64).max(1);
            compute_pass.dispatch_workgroups(workgroups as u32, 1, 1);
        }

        let staging_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Offset Staging Buffer"),
            size: output_buffer.size(),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        encoder.copy_buffer_to_buffer(&output_buffer, 0, &staging_buffer, 0, output_buffer.size());
        self.queue.submit(std::iter::once(encoder.finish()));

        let buffer_slice = staging_buffer.slice(..);
        let (sender, receiver) = futures::channel::oneshot::channel();

        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });

        // self.device.poll(wgpu::PollType::WaitForSubmissionIndex {
        //     submission_index: 0,
        //     timeout: None,
        // });

        match receiver.await {
            Ok(Ok(())) => {
                let data = buffer_slice.get_mapped_range();
                let result_data: &[GpuGeometryData] = bytemuck::cast_slice(&data);

                let mut result = Vec::new();
                for gpu_data in result_data {
                    if gpu_data.geometry_type != 0 {
                        if let Some(geom) = self.gpu_data_to_geometry(gpu_data) {
                            result.push(geom);
                        }
                    }
                }

                drop(data);
                staging_buffer.unmap();

                Ok(result)
            }
            _ => Err(ComputeError::OperationError("Failed to read offset results".to_string())),
        }
    }

    /// 将几何体转换为GPU数据格式
    fn geometry_to_gpu_data(&self, geometry: &Geometry) -> Vec<GpuGeometryData> {
        match geometry {
            Geometry::Line(line) => vec![
                GpuGeometryData {
                    geometry_type: 1, // Line
                    x1: line.start.x as f32,
                    y1: line.start.y as f32,
                    x2: line.end.x as f32,
                    y2: line.end.y as f32,
                    radius: 0.0,
                    bulge: 0.0,
                    param1: 0.0,
                    param2: 0.0,
                }
            ],
            Geometry::Circle(circle) => vec![
                GpuGeometryData {
                    geometry_type: 2, // Circle
                    x1: circle.center.x as f32,
                    y1: circle.center.y as f32,
                    x2: 0.0,
                    y2: 0.0,
                    radius: circle.radius as f32,
                    bulge: 0.0,
                    param1: 0.0,
                    param2: 0.0,
                }
            ],
            Geometry::Point(point) => vec![
                GpuGeometryData {
                    geometry_type: 3, // Point
                    x1: point.position.x as f32,
                    y1: point.position.y as f32,
                    x2: 0.0,
                    y2: 0.0,
                    radius: 0.0,
                    bulge: 0.0,
                    param1: 0.0,
                    param2: 0.0,
                }
            ],
            Geometry::Polyline(polyline) => {
                let mut data = Vec::new();
                for vertex in &polyline.vertices {
                    data.push(GpuGeometryData {
                        geometry_type: 4, // Polyline vertex
                        x1: vertex.point.x as f32,
                        y1: vertex.point.y as f32,
                        x2: 0.0,
                        y2: 0.0,
                        radius: 0.0,
                        bulge: vertex.bulge as f32,
                        param1: if polyline.closed { 1.0 } else { 0.0 },
                        param2: 0.0,
                    });
                }
                data
            },
            Geometry::Arc(arc) => vec![
                GpuGeometryData {
                    geometry_type: 5, // Arc
                    x1: arc.center.x as f32,
                    y1: arc.center.y as f32,
                    x2: 0.0,
                    y2: 0.0,
                    radius: arc.radius as f32,
                    bulge: 0.0,
                    param1: arc.start_angle as f32,
                    param2: arc.end_angle as f32,
                }
            ],
            Geometry::Text(text) => vec![
                GpuGeometryData {
                    geometry_type: 6, // Text (represented as a point for GPU operations)
                    x1: text.position.x as f32,
                    y1: text.position.y as f32,
                    x2: 0.0,
                    y2: 0.0,
                    radius: text.height as f32,
                    bulge: 0.0,
                    param1: text.rotation as f32,
                    param2: 0.0,
                }
            ],
            Geometry::Dimension(_) => vec![], // 暂不支持GPU计算标注
        }
    }

    /// 将GPU数据转换回几何体
    fn gpu_data_to_geometry(&self, data: &GpuGeometryData) -> Option<Geometry> {
        match data.geometry_type {
            1 => Some(Geometry::Line(Line::new(
                Point2::new(data.x1 as f64, data.y1 as f64),
                Point2::new(data.x2 as f64, data.y2 as f64),
            ))),
            2 => Some(Geometry::Circle(Circle::new(
                Point2::new(data.x1 as f64, data.y1 as f64),
                data.radius as f64,
            ))),
            3 => Some(Geometry::Point(Point::from_point2(Point2::new(
                data.x1 as f64,
                data.y1 as f64,
            )))),
            4 => {
                // 简化处理：单个顶点转成点
                Some(Geometry::Point(Point::from_point2(Point2::new(
                    data.x1 as f64,
                    data.y1 as f64,
                ))))
            }
            5 => Some(Geometry::Arc(zcad_core::geometry::Arc::new(
                Point2::new(data.x1 as f64, data.y1 as f64),
                data.radius as f64,
                data.param1 as f64,
                data.param2 as f64,
            ))),
            _ => None,
        }
    }
}

/// 布尔运算类型
#[derive(Debug, Clone, Copy)]
pub enum BooleanOp {
    Union = 0,
    Intersection = 1,
    Difference = 2,
    Xor = 3,
}

/// GPU几何数据格式
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct GpuGeometryData {
    geometry_type: u32, // 0: empty, 1: line, 2: circle, 3: point, 4: polyline vertex, 5: arc
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
    radius: f32,
    bulge: f32,
    param1: f32, // 额外参数（圆弧角度、是否闭合等）
    param2: f32,
}

/// 布尔运算参数
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct BooleanParams {
    operation: u32,
    tolerance: f32,
    input1_count: u32,
    input2_count: u32,
}

/// 偏移运算参数
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct OffsetParams {
    distance: f32,
    tolerance: f32,
    input_count: u32,
}
