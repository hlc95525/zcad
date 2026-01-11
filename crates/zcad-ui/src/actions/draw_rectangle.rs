//! 绘制矩形 Action

use crate::action::{
    Action, ActionContext, ActionResult, ActionType, MouseButton, PreviewGeometry,
};
use zcad_core::geometry::{Geometry, Polyline, PolylineVertex};
use zcad_core::math::Point2;

/// 矩形绘制状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Status {
    /// 等待第一个角点
    SetCorner1,
    /// 等待对角点
    SetCorner2,
}

/// 绘制矩形 Action
pub struct DrawRectangleAction {
    status: Status,
    corner1: Option<Point2>,
}

impl DrawRectangleAction {
    pub fn new() -> Self {
        Self {
            status: Status::SetCorner1,
            corner1: None,
        }
    }

    fn create_rectangle(&self, p1: Point2, p2: Point2) -> Polyline {
        let vertices = vec![
            PolylineVertex::new(Point2::new(p1.x, p1.y)),
            PolylineVertex::new(Point2::new(p2.x, p1.y)),
            PolylineVertex::new(Point2::new(p2.x, p2.y)),
            PolylineVertex::new(Point2::new(p1.x, p2.y)),
        ];
        Polyline::new(vertices, true)
    }
}

impl Default for DrawRectangleAction {
    fn default() -> Self {
        Self::new()
    }
}

impl Action for DrawRectangleAction {
    fn action_type(&self) -> ActionType {
        ActionType::DrawRectangle
    }

    fn reset(&mut self) {
        self.status = Status::SetCorner1;
        self.corner1 = None;
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
                if self.status == Status::SetCorner2 {
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
            Status::SetCorner1 => {
                self.corner1 = Some(coord);
                self.status = Status::SetCorner2;
                ActionResult::Continue
            }
            Status::SetCorner2 => {
                if let Some(c1) = self.corner1 {
                    // 确保矩形有一定大小
                    if (coord.x - c1.x).abs() > 1e-6 && (coord.y - c1.y).abs() > 1e-6 {
                        let rect = self.create_rectangle(c1, coord);
                        self.reset();
                        return ActionResult::CreateEntities(vec![Geometry::Polyline(rect)]);
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
            Status::SetCorner1 => "指定第一个角点:",
            Status::SetCorner2 => "指定对角点:",
        }
    }

    fn get_preview(&self, ctx: &ActionContext) -> Vec<PreviewGeometry> {
        let mut previews = Vec::new();
        
        if self.status == Status::SetCorner2 {
            if let Some(c1) = self.corner1 {
                let c2 = ctx.effective_point();
                let rect = self.create_rectangle(c1, c2);
                previews.push(PreviewGeometry::new(Geometry::Polyline(rect)));
            }
        }
        
        previews
    }
}
