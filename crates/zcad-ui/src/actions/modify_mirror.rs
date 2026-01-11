//! 镜像 Action
//!
//! 参考 LibreCAD 的 RS_ActionModifyMirror 实现

use crate::action::{
    Action, ActionContext, ActionResult, ActionType, MouseButton, PreviewGeometry,
};
use zcad_core::entity::EntityId;
use zcad_core::geometry::{Geometry, Line};
use zcad_core::math::Point2;

/// 镜像状态
#[derive(Debug, Clone, PartialEq)]
enum Status {
    /// 等待选择对象
    SelectObjects,
    /// 等待指定镜像线第一点
    SetPoint1,
    /// 等待指定镜像线第二点
    SetPoint2,
}

/// 镜像 Action
pub struct MirrorAction {
    status: Status,
    /// 选中的实体 ID
    entity_ids: Vec<EntityId>,
    /// 镜像线第一点
    point1: Option<Point2>,
    /// 是否删除原对象
    delete_original: bool,
}

impl MirrorAction {
    pub fn new() -> Self {
        Self {
            status: Status::SelectObjects,
            entity_ids: Vec::new(),
            point1: None,
            delete_original: false,
        }
    }
}

impl Default for MirrorAction {
    fn default() -> Self {
        Self::new()
    }
}

impl Action for MirrorAction {
    fn action_type(&self) -> ActionType {
        ActionType::Mirror
    }

    fn reset(&mut self) {
        self.status = Status::SelectObjects;
        self.entity_ids.clear();
        self.point1 = None;
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
                    _ => {
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
                    self.status = Status::SetPoint1;
                } else {
                    return ActionResult::NeedSelection;
                }
                ActionResult::Continue
            }
            Status::SetPoint1 => {
                self.point1 = Some(coord);
                self.status = Status::SetPoint2;
                ActionResult::Continue
            }
            Status::SetPoint2 => {
                if let Some(p1) = self.point1 {
                    // 确保镜像线有长度
                    if (coord - p1).norm() > 1e-6 {
                        // 返回镜像结果
                        let result = ActionResult::ModifyEntities(
                            self.entity_ids.iter().map(|&id| {
                                // 使用 Line 来传递镜像线
                                (id, Geometry::Line(Line::new(p1, coord)))
                            }).collect()
                        );
                        
                        self.reset();
                        return result;
                    }
                }
                ActionResult::Continue
            }
        }
    }

    fn on_command(&mut self, _ctx: &ActionContext, cmd: &str) -> Option<ActionResult> {
        let cmd_upper = cmd.to_uppercase();
        
        match cmd_upper.as_str() {
            "Y" | "YES" => {
                self.delete_original = true;
                Some(ActionResult::Continue)
            }
            "N" | "NO" => {
                self.delete_original = false;
                Some(ActionResult::Continue)
            }
            _ => None,
        }
    }

    fn get_prompt(&self) -> &str {
        match self.status {
            Status::SelectObjects => "选择要镜像的对象:",
            Status::SetPoint1 => "指定镜像线的第一点:",
            Status::SetPoint2 => "指定镜像线的第二点:",
        }
    }

    fn get_preview(&self, ctx: &ActionContext) -> Vec<PreviewGeometry> {
        let mut previews = Vec::new();
        
        if self.status == Status::SetPoint2 {
            if let Some(p1) = self.point1 {
                let p2 = ctx.effective_point();
                // 绘制镜像线
                let line = Line::new(p1, p2);
                previews.push(PreviewGeometry::reference(Geometry::Line(line)));
            }
        }
        
        previews
    }
}
