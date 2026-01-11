//! DXF 原始文本解析器
//!
//! 自己解析 DXF 文本格式，支持完整的 Layout/Viewport 功能。
//!
//! # DXF 文件结构
//!
//! DXF 文件由多个段 (Section) 组成：
//! ```text
//! 0
//! SECTION
//! 2
//! HEADER          ← 文件设置
//! ...
//! 0
//! ENDSEC
//! 0
//! SECTION
//! 2
//! TABLES          ← 图层、样式、块记录等
//! ...
//! 0
//! ENDSEC
//! 0
//! SECTION
//! 2
//! BLOCKS          ← 块定义（包括 *Model_Space, *Paper_Space）
//! ...
//! 0
//! ENDSEC
//! 0
//! SECTION
//! 2
//! ENTITIES        ← 实体（线、圆等）
//! ...
//! 0
//! ENDSEC
//! 0
//! SECTION
//! 2
//! OBJECTS         ← 对象（包括 LAYOUT）
//! ...
//! 0
//! ENDSEC
//! 0
//! EOF
//! ```
//!
//! # 组码 (Group Code)
//!
//! 每个数据项由两行组成：
//! - 第一行：组码（数字）
//! - 第二行：值
//!
//! 常用组码：
//! - 0: 实体类型
//! - 2: 名称
//! - 5: 句柄 (Handle)
//! - 10, 20, 30: X, Y, Z 坐标
//! - 11, 21, 31: 第二个点
//! - 40, 41, 42...: 浮点数值
//! - 62: 颜色
//! - 8: 图层名
//! - 330: 软指针（所属对象）
//! - 360: 硬指针（拥有的对象）

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::fs::File;

use crate::error::FileError;
use zcad_core::math::Point2;

/// DXF 组码-值对
#[derive(Debug, Clone)]
pub struct DxfPair {
    pub code: i32,
    pub value: String,
}

impl DxfPair {
    pub fn new(code: i32, value: impl Into<String>) -> Self {
        Self { code, value: value.into() }
    }

    /// 解析为浮点数
    pub fn as_f64(&self) -> Option<f64> {
        self.value.trim().parse().ok()
    }

    /// 解析为整数
    pub fn as_i32(&self) -> Option<i32> {
        self.value.trim().parse().ok()
    }

    /// 解析为整数（i64）
    pub fn as_i64(&self) -> Option<i64> {
        self.value.trim().parse().ok()
    }
}

/// DXF 原始解析器
pub struct DxfRawParser {
    pairs: Vec<DxfPair>,
    position: usize,
}

impl DxfRawParser {
    /// 从文件加载
    pub fn load(path: &Path) -> Result<Self, FileError> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        Self::parse(reader)
    }

    /// 从文本解析
    pub fn parse<R: BufRead>(reader: R) -> Result<Self, FileError> {
        let mut pairs = Vec::new();
        let mut lines = reader.lines();

        loop {
            // 读取组码
            let code_line = match lines.next() {
                Some(Ok(line)) => line,
                Some(Err(e)) => return Err(FileError::Io(e)),
                None => break,
            };

            // 读取值
            let value_line = match lines.next() {
                Some(Ok(line)) => line,
                Some(Err(e)) => return Err(FileError::Io(e)),
                None => break,
            };

            let code: i32 = code_line.trim().parse()
                .map_err(|_| FileError::InvalidFormat(format!("Invalid group code: {}", code_line)))?;

            pairs.push(DxfPair::new(code, value_line));

            // 检查是否到达文件末尾
            if code == 0 && pairs.last().map(|p| p.value.trim()) == Some("EOF") {
                break;
            }
        }

        Ok(Self { pairs, position: 0 })
    }

    /// 获取当前对
    pub fn current(&self) -> Option<&DxfPair> {
        self.pairs.get(self.position)
    }

    /// 前进一步
    pub fn advance(&mut self) -> Option<&DxfPair> {
        if self.position < self.pairs.len() {
            let pair = &self.pairs[self.position];
            self.position += 1;
            Some(pair)
        } else {
            None
        }
    }

    /// 回退一步
    pub fn back(&mut self) {
        if self.position > 0 {
            self.position -= 1;
        }
    }

    /// 跳到下一个指定组码
    pub fn skip_to(&mut self, code: i32, value: Option<&str>) -> bool {
        while let Some(pair) = self.advance() {
            if pair.code == code {
                if let Some(v) = value {
                    if pair.value.trim() == v {
                        return true;
                    }
                } else {
                    return true;
                }
            }
        }
        false
    }

    /// 读取直到遇到组码 0
    pub fn read_until_zero(&mut self) -> Vec<DxfPair> {
        let mut result = Vec::new();
        while let Some(pair) = self.current() {
            if pair.code == 0 {
                break;
            }
            result.push(pair.clone());
            self.advance();
        }
        result
    }
}

/// DXF 布局信息
#[derive(Debug, Clone)]
pub struct DxfLayout {
    /// 布局名称
    pub name: String,
    /// 句柄
    pub handle: String,
    /// 关联的块记录句柄
    pub block_record_handle: String,
    /// 图纸宽度
    pub paper_width: f64,
    /// 图纸高度
    pub paper_height: f64,
    /// 左边距
    pub left_margin: f64,
    /// 下边距
    pub bottom_margin: f64,
    /// 右边距
    pub right_margin: f64,
    /// 上边距
    pub top_margin: f64,
    /// 是否是模型空间
    pub is_model_space: bool,
    /// 布局顺序
    pub tab_order: i32,
}

impl Default for DxfLayout {
    fn default() -> Self {
        Self {
            name: "Layout1".to_string(),
            handle: String::new(),
            block_record_handle: String::new(),
            paper_width: 420.0,  // A3 宽度
            paper_height: 297.0, // A3 高度
            left_margin: 10.0,
            bottom_margin: 10.0,
            right_margin: 10.0,
            top_margin: 10.0,
            is_model_space: false,
            tab_order: 1,
        }
    }
}

/// DXF 视口信息
#[derive(Debug, Clone)]
pub struct DxfViewport {
    /// 句柄
    pub handle: String,
    /// 视口 ID
    pub id: i32,
    /// 中心点（图纸空间）
    pub center: Point2,
    /// 宽度（图纸空间）
    pub width: f64,
    /// 高度（图纸空间）
    pub height: f64,
    /// 视图中心（模型空间）
    pub view_center: Point2,
    /// 视图高度（模型空间）
    pub view_height: f64,
    /// 视口状态
    pub status: i32,
    /// 所属块记录句柄
    pub owner_handle: String,
}

impl Default for DxfViewport {
    fn default() -> Self {
        Self {
            handle: String::new(),
            id: 2,
            center: Point2::new(0.0, 0.0),
            width: 300.0,
            height: 200.0,
            view_center: Point2::new(0.0, 0.0),
            view_height: 200.0,
            status: 1,
            owner_handle: String::new(),
        }
    }
}

/// 解析 OBJECTS 段中的 LAYOUT 对象
pub fn parse_layouts(parser: &mut DxfRawParser) -> Vec<DxfLayout> {
    let mut layouts = Vec::new();

    // 跳到 OBJECTS 段
    parser.position = 0;
    if !parser.skip_to(2, Some("OBJECTS")) {
        return layouts;
    }

    // 查找所有 LAYOUT 对象
    while let Some(pair) = parser.advance() {
        if pair.code == 0 && pair.value.trim() == "ENDSEC" {
            break;
        }

        if pair.code == 0 && pair.value.trim() == "LAYOUT" {
            let mut layout = DxfLayout::default();

            // 读取布局属性
            let pairs = parser.read_until_zero();
            for pair in &pairs {
                match pair.code {
                    5 => layout.handle = pair.value.trim().to_string(),
                    1 => layout.name = pair.value.trim().to_string(),
                    330 => layout.block_record_handle = pair.value.trim().to_string(),
                    44 => layout.paper_width = pair.as_f64().unwrap_or(420.0),
                    45 => layout.paper_height = pair.as_f64().unwrap_or(297.0),
                    40 => layout.left_margin = pair.as_f64().unwrap_or(10.0),
                    41 => layout.bottom_margin = pair.as_f64().unwrap_or(10.0),
                    42 => layout.right_margin = pair.as_f64().unwrap_or(10.0),
                    43 => layout.top_margin = pair.as_f64().unwrap_or(10.0),
                    71 => layout.tab_order = pair.as_i32().unwrap_or(1),
                    _ => {}
                }
            }

            // 判断是否是模型空间
            layout.is_model_space = layout.name == "Model";

            layouts.push(layout);
        }
    }

    // 按 tab_order 排序
    layouts.sort_by_key(|l| l.tab_order);

    layouts
}

/// 解析 ENTITIES 段中的 VIEWPORT 实体
pub fn parse_viewports(parser: &mut DxfRawParser) -> Vec<DxfViewport> {
    let mut viewports = Vec::new();

    // 从头开始
    parser.position = 0;
    if !parser.skip_to(2, Some("ENTITIES")) {
        return viewports;
    }

    // 查找所有 VIEWPORT 实体
    while let Some(pair) = parser.advance() {
        if pair.code == 0 && pair.value.trim() == "ENDSEC" {
            break;
        }

        if pair.code == 0 && pair.value.trim() == "VIEWPORT" {
            let mut viewport = DxfViewport::default();
            let mut x = 0.0;
            let mut y = 0.0;
            let mut view_x = 0.0;
            let mut view_y = 0.0;

            // 读取视口属性
            let pairs = parser.read_until_zero();
            for pair in &pairs {
                match pair.code {
                    5 => viewport.handle = pair.value.trim().to_string(),
                    330 => viewport.owner_handle = pair.value.trim().to_string(),
                    69 => viewport.id = pair.as_i32().unwrap_or(2),
                    10 => x = pair.as_f64().unwrap_or(0.0),
                    20 => y = pair.as_f64().unwrap_or(0.0),
                    40 => viewport.width = pair.as_f64().unwrap_or(300.0),
                    41 => viewport.height = pair.as_f64().unwrap_or(200.0),
                    12 => view_x = pair.as_f64().unwrap_or(0.0),
                    22 => view_y = pair.as_f64().unwrap_or(0.0),
                    45 => viewport.view_height = pair.as_f64().unwrap_or(200.0),
                    68 => viewport.status = pair.as_i32().unwrap_or(1),
                    _ => {}
                }
            }

            viewport.center = Point2::new(x, y);
            viewport.view_center = Point2::new(view_x, view_y);

            // 跳过 ID 为 1 的视口（整个图纸空间视口）
            if viewport.id != 1 {
                viewports.push(viewport);
            }
        }
    }

    viewports
}

/// DXF 写入器
pub struct DxfWriter {
    output: Vec<String>,
    handle_counter: u64,
}

impl DxfWriter {
    pub fn new() -> Self {
        Self {
            output: Vec::new(),
            handle_counter: 100, // 从 100 开始分配句柄
        }
    }

    /// 生成新句柄
    pub fn new_handle(&mut self) -> String {
        let handle = format!("{:X}", self.handle_counter);
        self.handle_counter += 1;
        handle
    }

    /// 写入组码-值对
    pub fn write_pair(&mut self, code: i32, value: impl std::fmt::Display) {
        self.output.push(format!("{:>3}", code));
        self.output.push(value.to_string());
    }

    /// 写入句柄（组码 5）并返回生成的句柄值
    pub fn write_handle(&mut self) -> String {
        let handle = format!("{:X}", self.handle_counter);
        self.handle_counter += 1;
        self.output.push(format!("{:>3}", 5));
        self.output.push(handle.clone());
        handle
    }

    /// 写入句柄（组码 5）但不返回值
    pub fn write_handle_only(&mut self) {
        let handle = format!("{:X}", self.handle_counter);
        self.handle_counter += 1;
        self.output.push(format!("{:>3}", 5));
        self.output.push(handle);
    }

    /// 写入点坐标
    pub fn write_point(&mut self, base_code: i32, point: Point2) {
        self.write_pair(base_code, point.x);
        self.write_pair(base_code + 10, point.y);
        self.write_pair(base_code + 20, 0.0); // Z = 0
    }

    /// 写入 SECTION 开始
    pub fn begin_section(&mut self, name: &str) {
        self.write_pair(0, "SECTION");
        self.write_pair(2, name);
    }

    /// 写入 SECTION 结束
    pub fn end_section(&mut self) {
        self.write_pair(0, "ENDSEC");
    }

    /// 写入 VIEWPORT 实体
    pub fn write_viewport(&mut self, viewport: &DxfViewport, owner_handle: &str) {
        let handle = self.new_handle();
        
        self.write_pair(0, "VIEWPORT");
        self.write_pair(5, &handle);
        self.write_pair(330, owner_handle); // 所属块记录
        self.write_pair(100, "AcDbEntity");
        self.write_pair(67, 1); // 图纸空间标记
        self.write_pair(8, "0"); // 图层
        self.write_pair(100, "AcDbViewport");
        
        // 视口中心（图纸空间）
        self.write_pair(10, viewport.center.x);
        self.write_pair(20, viewport.center.y);
        self.write_pair(30, 0.0);
        
        // 视口尺寸
        self.write_pair(40, viewport.width);
        self.write_pair(41, viewport.height);
        
        // 视口 ID
        self.write_pair(69, viewport.id);
        
        // 视图中心（模型空间）
        self.write_pair(12, viewport.view_center.x);
        self.write_pair(22, viewport.view_center.y);
        
        // 视图高度
        self.write_pair(45, viewport.view_height);
        
        // 视口状态
        self.write_pair(68, viewport.status);
        
        // 视口开启
        self.write_pair(90, 32864); // 标准标志
    }

    /// 写入 LAYOUT 对象
    pub fn write_layout(&mut self, layout: &DxfLayout, dict_handle: &str) -> String {
        let handle = self.new_handle();
        
        self.write_pair(0, "LAYOUT");
        self.write_pair(5, &handle);
        self.write_pair(330, dict_handle); // 所属字典
        self.write_pair(100, "AcDbPlotSettings");
        
        // 图纸设置
        self.write_pair(1, ""); // 页面设置名
        self.write_pair(2, "none_device"); // 打印机名
        self.write_pair(4, ""); // 图纸尺寸名
        
        // 边距
        self.write_pair(40, layout.left_margin);
        self.write_pair(41, layout.bottom_margin);
        self.write_pair(42, layout.right_margin);
        self.write_pair(43, layout.top_margin);
        
        // 图纸尺寸
        self.write_pair(44, layout.paper_width);
        self.write_pair(45, layout.paper_height);
        
        self.write_pair(100, "AcDbLayout");
        
        // 布局名称
        self.write_pair(1, &layout.name);
        
        // 布局标志
        self.write_pair(70, 1);
        
        // 布局顺序
        self.write_pair(71, layout.tab_order);
        
        // 关联的块记录
        self.write_pair(330, &layout.block_record_handle);
        
        handle
    }

    /// 获取输出
    pub fn finish(mut self) -> String {
        self.write_pair(0, "EOF");
        self.output.join("\n")
    }

    /// 保存到文件
    pub fn save_to_file(self, path: &Path) -> Result<(), FileError> {
        let content = self.finish();
        let mut file = File::create(path)?;
        file.write_all(content.as_bytes())?;
        Ok(())
    }
}

/// 示例：解析 DXF 文件中的布局和视口
/// 
/// ```rust,ignore
/// use zcad_file::dxf_raw::{DxfRawParser, parse_layouts, parse_viewports};
/// 
/// let mut parser = DxfRawParser::load(Path::new("example.dxf"))?;
/// 
/// // 解析布局
/// let layouts = parse_layouts(&mut parser);
/// for layout in &layouts {
///     println!("Layout: {} ({}x{})", 
///         layout.name, layout.paper_width, layout.paper_height);
/// }
/// 
/// // 解析视口
/// let viewports = parse_viewports(&mut parser);
/// for vp in &viewports {
///     println!("Viewport {}: center={:?}, view_center={:?}", 
///         vp.id, vp.center, vp.view_center);
/// }
/// ```

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pair() {
        let pair = DxfPair::new(10, "100.5");
        assert_eq!(pair.as_f64(), Some(100.5));

        let pair = DxfPair::new(70, "42");
        assert_eq!(pair.as_i32(), Some(42));
    }

    #[test]
    fn test_dxf_writer() {
        let mut writer = DxfWriter::new();
        writer.begin_section("HEADER");
        writer.write_pair(9, "$ACADVER");
        writer.write_pair(1, "AC1027"); // AutoCAD 2013
        writer.end_section();
        
        let output = writer.finish();
        assert!(output.contains("SECTION"));
        assert!(output.contains("HEADER"));
        assert!(output.contains("AC1027"));
        assert!(output.contains("EOF"));
    }
}
