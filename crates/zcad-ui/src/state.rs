//! UI状态管理

use zcad_core::entity::EntityId;
use zcad_core::math::Point2;
use zcad_core::snap::{SnapConfig, SnapEngine, SnapPoint, SnapType};

/// 当前绘图工具
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrawingTool {
    None,
    Select,
    Line,
    Circle,
    Arc,
    Polyline,
    Rectangle,
    Point,
    Text,
    Dimension,
    DimensionRadius,
    DimensionDiameter,
}

impl DrawingTool {
    pub fn name(&self) -> &'static str {
        match self {
            DrawingTool::None => "None",
            DrawingTool::Select => "Select",
            DrawingTool::Line => "Line",
            DrawingTool::Circle => "Circle",
            DrawingTool::Arc => "Arc",
            DrawingTool::Polyline => "Polyline",
            DrawingTool::Rectangle => "Rectangle",
            DrawingTool::Point => "Point",
            DrawingTool::Text => "Text",
            DrawingTool::Dimension => "Dimension",
            DrawingTool::DimensionRadius => "Radius Dimension",
            DrawingTool::DimensionDiameter => "Diameter Dimension",
        }
    }

    pub fn shortcut(&self) -> Option<&'static str> {
        match self {
            DrawingTool::Select => Some("Space"),
            DrawingTool::Line => Some("L"),
            DrawingTool::Circle => Some("C"),
            DrawingTool::Arc => Some("A"),
            DrawingTool::Polyline => Some("P"),
            DrawingTool::Rectangle => Some("R"),
            DrawingTool::Point => Some("."),
            DrawingTool::Text => Some("T"),
            DrawingTool::Dimension => Some("D"),
            DrawingTool::DimensionRadius => Some("DRA"),
            DrawingTool::DimensionDiameter => Some("DDI"),
            DrawingTool::None => None,
        }
    }
}

/// 捕捉模式（保留向后兼容，实际使用SnapEngine）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnapMode {
    pub endpoint: bool,
    pub midpoint: bool,
    pub center: bool,
    pub intersection: bool,
    pub perpendicular: bool,
    pub tangent: bool,
    pub nearest: bool,
    pub grid: bool,
}

impl Default for SnapMode {
    fn default() -> Self {
        Self {
            endpoint: true,
            midpoint: true,
            center: true,
            intersection: true,
            perpendicular: false,
            tangent: false,
            nearest: false,
            grid: false,
        }
    }
}

/// 当前捕捉状态
#[derive(Debug, Clone)]
pub struct SnapState {
    /// 捕捉引擎
    engine: SnapEngine,
    /// 当前捕捉到的点
    pub current_snap: Option<SnapPoint>,
    /// 是否启用捕捉
    pub enabled: bool,
}

impl SnapState {
    pub fn new() -> Self {
        Self {
            engine: SnapEngine::default(),
            current_snap: None,
            enabled: true,
        }
    }

    /// 获取捕捉引擎的可变引用
    pub fn engine_mut(&mut self) -> &mut SnapEngine {
        &mut self.engine
    }

    /// 获取捕捉引擎的引用
    pub fn engine(&self) -> &SnapEngine {
        &self.engine
    }

    /// 获取捕捉配置
    pub fn config(&self) -> &SnapConfig {
        self.engine.config()
    }

    /// 获取捕捉配置（可变）
    pub fn config_mut(&mut self) -> &mut SnapConfig {
        self.engine.config_mut()
    }

    /// 切换捕捉类型
    pub fn toggle_snap_type(&mut self, snap_type: SnapType) {
        self.engine.config_mut().enabled_types.toggle(snap_type);
    }

    /// 检查捕捉类型是否启用
    pub fn is_snap_type_enabled(&self, snap_type: SnapType) -> bool {
        self.engine.config().enabled_types.is_enabled(snap_type)
    }
}

impl Default for SnapState {
    fn default() -> Self {
        Self::new()
    }
}

/// 期望的输入类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputType {
    /// 需要点坐标
    Point,
    /// 需要半径
    Radius,
    /// 需要长度
    Length,
    /// 需要角度
    Angle,
    /// 需要尺寸（宽高）
    Dimensions,
    /// 需要长度和角度
    LengthAngle,
}

impl InputType {
    /// 获取输入提示文本
    pub fn hint(&self) -> &'static str {
        match self {
            InputType::Point => "输入坐标 (如: 100,50 或 @100,50 或 @100<45)",
            InputType::Radius => "输入半径 (如: 50)",
            InputType::Length => "输入长度 (如: 100)",
            InputType::Angle => "输入角度 (如: <45)",
            InputType::Dimensions => "输入尺寸 (如: 100,50)",
            InputType::LengthAngle => "输入长度和角度 (如: 100<45)",
        }
    }
}

/// 编辑状态
#[derive(Debug, Clone)]
pub enum EditState {
    /// 空闲
    Idle,
    /// 正在绘制
    Drawing {
        tool: DrawingTool,
        points: Vec<Point2>,
        /// 当前期望的输入类型
        expected_input: Option<InputType>,
    },
    /// 选择中
    Selecting {
        start: Point2,
    },
    /// 移动选择的对象
    Moving {
        start: Point2,
        offset: Point2,
    },
    /// 等待命令输入
    Command {
        input: String,
    },
    /// 正在输入文本（新建）
    TextInput {
        position: Point2,
        content: String,
        height: f64,
    },
    /// 正在编辑现有文本
    TextEdit {
        entity_id: EntityId,
        position: Point2,
        content: String,
        height: f64,
    },
    /// 正在移动实体
    MovingEntities {
        start_pos: Point2,
        entity_ids: Vec<EntityId>,
    },
    /// 移动操作
    MoveOp {
        entity_ids: Vec<EntityId>,
        base_point: Option<Point2>,
    },
    /// 复制操作
    CopyOp {
        entity_ids: Vec<EntityId>,
        base_point: Option<Point2>,
    },
    /// 旋转操作
    RotateOp {
        entity_ids: Vec<EntityId>,
        center: Option<Point2>,
        start_angle: Option<f64>, // 鼠标初始角度
    },
    /// 缩放操作
    ScaleOp {
        entity_ids: Vec<EntityId>,
        center: Option<Point2>,
        start_dist: Option<f64>, // 鼠标初始距离
    },
    /// 镜像操作
    MirrorOp {
        entity_ids: Vec<EntityId>,
        point1: Option<Point2>, // 镜像线第一点
    },
}

impl Default for EditState {
    fn default() -> Self {
        Self::Idle
    }
}

/// UI状态
#[derive(Debug)]
pub struct UiState {
    /// 当前工具
    pub current_tool: DrawingTool,

    /// 编辑状态
    pub edit_state: EditState,

    /// 选中的实体
    pub selected_entities: Vec<EntityId>,

    /// 鼠标在世界坐标中的位置（原始位置）
    pub mouse_world_pos: Point2,

    /// 捕捉状态
    pub snap_state: SnapState,

    /// 捕捉到的点（如果有）- 保留向后兼容
    pub snap_point: Option<Point2>,

    /// 捕捉模式 - 保留向后兼容
    pub snap_mode: SnapMode,

    /// 是否显示网格
    pub show_grid: bool,

    /// 网格间距
    pub grid_spacing: f64,

    /// 命令行输入
    pub command_input: String,

    /// 命令历史
    pub command_history: Vec<String>,

    /// 状态栏消息
    pub status_message: String,

    /// 是否显示图层面板
    pub show_layers_panel: bool,

    /// 是否显示属性面板
    pub show_properties_panel: bool,

    /// 正交模式
    pub ortho_mode: bool,

    /// 待处理的命令（由UI组件生成）
    pub pending_command: Option<Command>,

    /// 上一次执行的命令（用于空格键重复）
    pub last_command: Option<Command>,

    /// 是否需要聚焦命令行
    pub should_focus_command_line: bool,
}

impl UiState {
    /// 获取实际使用的点（优先使用捕捉点）
    pub fn effective_point(&self) -> Point2 {
        if let Some(ref snap) = self.snap_state.current_snap {
            if self.snap_state.enabled {
                return snap.point;
            }
        }
        self.mouse_world_pos
    }

    /// 获取当前捕捉点信息
    pub fn current_snap(&self) -> Option<&SnapPoint> {
        if self.snap_state.enabled {
            self.snap_state.current_snap.as_ref()
        } else {
            None
        }
    }
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            current_tool: DrawingTool::Select,
            edit_state: EditState::Idle,
            selected_entities: Vec::new(),
            mouse_world_pos: Point2::origin(),
            snap_state: SnapState::default(),
            snap_point: None,
            snap_mode: SnapMode::default(),
            show_grid: true,
            grid_spacing: 10.0,
            command_input: String::new(),
            command_history: Vec::new(),
            status_message: "Ready".to_string(),
            show_layers_panel: true,
            show_properties_panel: true,
            ortho_mode: false,
            pending_command: None,
            last_command: None,
            should_focus_command_line: false,
        }
    }
}

impl UiState {
    /// 设置当前工具
    pub fn set_tool(&mut self, tool: DrawingTool) {
        self.current_tool = tool;
        self.edit_state = EditState::Idle;
        self.status_message = match tool {
            DrawingTool::Dimension => "标注工具已选择。指定第一点或 [半径(R)/直径(D)]:".to_string(),
            DrawingTool::DimensionRadius => "半径标注工具已选择。请选择圆或圆弧:".to_string(),
            DrawingTool::DimensionDiameter => "直径标注工具已选择。请选择圆或圆弧:".to_string(),
            _ => format!("{} 工具已选择", tool.name()),
        };
    }

    /// 取消当前操作
    pub fn cancel(&mut self) {
        self.edit_state = EditState::Idle;
        // 如果当前有工具（非选择工具），则切换回选择工具
        if self.current_tool != DrawingTool::Select {
            self.current_tool = DrawingTool::Select;
            self.status_message = "Cancelled. Tool reset to Select.".to_string();
        } else {
            // 如果已经是选择工具，则仅清空选择（如果有选中），或仅显示取消
            if !self.selected_entities.is_empty() {
                self.selected_entities.clear();
                self.status_message = "Selection cleared.".to_string();
            } else {
                self.status_message = "Cancelled.".to_string();
            }
        }
    }

    /// 清空选择
    pub fn clear_selection(&mut self) {
        self.selected_entities.clear();
    }

    /// 添加到选择
    pub fn add_to_selection(&mut self, id: EntityId) {
        if !self.selected_entities.contains(&id) {
            self.selected_entities.push(id);
        }
    }

    /// 从选择中移除
    pub fn remove_from_selection(&mut self, id: &EntityId) {
        self.selected_entities.retain(|e| e != id);
    }

    /// 切换选择状态
    pub fn toggle_selection(&mut self, id: EntityId) {
        if self.selected_entities.contains(&id) {
            self.remove_from_selection(&id);
        } else {
            self.add_to_selection(id);
        }
    }

    /// 执行命令
    pub fn execute_command(&mut self, command: &str) -> Option<Command> {
        let trimmed = command.trim();

        if trimmed.is_empty() {
            // 空命令（空格/回车），重复上一次命令
            if let Some(cmd) = &self.last_command {
                // 如果上一次是 DataInput，通常不重复数据，而是重复工具
                // 但这里我们简单重复整个命令。更好的做法是只记录非 DataInput 的命令。
                return Some(cmd.clone());
            }
            return None;
        }

        // 添加到历史
        self.command_history.push(command.to_string());

        // 检查是否在绘图状态，如果是，优先尝试解析为数据输入
        if let EditState::Drawing { tool, .. } = &self.edit_state {
            // 在标注工具下，R和D是子命令，优先处理
            if *tool == DrawingTool::Dimension {
                let upper = trimmed.to_uppercase();
                if matches!(upper.as_str(), "R" | "RADIUS" | "D" | "DIAMETER") {
                    return Some(Command::DataInput(trimmed.to_string()));
                }
            }

            // 在绘图状态下，尝试解析为数据输入
            // 如果输入看起来像数据（包含数字、@、<、,等），则作为数据输入处理
            if Self::looks_like_data_input(trimmed) {
                return Some(Command::DataInput(trimmed.to_string()));
            }
        }

        // 否则按命令解析
        let trimmed_upper = trimmed.to_uppercase();
        let cmd = match trimmed_upper.as_str() {
            "L" | "LINE" => Some(Command::SetTool(DrawingTool::Line)),
            "C" | "CIRCLE" => Some(Command::SetTool(DrawingTool::Circle)),
            "A" | "ARC" => Some(Command::SetTool(DrawingTool::Arc)),
            "P" | "PL" | "PLINE" | "POLYLINE" => Some(Command::SetTool(DrawingTool::Polyline)),
            "R" | "REC" | "RECTANGLE" => Some(Command::SetTool(DrawingTool::Rectangle)),
            "T" | "TEXT" | "DTEXT" | "MTEXT" => Some(Command::SetTool(DrawingTool::Text)),
            "D" | "DIM" | "DIMENSION" | "DIMLINEAR" | "DIMALIGNED" => Some(Command::SetTool(DrawingTool::Dimension)),
            "DRA" | "DIMRADIUS" => Some(Command::SetTool(DrawingTool::DimensionRadius)),
            "DDI" | "DIMDIAMETER" => Some(Command::SetTool(DrawingTool::DimensionDiameter)),
            "E" | "ERASE" | "DELETE" => Some(Command::DeleteSelected),
            "M" | "MOVE" => Some(Command::Move),
            "CO" | "COPY" => Some(Command::Copy),
            "RO" | "ROTATE" => Some(Command::Rotate),
            "SC" | "SCALE" => Some(Command::Scale),
            "MI" | "MIRROR" => Some(Command::Mirror),
            "Z" | "ZOOM" => Some(Command::ZoomExtents),
            "ZE" | "ZOOM EXTENTS" => Some(Command::ZoomExtents),
            "U" | "UNDO" => Some(Command::Undo),
            "REDO" => Some(Command::Redo),
            "EXPORT" | "DXFOUT" => Some(Command::ExportDxf),
            "ESC" => {
                self.cancel();
                None
            }
            _ => {
                self.status_message = format!("Unknown command: {}", command);
                None
            }
        };

        if let Some(ref c) = cmd {
            self.status_message = format!("Command: {:?}", c);
            // 记录上一次的有效命令（不记录数据输入）
            if !matches!(c, Command::DataInput(_)) {
                self.last_command = Some(c.clone());
            }
        }

        cmd
    }

    /// 检查输入是否看起来像数据输入（而不是命令）
    fn looks_like_data_input(input: &str) -> bool {
        // 如果包含数字、@、<、,等符号，可能是数据输入
        input.chars().any(|c| c.is_ascii_digit() || c == '@' || c == '<' || c == ',' || c == '.' || c == '-')
    }
}

/// 命令类型
#[derive(Debug, Clone)]
pub enum Command {
    SetTool(DrawingTool),
    DeleteSelected,
    Move,
    Copy,
    Rotate,
    Scale,
    Mirror,
    ZoomExtents,
    Undo,
    Redo,
    New,
    Open,
    Save,
    ExportDxf,
    /// 数据输入（在绘图状态下）
    DataInput(String),
}

