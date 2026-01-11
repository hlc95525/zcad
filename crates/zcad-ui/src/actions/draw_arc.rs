//! 绘制圆弧 Action

use crate::action::{
    Action, ActionContext, ActionResult, ActionType, MouseButton, PreviewGeometry,
};
use zcad_core::geometry::{Arc, Geometry};
use zcad_core::math::Point2;

/// 圆弧绘制状态（三点法）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Status {
    /// 等待第一点（起点）
    SetPoint1,
    /// 等待第二点（弧上的点）
    SetPoint2,
    /// 等待第三点（终点）
    SetPoint3,
}

/// 绘制圆弧 Action
pub struct DrawArcAction {
    status: Status,
    point1: Option<Point2>,
    point2: Option<Point2>,
}

impl DrawArcAction {
    pub fn new() -> Self {
        Self {
            status: Status::SetPoint1,
            point1: None,
            point2: None,
        }
    }
}

impl Default for DrawArcAction {
    fn default() -> Self {
        Self::new()
    }
}

impl Action for DrawArcAction {
    fn action_type(&self) -> ActionType {
        ActionType::DrawArc
    }

    fn reset(&mut self) {
        self.status = Status::SetPoint1;
        self.point1 = None;
        self.point2 = None;
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
                if self.status != Status::SetPoint1 {
                    self.reset();
                    ActionResult::Continue
                } else {
                    ActionResult::Cancel
                }
            }
            MouseButton::Middle => ActionResult::Continue,
        }
    }

    fn on_coordinate(&mut self, _ctx: &ActionContext, coord: Point2) -> ActionResult {
        match self.status {
            Status::SetPoint1 => {
                self.point1 = Some(coord);
                self.status = Status::SetPoint2;
                ActionResult::Continue
            }
            Status::SetPoint2 => {
                self.point2 = Some(coord);
                self.status = Status::SetPoint3;
                ActionResult::Continue
            }
            Status::SetPoint3 => {
                if let (Some(p1), Some(p2)) = (self.point1, self.point2) {
                    if let Some(arc) = Arc::from_three_points(p1, p2, coord) {
                        self.reset();
                        return ActionResult::CreateEntities(vec![Geometry::Arc(arc)]);
                    }
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
            Status::SetPoint1 => "指定圆弧的起点:",
            Status::SetPoint2 => "指定圆弧上的第二点:",
            Status::SetPoint3 => "指定圆弧的终点:",
        }
    }

    fn get_preview(&self, ctx: &ActionContext) -> Vec<PreviewGeometry> {
        let mut previews = Vec::new();
        
        if self.status == Status::SetPoint3 {
            if let (Some(p1), Some(p2)) = (self.point1, self.point2) {
                let p3 = ctx.effective_point();
                if let Some(arc) = Arc::from_three_points(p1, p2, p3) {
                    previews.push(PreviewGeometry::new(Geometry::Arc(arc)));
                }
            }
        }
        
        previews
    }
}
