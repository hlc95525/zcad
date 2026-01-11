//! Action 系统 - 参考 LibreCAD 的状态机设计
//!
//! 每个绘图/编辑工具是一个独立的 Action 实现，
//! 采用状态机模式处理用户交互。

use zcad_core::entity::{Entity, EntityId};
use zcad_core::geometry::Geometry;
use zcad_core::math::Point2;

/// Action 执行结果
#[derive(Debug, Clone)]
pub enum ActionResult {
    /// 继续当前 action
    Continue,
    /// 完成当前 action，创建实体
    CreateEntities(Vec<Geometry>),
    /// 完成当前 action，修改实体
    ModifyEntities(Vec<(EntityId, Geometry)>),
    /// 完成当前 action，删除实体
    DeleteEntities(Vec<EntityId>),
    /// 取消当前 action
    Cancel,
    /// 切换到另一个 action
    SwitchTo(ActionType),
    /// 需要选择实体
    NeedSelection,
}

/// Action 类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActionType {
    // 选择
    Select,
    
    // 绘图
    DrawLine,
    DrawCircle,
    DrawArc,
    DrawPolyline,
    DrawRectangle,
    DrawPoint,
    DrawText,
    DrawDimension,
    DrawDimensionRadius,
    DrawDimensionDiameter,
    
    // 修改
    Move,
    Copy,
    Rotate,
    Scale,
    Mirror,
    Erase,
    
    // 其他
    None,
}

impl ActionType {
    /// 获取 action 的名称
    pub fn name(&self) -> &'static str {
        match self {
            ActionType::Select => "Select",
            ActionType::DrawLine => "Line",
            ActionType::DrawCircle => "Circle",
            ActionType::DrawArc => "Arc",
            ActionType::DrawPolyline => "Polyline",
            ActionType::DrawRectangle => "Rectangle",
            ActionType::DrawPoint => "Point",
            ActionType::DrawText => "Text",
            ActionType::DrawDimension => "Dimension",
            ActionType::DrawDimensionRadius => "Radius Dimension",
            ActionType::DrawDimensionDiameter => "Diameter Dimension",
            ActionType::Move => "Move",
            ActionType::Copy => "Copy",
            ActionType::Rotate => "Rotate",
            ActionType::Scale => "Scale",
            ActionType::Mirror => "Mirror",
            ActionType::Erase => "Erase",
            ActionType::None => "None",
        }
    }

    /// 获取快捷键
    pub fn shortcut(&self) -> Option<&'static str> {
        match self {
            ActionType::Select => Some("Space"),
            ActionType::DrawLine => Some("L"),
            ActionType::DrawCircle => Some("C"),
            ActionType::DrawArc => Some("A"),
            ActionType::DrawPolyline => Some("P"),
            ActionType::DrawRectangle => Some("R"),
            ActionType::DrawPoint => Some("."),
            ActionType::DrawText => Some("T"),
            ActionType::DrawDimension => Some("D"),
            ActionType::DrawDimensionRadius => Some("DRA"),
            ActionType::DrawDimensionDiameter => Some("DDI"),
            ActionType::Move => Some("M"),
            ActionType::Copy => Some("CO"),
            ActionType::Rotate => Some("RO"),
            ActionType::Scale => Some("SC"),
            ActionType::Mirror => Some("MI"),
            ActionType::Erase => Some("E"),
            ActionType::None => None,
        }
    }
}

/// Action 上下文 - 传递给 Action 的运行时信息
pub struct ActionContext<'a> {
    /// 鼠标世界坐标
    pub mouse_pos: Point2,
    /// 捕捉后的坐标（如果有）
    pub snap_pos: Option<Point2>,
    /// 当前选中的实体
    pub selected_entities: &'a [EntityId],
    /// 所有实体（用于捕捉等）
    pub entities: &'a [Entity],
    /// 正交模式
    pub ortho_mode: bool,
    /// 参考点（用于相对坐标）
    pub reference_point: Option<Point2>,
}

impl<'a> ActionContext<'a> {
    /// 获取有效点（优先使用捕捉点）
    pub fn effective_point(&self) -> Point2 {
        self.snap_pos.unwrap_or(self.mouse_pos)
    }
}

/// 预览几何体
#[derive(Debug, Clone)]
pub struct PreviewGeometry {
    pub geometry: Geometry,
    pub is_reference: bool, // 是否是参考线（虚线显示）
}

impl PreviewGeometry {
    pub fn new(geometry: Geometry) -> Self {
        Self {
            geometry,
            is_reference: false,
        }
    }

    pub fn reference(geometry: Geometry) -> Self {
        Self {
            geometry,
            is_reference: true,
        }
    }
}

/// 鼠标按钮
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// Action trait - 所有绘图/编辑工具的核心接口
///
/// 参考 LibreCAD 的 RS_ActionInterface
pub trait Action: Send {
    /// 获取 action 类型
    fn action_type(&self) -> ActionType;

    /// 获取 action 名称
    fn name(&self) -> &str {
        self.action_type().name()
    }

    /// 初始化 action
    fn init(&mut self) {}

    /// 重置 action 状态
    fn reset(&mut self);

    // ========== 事件处理 ==========

    /// 鼠标移动事件
    fn on_mouse_move(&mut self, ctx: &ActionContext) -> ActionResult;

    /// 鼠标点击事件
    fn on_mouse_click(&mut self, ctx: &ActionContext, button: MouseButton) -> ActionResult;

    /// 坐标输入事件（来自命令行）
    fn on_coordinate(&mut self, ctx: &ActionContext, coord: Point2) -> ActionResult;

    /// 命令/子命令输入
    fn on_command(&mut self, ctx: &ActionContext, cmd: &str) -> Option<ActionResult>;

    /// 数值输入（半径、长度、角度等）
    fn on_value(&mut self, _ctx: &ActionContext, _value: f64) -> ActionResult {
        ActionResult::Continue
    }

    // ========== UI 提示 ==========

    /// 获取当前状态的提示文本
    fn get_prompt(&self) -> &str;

    /// 获取当前可用的子命令
    fn get_available_commands(&self) -> Vec<&str> {
        vec![]
    }

    // ========== 预览 ==========

    /// 获取预览几何体
    fn get_preview(&self, ctx: &ActionContext) -> Vec<PreviewGeometry>;

    // ========== 历史操作 ==========

    /// 是否可以撤销（action 内部的撤销）
    fn can_undo(&self) -> bool {
        false
    }

    /// 是否可以重做
    fn can_redo(&self) -> bool {
        false
    }

    /// 撤销
    fn undo(&mut self) {}

    /// 重做
    fn redo(&mut self) {}
}

/// Action 历史记录项
#[derive(Debug, Clone)]
pub struct ActionHistoryItem<T: Clone> {
    pub data: T,
}

/// 通用的 Action 历史管理器
#[derive(Debug, Clone)]
pub struct ActionHistory<T: Clone> {
    items: Vec<ActionHistoryItem<T>>,
    index: i32,
}

impl<T: Clone> ActionHistory<T> {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            index: -1,
        }
    }

    pub fn push(&mut self, data: T) {
        // 截断 redo 历史
        let new_len = (self.index + 1) as usize;
        self.items.truncate(new_len);
        
        self.items.push(ActionHistoryItem { data });
        self.index = self.items.len() as i32 - 1;
    }

    pub fn can_undo(&self) -> bool {
        self.index >= 0
    }

    pub fn can_redo(&self) -> bool {
        (self.index + 1) < self.items.len() as i32
    }

    pub fn undo(&mut self) -> Option<&T> {
        if self.can_undo() {
            let item = &self.items[self.index as usize];
            self.index -= 1;
            Some(&item.data)
        } else {
            None
        }
    }

    pub fn redo(&mut self) -> Option<&T> {
        if self.can_redo() {
            self.index += 1;
            Some(&self.items[self.index as usize].data)
        } else {
            None
        }
    }

    pub fn current(&self) -> Option<&T> {
        if self.index >= 0 {
            Some(&self.items[self.index as usize].data)
        } else {
            None
        }
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.index = -1;
    }
}

impl<T: Clone> Default for ActionHistory<T> {
    fn default() -> Self {
        Self::new()
    }
}
