//! 绘制圆 Action

use crate::action::{
    Action, ActionContext, ActionResult, ActionType, MouseButton, PreviewGeometry,
};
use zcad_core::geometry::{Circle, Geometry};
use zcad_core::math::Point2;

/// 圆绘制状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Status {
    /// 等待设置圆心
    SetCenter,
    /// 等待设置半径
    SetRadius,
}

/// 绘制圆 Action
pub struct DrawCircleAction {
    status: Status,
    center: Option<Point2>,
}

impl DrawCircleAction {
    pub fn new() -> Self {
        Self {
            status: Status::SetCenter,
            center: None,
        }
    }
}

impl Default for DrawCircleAction {
    fn default() -> Self {
        Self::new()
    }
}

impl Action for DrawCircleAction {
    fn action_type(&self) -> ActionType {
        ActionType::DrawCircle
    }

    fn reset(&mut self) {
        self.status = Status::SetCenter;
        self.center = None;
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
                if self.status == Status::SetRadius {
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
            Status::SetCenter => {
                self.center = Some(coord);
                self.status = Status::SetRadius;
                ActionResult::Continue
            }
            Status::SetRadius => {
                if let Some(center) = self.center {
                    let radius = (coord - center).norm();
                    if radius > 1e-6 {
                        let circle = Circle::new(center, radius);
                        self.reset();
                        return ActionResult::CreateEntities(vec![Geometry::Circle(circle)]);
                    }
                }
                ActionResult::Continue
            }
        }
    }

    fn on_command(&mut self, _ctx: &ActionContext, _cmd: &str) -> Option<ActionResult> {
        None
    }

    fn on_value(&mut self, _ctx: &ActionContext, value: f64) -> ActionResult {
        // 直接输入半径值
        if self.status == Status::SetRadius {
            if let Some(center) = self.center {
                if value > 1e-6 {
                    let circle = Circle::new(center, value);
                    self.reset();
                    return ActionResult::CreateEntities(vec![Geometry::Circle(circle)]);
                }
            }
        }
        ActionResult::Continue
    }

    fn get_prompt(&self) -> &str {
        match self.status {
            Status::SetCenter => "指定圆心:",
            Status::SetRadius => "指定半径 或 [直径(D)]:",
        }
    }

    fn get_preview(&self, ctx: &ActionContext) -> Vec<PreviewGeometry> {
        let mut previews = Vec::new();
        
        if self.status == Status::SetRadius {
            if let Some(center) = self.center {
                let radius = (ctx.effective_point() - center).norm();
                if radius > 1e-6 {
                    let circle = Circle::new(center, radius);
                    previews.push(PreviewGeometry::new(Geometry::Circle(circle)));
                }
            }
        }
        
        previews
    }
}
