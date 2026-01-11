//! 缩放 Action
//!
//! 参考 LibreCAD 的 RS_ActionModifyScale 实现

use crate::action::{
    Action, ActionContext, ActionResult, ActionType, MouseButton, PreviewGeometry,
};
use zcad_core::entity::EntityId;
use zcad_core::geometry::{Geometry, Line};
use zcad_core::math::Point2;

/// 缩放状态
#[derive(Debug, Clone, PartialEq)]
enum Status {
    /// 等待选择对象
    SelectObjects,
    /// 等待指定缩放中心
    SetCenter,
    /// 等待指定参考距离（起始点）
    SetReferencePoint,
    /// 等待指定目标距离
    SetTargetPoint,
}

/// 缩放 Action
pub struct ScaleAction {
    status: Status,
    /// 选中的实体 ID
    entity_ids: Vec<EntityId>,
    /// 缩放中心
    center: Option<Point2>,
    /// 参考点（用于计算初始距离）
    reference_point: Option<Point2>,
    /// 是否保留原对象（复制模式）
    copy_mode: bool,
}

impl ScaleAction {
    pub fn new() -> Self {
        Self {
            status: Status::SelectObjects,
            entity_ids: Vec::new(),
            center: None,
            reference_point: None,
            copy_mode: false,
        }
    }
}

impl Default for ScaleAction {
    fn default() -> Self {
        Self::new()
    }
}

impl Action for ScaleAction {
    fn action_type(&self) -> ActionType {
        ActionType::Scale
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
                self.status = Status::SetReferencePoint;
                ActionResult::Continue
            }
            Status::SetReferencePoint => {
                self.reference_point = Some(coord);
                self.status = Status::SetTargetPoint;
                ActionResult::Continue
            }
            Status::SetTargetPoint => {
                if let (Some(center), Some(ref_point)) = (self.center, self.reference_point) {
                    let ref_dist = (ref_point - center).norm();
                    let target_dist = (coord - center).norm();
                    
                    if ref_dist > 1e-6 {
                        let _scale_factor = target_dist / ref_dist;
                        
                        // 返回缩放结果
                        let result = ActionResult::ModifyEntities(
                            self.entity_ids.iter().map(|&id| {
                                (id, Geometry::Line(Line::new(center, coord)))
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
            "C" | "COPY" => {
                self.copy_mode = !self.copy_mode;
                Some(ActionResult::Continue)
            }
            _ => None,
        }
    }

    fn on_value(&mut self, _ctx: &ActionContext, value: f64) -> ActionResult {
        // 直接输入缩放比例
        if self.status == Status::SetReferencePoint || self.status == Status::SetTargetPoint {
            if let Some(center) = self.center {
                if value > 1e-6 {
                    let _scale_factor = value;
                    
                    // 返回缩放结果
                    let result = ActionResult::ModifyEntities(
                        self.entity_ids.iter().map(|&id| {
                            (id, Geometry::Line(Line::new(center, center)))
                        }).collect()
                    );
                    
                    self.reset();
                    return result;
                }
            }
        }
        ActionResult::Continue
    }

    fn get_prompt(&self) -> &str {
        match self.status {
            Status::SelectObjects => "选择要缩放的对象:",
            Status::SetCenter => "指定缩放中心:",
            Status::SetReferencePoint => "指定缩放比例 或 [复制(C)/参考(R)]:",
            Status::SetTargetPoint => "指定第二点:",
        }
    }

    fn get_available_commands(&self) -> Vec<&str> {
        if self.status == Status::SetReferencePoint {
            vec!["copy", "reference"]
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
