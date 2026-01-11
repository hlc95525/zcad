//! 选择 Action

use crate::action::{
    Action, ActionContext, ActionResult, ActionType, MouseButton, PreviewGeometry,
};
use zcad_core::math::Point2;

/// 选择状态
#[derive(Debug, Clone, Copy, PartialEq)]
enum Status {
    /// 空闲，等待选择
    Idle,
    /// 正在框选
    BoxSelect { start: Point2 },
}

/// 选择 Action
pub struct SelectAction {
    status: Status,
    /// 框选起点（公开用于外部访问）
    box_start: Option<Point2>,
}

impl SelectAction {
    pub fn new() -> Self {
        Self {
            status: Status::Idle,
            box_start: None,
        }
    }

    /// 获取当前框选状态
    pub fn get_box_select(&self) -> Option<Point2> {
        self.box_start
    }

    /// 是否正在框选
    pub fn is_box_selecting(&self) -> bool {
        matches!(self.status, Status::BoxSelect { .. })
    }
}

impl Default for SelectAction {
    fn default() -> Self {
        Self::new()
    }
}

impl Action for SelectAction {
    fn action_type(&self) -> ActionType {
        ActionType::Select
    }

    fn reset(&mut self) {
        self.status = Status::Idle;
        self.box_start = None;
    }

    fn on_mouse_move(&mut self, _ctx: &ActionContext) -> ActionResult {
        ActionResult::Continue
    }

    fn on_mouse_click(&mut self, ctx: &ActionContext, button: MouseButton) -> ActionResult {
        match button {
            MouseButton::Left => {
                match self.status {
                    Status::Idle => {
                        // 开始框选或点选
                        let start = ctx.effective_point();
                        self.status = Status::BoxSelect { start };
                        self.box_start = Some(start);
                        ActionResult::Continue
                    }
                    Status::BoxSelect { start: _ } => {
                        // 结束框选
                        self.status = Status::Idle;
                        self.box_start = None;
                        // 实际的选择逻辑由外部处理
                        ActionResult::Continue
                    }
                }
            }
            MouseButton::Right => {
                self.status = Status::Idle;
                self.box_start = None;
                ActionResult::Continue
            }
            MouseButton::Middle => ActionResult::Continue,
        }
    }

    fn on_coordinate(&mut self, ctx: &ActionContext, _coord: Point2) -> ActionResult {
        self.on_mouse_click(ctx, MouseButton::Left)
    }

    fn on_command(&mut self, _ctx: &ActionContext, _cmd: &str) -> Option<ActionResult> {
        None
    }

    fn get_prompt(&self) -> &str {
        match self.status {
            Status::Idle => "选择对象:",
            Status::BoxSelect { .. } => "指定对角点:",
        }
    }

    fn get_preview(&self, _ctx: &ActionContext) -> Vec<PreviewGeometry> {
        // 框选预览由渲染层处理
        vec![]
    }
}
