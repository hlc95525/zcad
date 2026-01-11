//! 绘制点 Action

use crate::action::{
    Action, ActionContext, ActionResult, ActionType, MouseButton, PreviewGeometry,
};
use zcad_core::geometry::{Geometry, Point};
use zcad_core::math::Point2;

/// 绘制点 Action
pub struct DrawPointAction;

impl DrawPointAction {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DrawPointAction {
    fn default() -> Self {
        Self::new()
    }
}

impl Action for DrawPointAction {
    fn action_type(&self) -> ActionType {
        ActionType::DrawPoint
    }

    fn reset(&mut self) {
        // 点工具无状态
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
            MouseButton::Right => ActionResult::Cancel,
            MouseButton::Middle => ActionResult::Continue,
        }
    }

    fn on_coordinate(&mut self, _ctx: &ActionContext, coord: Point2) -> ActionResult {
        let point = Point::from_point2(coord);
        ActionResult::CreateEntities(vec![Geometry::Point(point)])
    }

    fn on_command(&mut self, _ctx: &ActionContext, _cmd: &str) -> Option<ActionResult> {
        None
    }

    fn get_prompt(&self) -> &str {
        "指定点的位置:"
    }

    fn get_preview(&self, _ctx: &ActionContext) -> Vec<PreviewGeometry> {
        vec![]
    }
}
