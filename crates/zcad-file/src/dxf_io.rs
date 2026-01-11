//! DXF文件导入/导出
//!
//! 支持AutoCAD DXF格式的读写，包括：
//! - 模型空间实体
//! - 图纸空间（Layout）
//! - 视口（Viewport）

use crate::document::Document;
use crate::dxf_raw::{DxfRawParser, DxfWriter, parse_layouts, parse_viewports};
use crate::error::FileError;
use std::path::Path;
use zcad_core::entity::Entity;
use zcad_core::geometry::{
    Arc, Circle, Ellipse, Geometry, Leader, Line, Polyline, PolylineVertex, 
    Spline, Text,
};
use zcad_core::layout::{Layout, PaperSize, PaperOrientation, Viewport, ViewportId, ViewportStatus};
use zcad_core::math::{Point2, Vector2};
use zcad_core::properties::{Color, Properties};

/// 从DXF文件导入
pub fn import(path: &Path) -> Result<Document, FileError> {
    let drawing = dxf::Drawing::load_file(path).map_err(|e| FileError::Dxf(e.to_string()))?;

    let mut document = Document::new();

    // 导入图层
    for layer in drawing.layers() {
        let color = aci_to_color(layer.color.index().unwrap_or(7) as u8);
        let new_layer = zcad_core::layer::Layer::new(&layer.name).with_color(color);
        document.layers.add_layer(new_layer);
    }

    // 导入模型空间实体
    for entity in drawing.entities() {
        if let Some(zcad_entity) = convert_dxf_entity(entity) {
            document.add_entity(zcad_entity);
        }
    }

    // 使用原始解析器导入完整的布局和视口信息
    if let Ok(mut raw_parser) = DxfRawParser::load(path) {
        import_layouts_full(&mut raw_parser, &drawing, &mut document);
    } else {
        // 回退到简化模式
        import_layouts_simplified(&drawing, &mut document);
    }

    // 设置文件路径
    document.set_file_path(path);

    Ok(document)
}

/// 完整的布局导入（使用原始解析器）
fn import_layouts_full(
    raw_parser: &mut DxfRawParser,
    drawing: &dxf::Drawing,
    document: &mut Document,
) {
    // 1. 解析 LAYOUT 对象
    let dxf_layouts = parse_layouts(raw_parser);
    
    // 2. 解析 VIEWPORT 实体
    let dxf_viewports = parse_viewports(raw_parser);
    
    // 3. 计算模型空间边界（用于设置默认视图）
    let model_bounds = calculate_model_bounds(drawing);
    
    // 4. 更新或创建布局
    for dxf_layout in &dxf_layouts {
        // 跳过模型空间
        if dxf_layout.is_model_space {
            continue;
        }
        
        // 确定图纸尺寸
        let paper_size = determine_paper_size(dxf_layout.paper_width, dxf_layout.paper_height);
        let orientation = if dxf_layout.paper_width > dxf_layout.paper_height {
            PaperOrientation::Landscape
        } else {
            PaperOrientation::Portrait
        };
        
        // 查找属于此布局的视口
        let layout_viewports: Vec<Viewport> = dxf_viewports
            .iter()
            .filter(|vp| vp.owner_handle == dxf_layout.block_record_handle || 
                         vp.owner_handle.is_empty()) // 如果没有 owner，假设属于第一个布局
            .enumerate()
            .map(|(idx, dxf_vp)| {
                convert_raw_viewport_to_zcad(dxf_vp, idx as u64 + 1, &model_bounds)
            })
            .collect();
        
        // 更新现有布局或添加新布局
        if let Some(existing) = document.layout_manager.get_layout_by_name(&dxf_layout.name) {
            let existing_id = existing.id;
            if let Some(layout) = document.layout_manager.get_layout_mut(existing_id) {
                layout.paper_size = paper_size;
                layout.orientation = orientation;
                layout.margins = (
                    dxf_layout.top_margin,
                    dxf_layout.right_margin,
                    dxf_layout.bottom_margin,
                    dxf_layout.left_margin,
                );
                if !layout_viewports.is_empty() {
                    layout.viewports = layout_viewports;
                }
            }
        } else {
            // 添加新布局
            let layout_id = document.layout_manager.add_layout(&dxf_layout.name);
            if let Some(layout) = document.layout_manager.get_layout_mut(layout_id) {
                layout.paper_size = paper_size;
                layout.orientation = orientation;
                layout.margins = (
                    dxf_layout.top_margin,
                    dxf_layout.right_margin,
                    dxf_layout.bottom_margin,
                    dxf_layout.left_margin,
                );
                if !layout_viewports.is_empty() {
                    layout.viewports = layout_viewports;
                }
            }
        }
    }
    
    // 如果没有解析到任何布局，使用简化模式
    if dxf_layouts.iter().filter(|l| !l.is_model_space).count() == 0 {
        import_layouts_simplified(drawing, document);
    }
}

/// 计算模型空间边界
fn calculate_model_bounds(drawing: &dxf::Drawing) -> Option<(f64, f64, f64, f64)> {
    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut max_x = f64::MIN;
    let mut max_y = f64::MIN;
    let mut has_entities = false;
    
    for entity in drawing.entities() {
        if let Some(bbox) = get_entity_bounds(entity) {
            min_x = min_x.min(bbox.0);
            min_y = min_y.min(bbox.1);
            max_x = max_x.max(bbox.2);
            max_y = max_y.max(bbox.3);
            has_entities = true;
        }
    }
    
    if has_entities {
        Some((min_x, min_y, max_x, max_y))
    } else {
        None
    }
}

/// 根据尺寸确定标准图纸大小
fn determine_paper_size(width: f64, height: f64) -> PaperSize {
    let (w, h) = if width > height { (width, height) } else { (height, width) };
    
    // 检查常见纸张尺寸（允许 5mm 误差）
    let tolerance = 5.0;
    
    if (w - 1189.0).abs() < tolerance && (h - 841.0).abs() < tolerance {
        PaperSize::A0
    } else if (w - 841.0).abs() < tolerance && (h - 594.0).abs() < tolerance {
        PaperSize::A1
    } else if (w - 594.0).abs() < tolerance && (h - 420.0).abs() < tolerance {
        PaperSize::A2
    } else if (w - 420.0).abs() < tolerance && (h - 297.0).abs() < tolerance {
        PaperSize::A3
    } else if (w - 297.0).abs() < tolerance && (h - 210.0).abs() < tolerance {
        PaperSize::A4
    } else {
        PaperSize::Custom { width, height }
    }
}

/// 将原始 DXF 视口转换为 ZCAD 视口
fn convert_raw_viewport_to_zcad(
    dxf_vp: &crate::dxf_raw::DxfViewport,
    id: u64,
    model_bounds: &Option<(f64, f64, f64, f64)>,
) -> Viewport {
    // 计算视口位置（从中心转换为左下角）
    let position = Point2::new(
        dxf_vp.center.x - dxf_vp.width / 2.0,
        dxf_vp.center.y - dxf_vp.height / 2.0,
    );
    
    let mut viewport = Viewport::new(ViewportId::new(id), position, dxf_vp.width, dxf_vp.height);
    
    // 设置视图中心
    viewport.view_center = dxf_vp.view_center;
    
    // 计算比例
    if dxf_vp.view_height > 0.0 && dxf_vp.height > 0.0 {
        viewport.scale = dxf_vp.view_height / dxf_vp.height;
    }
    
    // 设置状态
    viewport.status = if dxf_vp.status > 0 {
        ViewportStatus::Inactive
    } else {
        ViewportStatus::Hidden
    };
    
    viewport
}

/// 创建默认视口
fn create_default_viewport(layout: &Layout, model_bounds: &Option<(f64, f64, f64, f64)>) -> Viewport {
    let (paper_w, paper_h) = layout.paper_size.dimensions_mm();
    let margins = layout.margins;
    
    // 计算可用区域
    let usable_width = paper_w - margins.1 - margins.3;
    let usable_height = paper_h - margins.0 - margins.2;
    
    // 视口位置（留边距）
    let position = Point2::new(margins.3, margins.2);
    
    let mut viewport = Viewport::new(
        ViewportId::new(1),
        position,
        usable_width,
        usable_height,
    );
    
    // 如果有模型边界，设置视图
    if let Some((min_x, min_y, max_x, max_y)) = model_bounds {
        let model_width = max_x - min_x;
        let model_height = max_y - min_y;
        
        viewport.view_center = Point2::new(
            (min_x + max_x) / 2.0,
            (min_y + max_y) / 2.0,
        );
        
        // 计算适合的比例
        let scale_x = model_width / usable_width;
        let scale_y = model_height / usable_height;
        viewport.scale = scale_x.max(scale_y) * 1.1; // 10% 边距
    }
    
    viewport.status = ViewportStatus::Active;
    viewport
}

/// 简化的布局导入
/// 
/// 由于 dxf crate 对 VIEWPORT 实体的支持有限，
/// 这里使用简化的方式：基于模型空间范围创建默认视口
fn import_layouts_simplified(drawing: &dxf::Drawing, document: &mut Document) {
    // 计算模型空间的边界
    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut max_x = f64::MIN;
    let mut max_y = f64::MIN;
    let mut has_entities = false;
    
    for entity in drawing.entities() {
        if let Some(bbox) = get_entity_bounds(entity) {
            min_x = min_x.min(bbox.0);
            min_y = min_y.min(bbox.1);
            max_x = max_x.max(bbox.2);
            max_y = max_y.max(bbox.3);
            has_entities = true;
        }
    }
    
    // 如果有实体，更新默认视口的视图范围
    if has_entities {
        if let Some(layout) = document.layout_manager.get_layout_by_name("Layout1") {
            let layout_id = layout.id;
            if let Some(layout) = document.layout_manager.get_layout_mut(layout_id) {
                // 更新第一个视口的视图中心和比例
                if let Some(viewport) = layout.viewports.first_mut() {
                    let model_width = max_x - min_x;
                    let model_height = max_y - min_y;
                    
                    // 设置视图中心
                    viewport.view_center = Point2::new(
                        (min_x + max_x) / 2.0,
                        (min_y + max_y) / 2.0,
                    );
                    
                    // 计算合适的比例
                    let scale_x = model_width / viewport.width;
                    let scale_y = model_height / viewport.height;
                    viewport.scale = scale_x.max(scale_y) * 1.1; // 留 10% 边距
                }
            }
        }
    }
}

/// 获取实体的边界范围
fn get_entity_bounds(entity: &dxf::entities::Entity) -> Option<(f64, f64, f64, f64)> {
    match &entity.specific {
        dxf::entities::EntityType::Line(line) => {
            let min_x = line.p1.x.min(line.p2.x);
            let min_y = line.p1.y.min(line.p2.y);
            let max_x = line.p1.x.max(line.p2.x);
            let max_y = line.p1.y.max(line.p2.y);
            Some((min_x, min_y, max_x, max_y))
        }
        dxf::entities::EntityType::Circle(circle) => {
            let r = circle.radius;
            Some((
                circle.center.x - r,
                circle.center.y - r,
                circle.center.x + r,
                circle.center.y + r,
            ))
        }
        dxf::entities::EntityType::Arc(arc) => {
            let r = arc.radius;
            Some((
                arc.center.x - r,
                arc.center.y - r,
                arc.center.x + r,
                arc.center.y + r,
            ))
        }
        dxf::entities::EntityType::LwPolyline(lwpoly) => {
            if lwpoly.vertices.is_empty() {
                return None;
            }
            let min_x = lwpoly.vertices.iter().map(|v| v.x).fold(f64::MAX, f64::min);
            let min_y = lwpoly.vertices.iter().map(|v| v.y).fold(f64::MAX, f64::min);
            let max_x = lwpoly.vertices.iter().map(|v| v.x).fold(f64::MIN, f64::max);
            let max_y = lwpoly.vertices.iter().map(|v| v.y).fold(f64::MIN, f64::max);
            Some((min_x, min_y, max_x, max_y))
        }
        _ => None,
    }
}

/// 将DXF实体转换为ZCAD实体
fn convert_dxf_entity(entity: &dxf::entities::Entity) -> Option<Entity> {
    let geometry = match &entity.specific {
        dxf::entities::EntityType::Line(line) => {
            let start = Point2::new(line.p1.x, line.p1.y);
            let end = Point2::new(line.p2.x, line.p2.y);
            Geometry::Line(Line::new(start, end))
        }

        dxf::entities::EntityType::Circle(circle) => {
            let center = Point2::new(circle.center.x, circle.center.y);
            Geometry::Circle(Circle::new(center, circle.radius))
        }

        dxf::entities::EntityType::Arc(arc) => {
            let center = Point2::new(arc.center.x, arc.center.y);
            let start_angle = arc.start_angle.to_radians();
            let end_angle = arc.end_angle.to_radians();
            Geometry::Arc(Arc::new(center, arc.radius, start_angle, end_angle))
        }

        dxf::entities::EntityType::LwPolyline(lwpoly) => {
            let vertices: Vec<PolylineVertex> = lwpoly
                .vertices
                .iter()
                .map(|v| PolylineVertex::with_bulge(Point2::new(v.x, v.y), v.bulge))
                .collect();

            Geometry::Polyline(Polyline::new(vertices, lwpoly.is_closed()))
        }

        dxf::entities::EntityType::Polyline(poly) => {
            let vertices: Vec<PolylineVertex> = poly
                .vertices()
                .map(|v| {
                    PolylineVertex::with_bulge(Point2::new(v.location.x, v.location.y), v.bulge)
                })
                .collect();

            Geometry::Polyline(Polyline::new(vertices, poly.is_closed()))
        }

        dxf::entities::EntityType::Text(text) => {
            let position = Point2::new(text.location.x, text.location.y);
            let height = text.text_height;
            let rotation = text.rotation.to_radians();
            let mut zcad_text = Text::new(position, text.value.clone(), height);
            zcad_text.rotation = rotation;
            Geometry::Text(zcad_text)
        }

        dxf::entities::EntityType::MText(mtext) => {
            let position = Point2::new(mtext.insertion_point.x, mtext.insertion_point.y);
            let height = mtext.initial_text_height;
            let rotation = mtext.rotation_angle.to_radians();
            // MText 内容可能包含格式代码，这里简化处理
            let content = mtext.text.replace("\\P", "\n"); // 简单的换行处理
            let mut zcad_text = Text::new(position, content, height);
            zcad_text.rotation = rotation;
            Geometry::Text(zcad_text)
        }

        dxf::entities::EntityType::ModelPoint(point) => {
            let position = Point2::new(point.location.x, point.location.y);
            Geometry::Point(zcad_core::geometry::Point::from_point2(position))
        }

        dxf::entities::EntityType::Ellipse(ellipse) => {
            let center = Point2::new(ellipse.center.x, ellipse.center.y);
            let major_axis = Vector2::new(ellipse.major_axis.x, ellipse.major_axis.y);
            let ratio = ellipse.minor_axis_ratio;
            let start_param = ellipse.start_parameter;
            let end_param = ellipse.end_parameter;
            Geometry::Ellipse(Ellipse::arc(center, major_axis, ratio, start_param, end_param))
        }

        dxf::entities::EntityType::Spline(spline) => {
            let degree = spline.degree_of_curve as u8;
            let control_points: Vec<Point2> = spline
                .control_points
                .iter()
                .map(|p| Point2::new(p.x, p.y))
                .collect();
            let knots: Vec<f64> = spline.knot_values.clone();
            let fit_points: Vec<Point2> = spline
                .fit_points
                .iter()
                .map(|p| Point2::new(p.x, p.y))
                .collect();
            let closed = spline.is_closed();
            
            let mut zcad_spline = Spline::new(degree);
            zcad_spline.control_points = control_points;
            zcad_spline.knots = knots;
            zcad_spline.fit_points = fit_points;
            zcad_spline.closed = closed;
            
            Geometry::Spline(zcad_spline)
        }

        dxf::entities::EntityType::Leader(leader) => {
            let vertices: Vec<Point2> = leader
                .vertices
                .iter()
                .map(|p| Point2::new(p.x, p.y))
                .collect();
            
            let zcad_leader = Leader::new(vertices);
            
            Geometry::Leader(zcad_leader)
        }

        dxf::entities::EntityType::RotatedDimension(dim) => {
            // RotatedDimension (AcDbRotatedDimension/AcDbAlignedDimension)
            // definition_point_2 (13) = Extension line 1 origin (Start point)
            // definition_point_3 (14) = Extension line 2 origin (End point)
            // definition_point_1 (10 in base) = Dimension line definition point
            
            let p1 = Point2::new(dim.definition_point_2.x, dim.definition_point_2.y);
            let p2 = Point2::new(dim.definition_point_3.x, dim.definition_point_3.y);
            let location = Point2::new(dim.dimension_base.definition_point_1.x, dim.dimension_base.definition_point_1.y);
            
            let mut zcad_dim = zcad_core::geometry::Dimension::new(p1, p2, location);
            
            match dim.dimension_base.dimension_type {
                dxf::enums::DimensionType::Aligned => {
                    zcad_dim.dim_type = zcad_core::geometry::DimensionType::Aligned;
                }
                _ => {
                    // Default to Linear for RotatedHorizontalOrVertical or others
                    zcad_dim.dim_type = zcad_core::geometry::DimensionType::Linear;
                }
            }
            
            if !dim.dimension_base.text.is_empty() && dim.dimension_base.text != "<>" {
                zcad_dim.text_override = Some(dim.dimension_base.text.clone());
            }
            
            // 读取文本位置 (11)
            let text_pos = Point2::new(dim.dimension_base.text_mid_point.x, dim.dimension_base.text_mid_point.y);
            // 检查是否是有效位置 (0,0可能是未设置)
            if text_pos.x.abs() > 1e-6 || text_pos.y.abs() > 1e-6 {
                zcad_dim.text_position = Some(text_pos);
            }
            
            Geometry::Dimension(zcad_dim)
        }

        dxf::entities::EntityType::RadialDimension(dim) => {
            // 10: Center (definition_point_1 in base)
            // 15: Point on curve (definition_point_2)
            let center = Point2::new(dim.dimension_base.definition_point_1.x, dim.dimension_base.definition_point_1.y);
            let point_on_curve = Point2::new(dim.definition_point_2.x, dim.definition_point_2.y);
            let text_pos = Point2::new(dim.dimension_base.text_mid_point.x, dim.dimension_base.text_mid_point.y);

            let mut zcad_dim = zcad_core::geometry::Dimension::new(center, point_on_curve, text_pos);
            zcad_dim.dim_type = zcad_core::geometry::DimensionType::Radius;

            if !dim.dimension_base.text.is_empty() && dim.dimension_base.text != "<>" {
                zcad_dim.text_override = Some(dim.dimension_base.text.clone());
            }
            
            // 半径/直径标注的 text_pos 总是有效的
            zcad_dim.text_position = Some(text_pos);

            Geometry::Dimension(zcad_dim)
        }

        dxf::entities::EntityType::DiameterDimension(dim) => {
            // 15: Point on curve (definition_point_2)
            // 10: Opposite point on curve (definition_point_1 in base)
            let p1 = Point2::new(dim.definition_point_2.x, dim.definition_point_2.y);
            let p2 = Point2::new(dim.dimension_base.definition_point_1.x, dim.dimension_base.definition_point_1.y);
            
            // Calculate center as midpoint
            let center = p1 + (p2 - p1) * 0.5;
            let text_pos = Point2::new(dim.dimension_base.text_mid_point.x, dim.dimension_base.text_mid_point.y);

            let mut zcad_dim = zcad_core::geometry::Dimension::new(center, p1, text_pos);
            zcad_dim.dim_type = zcad_core::geometry::DimensionType::Diameter;

            if !dim.dimension_base.text.is_empty() && dim.dimension_base.text != "<>" {
                zcad_dim.text_override = Some(dim.dimension_base.text.clone());
            }
            
            zcad_dim.text_position = Some(text_pos);

            Geometry::Dimension(zcad_dim)
        }

        // TODO: 支持更多实体类型
        _ => return None,
    };

    // 提取属性
    let color = entity
        .common
        .color
        .index()
        .map(|i| aci_to_color(i as u8))
        .unwrap_or(Color::BY_LAYER);

    let properties = Properties::with_color(color);

    Some(Entity::new(geometry).with_properties(properties))
}

/// 导出到DXF文件
pub fn export(document: &Document, path: &Path) -> Result<(), FileError> {
    let mut drawing = dxf::Drawing::new();

    // 导出图层
    for layer in document.layers.all_layers() {
        let mut dxf_layer = dxf::tables::Layer::default();
        dxf_layer.name = layer.name.clone();
        dxf_layer.color = dxf::Color::from_index(color_to_aci(&layer.color));
        drawing.add_layer(dxf_layer);
    }

    // 导出模型空间实体
    for entity in document.all_entities() {
        if let Some(dxf_entity) = convert_to_dxf_entity(entity) {
            drawing.add_entity(dxf_entity);
        }
    }

    // 导出图纸空间实体（如果有）
    export_paper_space_entities(document, &mut drawing);

    drawing
        .save_file(path)
        .map_err(|e| FileError::Dxf(e.to_string()))?;

    Ok(())
}

/// 导出图纸空间实体和视口
fn export_paper_space_entities(document: &Document, drawing: &mut dxf::Drawing) {
    // 遍历所有布局
    for layout in document.layout_manager.layouts() {
        // 导出图纸空间实体
        for entity in &layout.paper_space_entities {
            if let Some(dxf_entity) = convert_to_dxf_entity(entity) {
                drawing.add_entity(dxf_entity);
            }
        }
    }
}

/// 使用原始写入器导出完整的 DXF（包括布局和视口）
/// 
/// 此函数生成包含完整 Layout/Viewport 信息的 DXF 文件
#[allow(dead_code)]
pub fn export_full(document: &Document, path: &Path) -> Result<(), FileError> {
    let mut writer = DxfWriter::new();
    
    // 1. 写入 HEADER 段
    write_header_section(&mut writer);
    
    // 2. 写入 TABLES 段
    write_tables_section(&mut writer, document);
    
    // 3. 写入 BLOCKS 段
    write_blocks_section(&mut writer, document);
    
    // 4. 写入 ENTITIES 段
    write_entities_section(&mut writer, document);
    
    // 5. 写入 OBJECTS 段
    write_objects_section(&mut writer, document);
    
    // 保存文件
    writer.save_to_file(path)
}

/// 写入 HEADER 段
fn write_header_section(writer: &mut DxfWriter) {
    writer.begin_section("HEADER");
    
    // AutoCAD 版本
    writer.write_pair(9, "$ACADVER");
    writer.write_pair(1, "AC1027"); // AutoCAD 2013 格式
    
    // 默认图层
    writer.write_pair(9, "$CLAYER");
    writer.write_pair(8, "0");
    
    writer.end_section();
}

/// 写入 TABLES 段
fn write_tables_section(writer: &mut DxfWriter, document: &Document) {
    writer.begin_section("TABLES");
    
    // VPORT 表
    writer.write_pair(0, "TABLE");
    writer.write_pair(2, "VPORT");
    writer.write_handle_only();
    writer.write_pair(70, 1);
    writer.write_pair(0, "ENDTAB");
    
    // LTYPE 表
    writer.write_pair(0, "TABLE");
    writer.write_pair(2, "LTYPE");
    writer.write_handle_only();
    writer.write_pair(70, 1);
    
    // CONTINUOUS 线型
    writer.write_pair(0, "LTYPE");
    writer.write_handle_only();
    writer.write_pair(2, "CONTINUOUS");
    writer.write_pair(70, 0);
    writer.write_pair(3, "Solid line");
    writer.write_pair(72, 65);
    writer.write_pair(73, 0);
    writer.write_pair(40, 0.0);
    
    writer.write_pair(0, "ENDTAB");
    
    // LAYER 表
    writer.write_pair(0, "TABLE");
    writer.write_pair(2, "LAYER");
    writer.write_handle_only();
    writer.write_pair(70, document.layers.all_layers().len() as i32);
    
    for layer in document.layers.all_layers() {
        writer.write_pair(0, "LAYER");
        writer.write_handle_only();
        writer.write_pair(2, &layer.name);
        writer.write_pair(70, if layer.visible { 0 } else { 1 });
        writer.write_pair(62, color_to_aci(&layer.color) as i32);
        writer.write_pair(6, "CONTINUOUS");
    }
    
    writer.write_pair(0, "ENDTAB");
    
    // BLOCK_RECORD 表
    let model_handle = writer.new_handle();
    let paper_handle = writer.new_handle();
    
    writer.write_pair(0, "TABLE");
    writer.write_pair(2, "BLOCK_RECORD");
    writer.write_handle_only();
    writer.write_pair(70, 2 + document.layout_manager.layouts().len() as i32);
    
    // *Model_Space
    writer.write_pair(0, "BLOCK_RECORD");
    writer.write_pair(5, &model_handle);
    writer.write_pair(2, "*Model_Space");
    
    // *Paper_Space
    writer.write_pair(0, "BLOCK_RECORD");
    writer.write_pair(5, &paper_handle);
    writer.write_pair(2, "*Paper_Space");
    
    writer.write_pair(0, "ENDTAB");
    
    writer.end_section();
}

/// 写入 BLOCKS 段
fn write_blocks_section(writer: &mut DxfWriter, _document: &Document) {
    writer.begin_section("BLOCKS");
    
    // *Model_Space 块
    writer.write_pair(0, "BLOCK");
    writer.write_handle_only();
    writer.write_pair(8, "0");
    writer.write_pair(2, "*Model_Space");
    writer.write_pair(70, 0);
    writer.write_pair(10, 0.0);
    writer.write_pair(20, 0.0);
    writer.write_pair(30, 0.0);
    writer.write_pair(0, "ENDBLK");
    writer.write_handle_only();
    writer.write_pair(8, "0");
    
    // *Paper_Space 块
    writer.write_pair(0, "BLOCK");
    writer.write_handle_only();
    writer.write_pair(8, "0");
    writer.write_pair(2, "*Paper_Space");
    writer.write_pair(70, 0);
    writer.write_pair(10, 0.0);
    writer.write_pair(20, 0.0);
    writer.write_pair(30, 0.0);
    writer.write_pair(0, "ENDBLK");
    writer.write_handle_only();
    writer.write_pair(8, "0");
    
    writer.end_section();
}

/// 写入 ENTITIES 段
fn write_entities_section(writer: &mut DxfWriter, document: &Document) {
    writer.begin_section("ENTITIES");
    
    // 导出模型空间实体
    for entity in document.all_entities() {
        write_entity(writer, entity, false);
    }
    
    // 导出视口和图纸空间实体
    for layout in document.layout_manager.layouts() {
        // 导出视口
        for viewport in &layout.viewports {
            write_viewport(writer, viewport);
        }
        
        // 导出图纸空间实体
        for entity in &layout.paper_space_entities {
            write_entity(writer, entity, true);
        }
    }
    
    writer.end_section();
}

/// 写入单个实体
fn write_entity(writer: &mut DxfWriter, entity: &Entity, is_paper_space: bool) {
    match &entity.geometry {
        Geometry::Line(line) => {
            writer.write_pair(0, "LINE");
            writer.write_handle_only();
            if is_paper_space {
                writer.write_pair(67, 1);
            }
            writer.write_pair(8, "0");
            writer.write_pair(10, line.start.x);
            writer.write_pair(20, line.start.y);
            writer.write_pair(30, 0.0);
            writer.write_pair(11, line.end.x);
            writer.write_pair(21, line.end.y);
            writer.write_pair(31, 0.0);
        }
        Geometry::Circle(circle) => {
            writer.write_pair(0, "CIRCLE");
            writer.write_handle_only();
            if is_paper_space {
                writer.write_pair(67, 1);
            }
            writer.write_pair(8, "0");
            writer.write_pair(10, circle.center.x);
            writer.write_pair(20, circle.center.y);
            writer.write_pair(30, 0.0);
            writer.write_pair(40, circle.radius);
        }
        Geometry::Arc(arc) => {
            writer.write_pair(0, "ARC");
            writer.write_handle_only();
            if is_paper_space {
                writer.write_pair(67, 1);
            }
            writer.write_pair(8, "0");
            writer.write_pair(10, arc.center.x);
            writer.write_pair(20, arc.center.y);
            writer.write_pair(30, 0.0);
            writer.write_pair(40, arc.radius);
            writer.write_pair(50, arc.start_angle.to_degrees());
            writer.write_pair(51, arc.end_angle.to_degrees());
        }
        Geometry::Polyline(polyline) => {
            writer.write_pair(0, "LWPOLYLINE");
            writer.write_handle_only();
            if is_paper_space {
                writer.write_pair(67, 1);
            }
            writer.write_pair(8, "0");
            writer.write_pair(90, polyline.vertices.len() as i32);
            writer.write_pair(70, if polyline.closed { 1 } else { 0 });
            
            for vertex in &polyline.vertices {
                writer.write_pair(10, vertex.point.x);
                writer.write_pair(20, vertex.point.y);
                writer.write_pair(42, vertex.bulge);
            }
        }
        Geometry::Text(text) => {
            writer.write_pair(0, "TEXT");
            writer.write_handle_only();
            if is_paper_space {
                writer.write_pair(67, 1);
            }
            writer.write_pair(8, "0");
            writer.write_pair(10, text.position.x);
            writer.write_pair(20, text.position.y);
            writer.write_pair(30, 0.0);
            writer.write_pair(40, text.height);
            writer.write_pair(1, &text.content);
            writer.write_pair(50, text.rotation.to_degrees());
        }
        _ => {
            // 其他几何类型暂不支持
        }
    }
}

/// 写入视口
fn write_viewport(writer: &mut DxfWriter, viewport: &Viewport) {
    writer.write_pair(0, "VIEWPORT");
    writer.write_handle_only();
    writer.write_pair(67, 1); // 图纸空间标记
    writer.write_pair(8, "0");
    writer.write_pair(100, "AcDbEntity");
    writer.write_pair(100, "AcDbViewport");
    
    // 视口中心（图纸空间）
    let center_x = viewport.position.x + viewport.width / 2.0;
    let center_y = viewport.position.y + viewport.height / 2.0;
    writer.write_pair(10, center_x);
    writer.write_pair(20, center_y);
    writer.write_pair(30, 0.0);
    
    // 视口尺寸
    writer.write_pair(40, viewport.width);
    writer.write_pair(41, viewport.height);
    
    // 视口 ID
    writer.write_pair(69, viewport.id.0 as i32 + 1);
    
    // 视图中心（模型空间）
    writer.write_pair(12, viewport.view_center.x);
    writer.write_pair(22, viewport.view_center.y);
    
    // 视图高度
    writer.write_pair(45, viewport.height * viewport.scale);
    
    // 视口状态
    let status = match viewport.status {
        ViewportStatus::Active => 1,
        ViewportStatus::Inactive => 1,
        ViewportStatus::Locked => 1,
        ViewportStatus::Hidden => 0,
    };
    writer.write_pair(68, status);
    
    // 标准标志
    writer.write_pair(90, 32864);
}

/// 写入 OBJECTS 段
fn write_objects_section(writer: &mut DxfWriter, document: &Document) {
    writer.begin_section("OBJECTS");
    
    // 写入字典
    let dict_handle = writer.new_handle();
    writer.write_pair(0, "DICTIONARY");
    writer.write_pair(5, &dict_handle);
    writer.write_pair(100, "AcDbDictionary");
    
    // 布局字典
    let layout_dict_handle = writer.new_handle();
    writer.write_pair(3, "ACAD_LAYOUT");
    writer.write_pair(350, &layout_dict_handle);
    
    // 布局字典内容
    writer.write_pair(0, "DICTIONARY");
    writer.write_pair(5, &layout_dict_handle);
    writer.write_pair(100, "AcDbDictionary");
    
    // 写入每个布局
    for layout in document.layout_manager.layouts() {
        let layout_obj_handle = writer.new_handle();
        writer.write_pair(3, &layout.name);
        writer.write_pair(350, &layout_obj_handle);
        
        // 写入 LAYOUT 对象
        write_layout_object(writer, layout, &layout_obj_handle, &layout_dict_handle);
    }
    
    writer.end_section();
}

/// 写入 LAYOUT 对象
fn write_layout_object(
    writer: &mut DxfWriter,
    layout: &Layout,
    handle: &str,
    owner_handle: &str,
) {
    let (width, height) = layout.paper_size.dimensions_mm();
    
    writer.write_pair(0, "LAYOUT");
    writer.write_pair(5, handle);
    writer.write_pair(330, owner_handle);
    writer.write_pair(100, "AcDbPlotSettings");
    
    // 图纸设置
    writer.write_pair(1, ""); // 页面设置名
    writer.write_pair(2, "none_device"); // 打印机
    writer.write_pair(4, ""); // 图纸尺寸名
    
    // 边距
    writer.write_pair(40, layout.margins.3); // 左
    writer.write_pair(41, layout.margins.2); // 下
    writer.write_pair(42, layout.margins.1); // 右
    writer.write_pair(43, layout.margins.0); // 上
    
    // 图纸尺寸
    writer.write_pair(44, width);
    writer.write_pair(45, height);
    
    writer.write_pair(100, "AcDbLayout");
    
    // 布局名称
    writer.write_pair(1, &layout.name);
    
    // 布局标志
    writer.write_pair(70, 1);
    
    // 布局顺序
    writer.write_pair(71, layout.id.0 as i32);
}

/// 将ZCAD实体转换为DXF实体
fn convert_to_dxf_entity(entity: &Entity) -> Option<dxf::entities::Entity> {
    let specific = match &entity.geometry {
        Geometry::Line(line) => {
            let mut dxf_line = dxf::entities::Line::default();
            dxf_line.p1 = dxf::Point::new(line.start.x, line.start.y, 0.0);
            dxf_line.p2 = dxf::Point::new(line.end.x, line.end.y, 0.0);
            dxf::entities::EntityType::Line(dxf_line)
        }

        Geometry::Circle(circle) => {
            let mut dxf_circle = dxf::entities::Circle::default();
            dxf_circle.center = dxf::Point::new(circle.center.x, circle.center.y, 0.0);
            dxf_circle.radius = circle.radius;
            dxf::entities::EntityType::Circle(dxf_circle)
        }

        Geometry::Arc(arc) => {
            let mut dxf_arc = dxf::entities::Arc::default();
            dxf_arc.center = dxf::Point::new(arc.center.x, arc.center.y, 0.0);
            dxf_arc.radius = arc.radius;
            dxf_arc.start_angle = arc.start_angle.to_degrees();
            dxf_arc.end_angle = arc.end_angle.to_degrees();
            dxf::entities::EntityType::Arc(dxf_arc)
        }

        Geometry::Polyline(polyline) => {
            let mut lwpoly = dxf::entities::LwPolyline::default();
            lwpoly.set_is_closed(polyline.closed);
            lwpoly.vertices = polyline
                .vertices
                .iter()
                .map(|v| {
                    let mut vertex = dxf::LwPolylineVertex::default();
                    vertex.x = v.point.x;
                    vertex.y = v.point.y;
                    vertex.bulge = v.bulge;
                    vertex
                })
                .collect();
            dxf::entities::EntityType::LwPolyline(lwpoly)
        }

        Geometry::Point(point) => {
            let mut dxf_point = dxf::entities::ModelPoint::default();
            dxf_point.location = dxf::Point::new(point.position.x, point.position.y, 0.0);
            dxf::entities::EntityType::ModelPoint(dxf_point)
        }

        Geometry::Text(text) => {
            let mut dxf_text = dxf::entities::Text::default();
            dxf_text.location = dxf::Point::new(text.position.x, text.position.y, 0.0);
            dxf_text.text_height = text.height;
            dxf_text.value = text.content.clone();
            dxf_text.rotation = text.rotation.to_degrees();
            dxf::entities::EntityType::Text(dxf_text)
        }
        Geometry::Dimension(dim) => {
            let mut base = dxf::entities::DimensionBase::default();
            
            // 设置文本位置 (11)
            // base.text_mid_point = dxf::Point::new(dim.line_location.x, dim.line_location.y, 0.0);
            
            // 设置文本内容
            if let Some(text) = &dim.text_override {
                base.text = text.clone();
            } else {
                // 空字符串表示使用测量值
                base.text = String::new();
            }

            // 设置文本位置 (11) - 如果有自定义位置，使用它；否则使用默认计算位置
            let text_pos = dim.get_text_position();
            base.text_mid_point = dxf::Point::new(text_pos.x, text_pos.y, 0.0);
            
            match dim.dim_type {
                zcad_core::geometry::DimensionType::Radius => {
                    base.dimension_type = dxf::enums::DimensionType::Radius;
                    
                    // 10: Center (p1)
                    base.definition_point_1 = dxf::Point::new(dim.definition_point1.x, dim.definition_point1.y, 0.0);
                    
                    let mut dxf_dim = dxf::entities::RadialDimension::default();
                    dxf_dim.dimension_base = base;
                    
                    // 15: Point on curve (p2)
                    dxf_dim.definition_point_2 = dxf::Point::new(dim.definition_point2.x, dim.definition_point2.y, 0.0);
                    
                    dxf::entities::EntityType::RadialDimension(dxf_dim)
                },
                zcad_core::geometry::DimensionType::Diameter => {
                    base.dimension_type = dxf::enums::DimensionType::Diameter;
                    
                    // 10: Opposite point
                    let opposite = dim.definition_point1 + (dim.definition_point1 - dim.definition_point2);
                    base.definition_point_1 = dxf::Point::new(opposite.x, opposite.y, 0.0);
                    
                    let mut dxf_dim = dxf::entities::DiameterDimension::default();
                    dxf_dim.dimension_base = base;
                    
                    // 15: Point on curve (p2)
                    dxf_dim.definition_point_2 = dxf::Point::new(dim.definition_point2.x, dim.definition_point2.y, 0.0);
                    
                    dxf::entities::EntityType::DiameterDimension(dxf_dim)
                },
                _ => {
                    // definition_point_1 (10) = Dimension line definition point
                    base.definition_point_1 = dxf::Point::new(dim.line_location.x, dim.line_location.y, 0.0);
                    
                    let mut dxf_dim = dxf::entities::RotatedDimension::default();
                    
                    if dim.dim_type == zcad_core::geometry::DimensionType::Aligned {
                         base.dimension_type = dxf::enums::DimensionType::Aligned;
                    } else {
                         base.dimension_type = dxf::enums::DimensionType::RotatedHorizontalOrVertical;
                    }

                    dxf_dim.dimension_base = base;
                    
                    // definition_point_2 (13) = Extension line 1 origin (Start point)
                    dxf_dim.definition_point_2 = dxf::Point::new(dim.definition_point1.x, dim.definition_point1.y, 0.0);
                    // definition_point_3 (14) = Extension line 2 origin (End point)
                    dxf_dim.definition_point_3 = dxf::Point::new(dim.definition_point2.x, dim.definition_point2.y, 0.0);
                    
                    // insertion_point (12)
                    dxf_dim.insertion_point = dxf::Point::new(dim.line_location.x, dim.line_location.y, 0.0);
                    
                    dxf::entities::EntityType::RotatedDimension(dxf_dim)
                }
            }
        }

        Geometry::Ellipse(ellipse) => {
            let mut dxf_ellipse = dxf::entities::Ellipse::default();
            dxf_ellipse.center = dxf::Point::new(ellipse.center.x, ellipse.center.y, 0.0);
            dxf_ellipse.major_axis = dxf::Vector::new(ellipse.major_axis.x, ellipse.major_axis.y, 0.0);
            dxf_ellipse.minor_axis_ratio = ellipse.ratio;
            dxf_ellipse.start_parameter = ellipse.start_param;
            dxf_ellipse.end_parameter = ellipse.end_param;
            dxf::entities::EntityType::Ellipse(dxf_ellipse)
        }

        Geometry::Spline(spline) => {
            let mut dxf_spline = dxf::entities::Spline::default();
            dxf_spline.degree_of_curve = spline.degree as i32;
            dxf_spline.control_points = spline
                .control_points
                .iter()
                .map(|p| dxf::Point::new(p.x, p.y, 0.0))
                .collect();
            dxf_spline.knot_values = spline.knots.clone();
            dxf_spline.fit_points = spline
                .fit_points
                .iter()
                .map(|p| dxf::Point::new(p.x, p.y, 0.0))
                .collect();
            if spline.closed {
                dxf_spline.flags |= 1; // Closed spline
            }
            dxf::entities::EntityType::Spline(dxf_spline)
        }

        Geometry::Hatch(_hatch) => {
            // TODO: 实现完整的 Hatch 导出
            // 当前跳过填充，因为 DXF Hatch 结构复杂
            return None;
        }

        Geometry::Leader(leader) => {
            let mut dxf_leader = dxf::entities::Leader::default();
            dxf_leader.vertices = leader
                .vertices
                .iter()
                .map(|p| dxf::Point::new(p.x, p.y, 0.0))
                .collect();
            dxf::entities::EntityType::Leader(dxf_leader)
        }
    };

    let mut dxf_entity = dxf::entities::Entity::new(specific);

    // 设置颜色
    if !entity.properties.color.is_by_layer() {
        dxf_entity.common.color =
            dxf::Color::from_index(color_to_aci(&entity.properties.color));
    }

    Some(dxf_entity)
}

/// AutoCAD颜色索引(ACI)转ZCAD颜色
fn aci_to_color(aci: u8) -> Color {
    match aci {
        1 => Color::RED,
        2 => Color::YELLOW,
        3 => Color::GREEN,
        4 => Color::CYAN,
        5 => Color::BLUE,
        6 => Color::MAGENTA,
        7 => Color::WHITE,
        8 => Color::GRAY,
        _ => Color::WHITE,
    }
}

/// ZCAD颜色转AutoCAD颜色索引
fn color_to_aci(color: &Color) -> u8 {
    if color.is_by_layer() || color.is_by_block() {
        return 7; // 默认白色（ByLayer/ByBlock在其他地方处理）
    }

    // 简单的颜色匹配
    match (color.r, color.g, color.b) {
        (255, 0, 0) => 1,
        (255, 255, 0) => 2,
        (0, 255, 0) => 3,
        (0, 255, 255) => 4,
        (0, 0, 255) => 5,
        (255, 0, 255) => 6,
        (255, 255, 255) => 7,
        (128, 128, 128) => 8,
        _ => 7, // 默认白色
    }
}

