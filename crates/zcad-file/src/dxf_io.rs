//! DXF文件导入/导出
//!
//! 支持AutoCAD DXF格式的读写。

use crate::document::Document;
use crate::error::FileError;
use std::path::Path;
use zcad_core::entity::Entity;
use zcad_core::geometry::{Arc, Circle, Geometry, Line, Polyline, PolylineVertex, Text};
use zcad_core::math::Point2;
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

    // 导入实体
    for entity in drawing.entities() {
        if let Some(zcad_entity) = convert_dxf_entity(entity) {
            document.add_entity(zcad_entity);
        }
    }

    // 设置文件路径
    document.set_file_path(path);

    Ok(document)
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

    // 导出实体
    for entity in document.all_entities() {
        if let Some(dxf_entity) = convert_to_dxf_entity(entity) {
            drawing.add_entity(dxf_entity);
        }
    }

    drawing
        .save_file(path)
        .map_err(|e| FileError::Dxf(e.to_string()))?;

    Ok(())
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

