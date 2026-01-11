//! 移动 Action
//!
//! 参考 LibreCAD 的 RS_ActionModifyMove 实现

use crate::action::{
    Action, ActionContext, ActionResult, ActionType, MouseButton, PreviewGeometry,
};
use zcad_core::entity::EntityId;
use zcad_core::geometry::{Geometry, Line};
use zcad_core::math::Point2;

/// 移动状态
#[derive(Debug, Clone, PartialEq)]
enum Status {
    /// 等待选择对象（如果没有预选）
    SelectObjects,
    /// 等待指定基点
    SetBasePoint,
    /// 等待指定目标点
    SetDestination,
}

/// 移动 Action
pub struct MoveAction {
    status: Status,
    /// 选中的实体 ID
    entity_ids: Vec<EntityId>,
    /// 基点
    base_point: Option<Point2>,
}

impl MoveAction {
    pub fn new() -> Self {
        Self {
            status: Status::SelectObjects,
            entity_ids: Vec::new(),
            base_point: None,
        }
    }

    /// 使用预选的实体初始化
    pub fn with_selection(entity_ids: Vec<EntityId>) -> Self {
        Self {
            status: if entity_ids.is_empty() {
                Status::SelectObjects
            } else {
                Status::SetBasePoint
            },
            entity_ids,
            base_point: None,
        }
    }
}

impl Default for MoveAction {
    fn default() -> Self {
        Self::new()
    }
}

impl Action for MoveAction {
    fn action_type(&self) -> ActionType {
        ActionType::Move
    }

    fn init(&mut self) {
        // 检查是否有预选对象
    }

    fn reset(&mut self) {
        self.status = Status::SelectObjects;
        self.entity_ids.clear();
        self.base_point = None;
    }

    fn on_mouse_move(&mut self, _ctx: &ActionContext) -> ActionResult {
        ActionResult::Continue
    }

    fn on_mouse_click(&mut self, ctx: &ActionContext, button: MouseButton) -> ActionResult {
        match button {
            MouseButton::Left => {
                let point = ctx.effective_point();
                self.on_coordinate(ctx, point)
            }
            MouseButton::Right => {
                match self.status {
                    Status::SelectObjects => ActionResult::Cancel,
                    Status::SetBasePoint => {
                        self.reset();
                        ActionResult::Continue
                    }
                    Status::SetDestination => {
                        self.status = Status::SetBasePoint;
                        self.base_point = None;
                        ActionResult::Continue
                    }
                }
            }
            MouseButton::Middle => ActionResult::Continue,
        }
    }

    fn on_coordinate(&mut self, ctx: &ActionContext, coord: Point2) -> ActionResult {
        match self.status {
            Status::SelectObjects => {
                // 如果有预选对象，使用它们
                if !ctx.selected_entities.is_empty() {
                    self.entity_ids = ctx.selected_entities.to_vec();
                    self.status = Status::SetBasePoint;
                } else {
                    return ActionResult::NeedSelection;
                }
                ActionResult::Continue
            }
            Status::SetBasePoint => {
                self.base_point = Some(coord);
                self.status = Status::SetDestination;
                ActionResult::Continue
            }
            Status::SetDestination => {
                if let Some(base) = self.base_point {
                    let offset = coord - base;
                    
                    // 返回移动结果（由外部处理实际的实体移动）
                    // 这里返回一个特殊的结果类型来通知外部需要移动实体
                    let result = ActionResult::ModifyEntities(
                        self.entity_ids.iter().map(|&id| {
                            // 创建一个占位几何体，实际移动由外部处理
                            // 这里需要外部知道偏移量
                            (id, Geometry::Line(Line::new(base, coord)))
                        }).collect()
                    );
                    
                    self.reset();
                    return result;
                }
                ActionResult::Continue
            }
        }
    }

    fn on_command(&mut self, _ctx: &ActionContext, _cmd: &str) -> Option<ActionResult> {
        None
    }

    fn get_prompt(&self) -> &str {
        match self.status {
            Status::SelectObjects => "选择要移动的对象:",
            Status::SetBasePoint => "指定基点:",
            Status::SetDestination => "指定第二点 或 <使用第一点作为位移>:",
        }
    }

    fn get_preview(&self, ctx: &ActionContext) -> Vec<PreviewGeometry> {
        let mut previews = Vec::new();
        
        if self.status == Status::SetDestination {
            if let Some(base) = self.base_point {
                let dest = ctx.effective_point();
                // 绘制从基点到目标点的参考线
                let line = Line::new(base, dest);
                previews.push(PreviewGeometry::reference(Geometry::Line(line)));
            }
        }
        
        previews
    }
}
