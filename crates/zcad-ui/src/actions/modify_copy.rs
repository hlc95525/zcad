//! 复制 Action
//!
//! 参考 LibreCAD 的 RS_ActionModifyCopy 实现

use crate::action::{
    Action, ActionContext, ActionResult, ActionType, MouseButton, PreviewGeometry,
};
use zcad_core::entity::EntityId;
use zcad_core::geometry::{Geometry, Line};
use zcad_core::math::Point2;

/// 复制状态
#[derive(Debug, Clone, PartialEq)]
enum Status {
    /// 等待选择对象
    SelectObjects,
    /// 等待指定基点
    SetBasePoint,
    /// 等待指定目标点（可多次复制）
    SetDestination,
}

/// 复制 Action
pub struct CopyAction {
    status: Status,
    /// 选中的实体 ID
    entity_ids: Vec<EntityId>,
    /// 基点
    base_point: Option<Point2>,
    /// 是否多次复制模式
    multiple: bool,
}

impl CopyAction {
    pub fn new() -> Self {
        Self {
            status: Status::SelectObjects,
            entity_ids: Vec::new(),
            base_point: None,
            multiple: true, // 默认开启多次复制
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
            multiple: true,
        }
    }
}

impl Default for CopyAction {
    fn default() -> Self {
        Self::new()
    }
}

impl Action for CopyAction {
    fn action_type(&self) -> ActionType {
        ActionType::Copy
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
                        // 在多次复制模式下，右键结束复制
                        self.reset();
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
                    // 返回复制结果
                    let result = ActionResult::ModifyEntities(
                        self.entity_ids.iter().map(|&id| {
                            (id, Geometry::Line(Line::new(base, coord)))
                        }).collect()
                    );
                    
                    if !self.multiple {
                        self.reset();
                    }
                    // 在多次复制模式下，保持状态不变，可以继续复制
                    
                    return result;
                }
                ActionResult::Continue
            }
        }
    }

    fn on_command(&mut self, _ctx: &ActionContext, cmd: &str) -> Option<ActionResult> {
        let cmd_upper = cmd.to_uppercase();
        
        match cmd_upper.as_str() {
            "M" | "MULTIPLE" => {
                self.multiple = !self.multiple;
                Some(ActionResult::Continue)
            }
            _ => None,
        }
    }

    fn get_prompt(&self) -> &str {
        match self.status {
            Status::SelectObjects => "选择要复制的对象:",
            Status::SetBasePoint => "指定基点:",
            Status::SetDestination => {
                if self.multiple {
                    "指定第二点 或 [多次(M)]:"
                } else {
                    "指定第二点:"
                }
            }
        }
    }

    fn get_available_commands(&self) -> Vec<&str> {
        if self.status == Status::SetDestination {
            vec!["multiple"]
        } else {
            vec![]
        }
    }

    fn get_preview(&self, ctx: &ActionContext) -> Vec<PreviewGeometry> {
        let mut previews = Vec::new();
        
        if self.status == Status::SetDestination {
            if let Some(base) = self.base_point {
                let dest = ctx.effective_point();
                let line = Line::new(base, dest);
                previews.push(PreviewGeometry::reference(Geometry::Line(line)));
            }
        }
        
        previews
    }
}
