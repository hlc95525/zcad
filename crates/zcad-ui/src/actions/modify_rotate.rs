//! 旋转 Action
//!
//! 参考 LibreCAD 的 RS_ActionModifyRotate 实现

use crate::action::{
    Action, ActionContext, ActionResult, ActionType, MouseButton, PreviewGeometry,
};
use zcad_core::entity::EntityId;
use zcad_core::geometry::{Geometry, Line};
use zcad_core::math::Point2;

/// 旋转状态
#[derive(Debug, Clone, PartialEq)]
enum Status {
    /// 等待选择对象
    SelectObjects,
    /// 等待指定旋转中心
    SetCenter,
    /// 等待指定参考角度（起始角度）
    SetReferenceAngle,
    /// 等待指定目标角度
    SetTargetAngle,
}

/// 旋转 Action
pub struct RotateAction {
    status: Status,
    /// 选中的实体 ID
    entity_ids: Vec<EntityId>,
    /// 旋转中心
    center: Option<Point2>,
    /// 参考角度（起始点）
    reference_point: Option<Point2>,
    /// 是否保留原对象（复制模式）
    copy_mode: bool,
}

impl RotateAction {
    pub fn new() -> Self {
        Self {
            status: Status::SelectObjects,
            entity_ids: Vec::new(),
            center: None,
            reference_point: None,
            copy_mode: false,
        }
    }

    /// 计算角度（从中心到点）
    fn angle_to_point(&self, center: Point2, point: Point2) -> f64 {
        (point.y - center.y).atan2(point.x - center.x)
    }
}

impl Default for RotateAction {
    fn default() -> Self {
        Self::new()
    }
}

impl Action for RotateAction {
    fn action_type(&self) -> ActionType {
        ActionType::Rotate
    }

    fn reset(&mut self) {
        self.status = Status::SelectObjects;
        self.entity_ids.clear();
        self.center = None;
        self.reference_point = None;
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
                    self.status = Status::SetCenter;
                } else {
                    return ActionResult::NeedSelection;
                }
                ActionResult::Continue
            }
            Status::SetCenter => {
                self.center = Some(coord);
                self.status = Status::SetReferenceAngle;
                ActionResult::Continue
            }
            Status::SetReferenceAngle => {
                self.reference_point = Some(coord);
                self.status = Status::SetTargetAngle;
                ActionResult::Continue
            }
            Status::SetTargetAngle => {
                if let (Some(center), Some(ref_point)) = (self.center, self.reference_point) {
                    let ref_angle = self.angle_to_point(center, ref_point);
                    let target_angle = self.angle_to_point(center, coord);
                    let _rotation_angle = target_angle - ref_angle;
                    
                    // 返回旋转结果
                    let result = ActionResult::ModifyEntities(
                        self.entity_ids.iter().map(|&id| {
                            // 使用 Line 来传递中心点和目标点（外部需要解析）
                            (id, Geometry::Line(Line::new(center, coord)))
                        }).collect()
                    );
                    
                    self.reset();
                    return result;
                }
                ActionResult::Continue
            }
        }
    }

    fn on_command(&mut self, _ctx: &ActionContext, cmd: &str) -> Option<ActionResult> {
        let cmd_upper = cmd.to_uppercase();
        
        match cmd_upper.as_str() {
            "C" | "COPY" => {
                self.copy_mode = !self.copy_mode;
                Some(ActionResult::Continue)
            }
            _ => None,
        }
    }

    fn on_value(&mut self, _ctx: &ActionContext, value: f64) -> ActionResult {
        // 直接输入角度值（度数）
        if self.status == Status::SetReferenceAngle || self.status == Status::SetTargetAngle {
            if let Some(center) = self.center {
                let _angle_rad = value.to_radians();
                
                // 返回旋转结果
                let result = ActionResult::ModifyEntities(
                    self.entity_ids.iter().map(|&id| {
                        (id, Geometry::Line(Line::new(center, center)))
                    }).collect()
                );
                
                self.reset();
                return result;
            }
        }
        ActionResult::Continue
    }

    fn get_prompt(&self) -> &str {
        match self.status {
            Status::SelectObjects => "选择要旋转的对象:",
            Status::SetCenter => "指定旋转中心:",
            Status::SetReferenceAngle => "指定参考角度 或 [复制(C)]:",
            Status::SetTargetAngle => "指定新角度:",
        }
    }

    fn get_available_commands(&self) -> Vec<&str> {
        if self.status == Status::SetReferenceAngle {
            vec!["copy"]
        } else {
            vec![]
        }
    }

    fn get_preview(&self, ctx: &ActionContext) -> Vec<PreviewGeometry> {
        let mut previews = Vec::new();
        
        if let Some(center) = self.center {
            let current = ctx.effective_point();
            
            // 从中心到当前点的参考线
            let line = Line::new(center, current);
            previews.push(PreviewGeometry::reference(Geometry::Line(line)));
            
            // 如果有参考点，也画一条线
            if let Some(ref_point) = self.reference_point {
                let ref_line = Line::new(center, ref_point);
                previews.push(PreviewGeometry::reference(Geometry::Line(ref_line)));
            }
        }
        
        previews
    }
}
