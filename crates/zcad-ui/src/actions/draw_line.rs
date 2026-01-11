//! 绘制线段 Action
//!
//! 参考 LibreCAD 的 RS_ActionDrawLine 实现

use crate::action::{
    Action, ActionContext, ActionHistory, ActionResult, ActionType, MouseButton, PreviewGeometry,
};
use zcad_core::geometry::{Geometry, Line};
use zcad_core::math::Point2;

/// 线段绘制状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Status {
    /// 等待设置起点
    SetStartpoint,
    /// 等待设置终点
    SetEndpoint,
}

/// 历史动作类型
#[derive(Debug, Clone)]
enum HistoryAction {
    SetStartpoint { point: Point2 },
    SetEndpoint { from: Point2, to: Point2 },
    Close { from: Point2, to: Point2 },
    Next,
}

/// 绘制线段 Action
pub struct DrawLineAction {
    status: Status,
    /// 当前起点
    start_point: Option<Point2>,
    /// 已确定的点列表（用于连续画线）
    points: Vec<Point2>,
    /// 历史记录（用于 undo/redo）
    history: ActionHistory<HistoryAction>,
    /// 起点偏移（用于 close 命令）
    start_offset: usize,
}

impl DrawLineAction {
    pub fn new() -> Self {
        Self {
            status: Status::SetStartpoint,
            start_point: None,
            points: Vec::new(),
            history: ActionHistory::new(),
            start_offset: 0,
        }
    }

    /// 触发创建线段
    fn trigger(&mut self, start: Point2, end: Point2) -> ActionResult {
        let line = Line::new(start, end);
        ActionResult::CreateEntities(vec![Geometry::Line(line)])
    }

    /// 闭合线段序列
    fn close(&mut self) -> ActionResult {
        if self.points.len() >= 2 && self.start_point.is_some() {
            let start = self.start_point.unwrap();
            let end = self.points[0]; // 回到第一个点
            
            if (start - end).norm() > 1e-6 {
                self.history.push(HistoryAction::Close { from: start, to: end });
                self.status = Status::SetStartpoint;
                self.start_point = None;
                self.points.clear();
                self.start_offset = 0;
                return self.trigger(start, end);
            }
        }
        ActionResult::Continue
    }

    /// 开始新的线段序列（不闭合）
    fn next(&mut self) {
        self.history.push(HistoryAction::Next);
        self.status = Status::SetStartpoint;
        self.start_point = None;
        self.start_offset = 0;
    }
}

impl Default for DrawLineAction {
    fn default() -> Self {
        Self::new()
    }
}

impl Action for DrawLineAction {
    fn action_type(&self) -> ActionType {
        ActionType::DrawLine
    }

    fn reset(&mut self) {
        self.status = Status::SetStartpoint;
        self.start_point = None;
        self.points.clear();
        self.history.clear();
        self.start_offset = 0;
    }

    fn on_mouse_move(&mut self, _ctx: &ActionContext) -> ActionResult {
        // 鼠标移动只更新预览，不改变状态
        ActionResult::Continue
    }

    fn on_mouse_click(&mut self, ctx: &ActionContext, button: MouseButton) -> ActionResult {
        match button {
            MouseButton::Left => {
                let point = ctx.effective_point();
                self.on_coordinate(ctx, point)
            }
            MouseButton::Right => {
                // 右键：在 SetEndpoint 状态下开始新序列，否则取消
                match self.status {
                    Status::SetStartpoint => ActionResult::Cancel,
                    Status::SetEndpoint => {
                        self.next();
                        ActionResult::Continue
                    }
                }
            }
            MouseButton::Middle => ActionResult::Continue,
        }
    }

    fn on_coordinate(&mut self, _ctx: &ActionContext, coord: Point2) -> ActionResult {
        match self.status {
            Status::SetStartpoint => {
                self.start_point = Some(coord);
                self.points.clear();
                self.points.push(coord);
                self.start_offset = 0;
                self.history.push(HistoryAction::SetStartpoint { point: coord });
                self.status = Status::SetEndpoint;
                ActionResult::Continue
            }
            Status::SetEndpoint => {
                if let Some(start) = self.start_point {
                    // 拒绝零长度线段
                    if (coord - start).norm() > 1e-6 {
                        self.history.push(HistoryAction::SetEndpoint { from: start, to: coord });
                        self.start_offset += 1;
                        
                        let result = self.trigger(start, coord);
                        
                        // 继续画线：终点变成下一条线的起点
                        self.start_point = Some(coord);
                        self.points.push(coord);
                        
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
            "C" | "CLOSE" => {
                if self.status == Status::SetEndpoint && self.points.len() >= 2 {
                    Some(self.close())
                } else {
                    None
                }
            }
            "U" | "UNDO" => {
                if self.can_undo() {
                    self.undo();
                    Some(ActionResult::Continue)
                } else {
                    None
                }
            }
            "REDO" => {
                if self.can_redo() {
                    self.redo();
                    Some(ActionResult::Continue)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn get_prompt(&self) -> &str {
        match self.status {
            Status::SetStartpoint => "指定第一点:",
            Status::SetEndpoint => {
                if self.points.len() >= 2 {
                    "指定下一点 或 [闭合(C)/放弃(U)]:"
                } else {
                    "指定下一点 或 [放弃(U)]:"
                }
            }
        }
    }

    fn get_available_commands(&self) -> Vec<&str> {
        match self.status {
            Status::SetStartpoint => vec![],
            Status::SetEndpoint => {
                let mut cmds = vec!["undo"];
                if self.points.len() >= 2 {
                    cmds.push("close");
                }
                if self.can_redo() {
                    cmds.push("redo");
                }
                cmds
            }
        }
    }

    fn get_preview(&self, ctx: &ActionContext) -> Vec<PreviewGeometry> {
        let mut previews = Vec::new();
        
        if self.status == Status::SetEndpoint {
            if let Some(start) = self.start_point {
                let end = ctx.effective_point();
                
                // 如果正交模式开启，调整终点
                let adjusted_end = if ctx.ortho_mode {
                    let dx = (end.x - start.x).abs();
                    let dy = (end.y - start.y).abs();
                    if dx > dy {
                        Point2::new(end.x, start.y)
                    } else {
                        Point2::new(start.x, end.y)
                    }
                } else {
                    end
                };
                
                let line = Line::new(start, adjusted_end);
                previews.push(PreviewGeometry::new(Geometry::Line(line)));
            }
        }
        
        previews
    }

    fn can_undo(&self) -> bool {
        self.history.can_undo()
    }

    fn can_redo(&self) -> bool {
        self.history.can_redo()
    }

    fn undo(&mut self) {
        if let Some(action) = self.history.undo() {
            match action {
                HistoryAction::SetStartpoint { .. } => {
                    self.status = Status::SetStartpoint;
                    self.start_point = None;
                    self.points.clear();
                }
                HistoryAction::SetEndpoint { from, .. } => {
                    self.start_point = Some(*from);
                    self.points.pop();
                    if self.start_offset > 0 {
                        self.start_offset -= 1;
                    }
                }
                HistoryAction::Close { from, .. } => {
                    self.start_point = Some(*from);
                    self.status = Status::SetEndpoint;
                }
                HistoryAction::Next => {
                    // 恢复之前的状态
                    if let Some(last_point) = self.points.last() {
                        self.start_point = Some(*last_point);
                        self.status = Status::SetEndpoint;
                    }
                }
            }
        }
    }

    fn redo(&mut self) {
        if let Some(action) = self.history.redo() {
            match action.clone() {
                HistoryAction::SetStartpoint { point } => {
                    self.start_point = Some(point);
                    self.points.push(point);
                    self.status = Status::SetEndpoint;
                }
                HistoryAction::SetEndpoint { to, .. } => {
                    self.start_point = Some(to);
                    self.points.push(to);
                    self.start_offset += 1;
                }
                HistoryAction::Close { .. } => {
                    self.status = Status::SetStartpoint;
                    self.start_point = None;
                    self.points.clear();
                }
                HistoryAction::Next => {
                    self.status = Status::SetStartpoint;
                    self.start_point = None;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_ctx() -> ActionContext<'static> {
        ActionContext {
            mouse_pos: Point2::new(0.0, 0.0),
            snap_pos: None,
            selected_entities: &[],
            entities: &[],
            ortho_mode: false,
            reference_point: None,
        }
    }

    #[test]
    fn test_draw_line_basic() {
        let mut action = DrawLineAction::new();
        let mut ctx = create_ctx();

        // 设置起点
        let result = action.on_coordinate(&ctx, Point2::new(0.0, 0.0));
        assert!(matches!(result, ActionResult::Continue));
        assert_eq!(action.status, Status::SetEndpoint);

        // 设置终点
        ctx.mouse_pos = Point2::new(100.0, 100.0);
        let result = action.on_coordinate(&ctx, Point2::new(100.0, 100.0));
        assert!(matches!(result, ActionResult::CreateEntities(_)));
    }

    #[test]
    fn test_draw_line_undo() {
        let mut action = DrawLineAction::new();
        let ctx = create_ctx();

        // 设置起点
        action.on_coordinate(&ctx, Point2::new(0.0, 0.0));
        assert_eq!(action.status, Status::SetEndpoint);

        // 撤销
        assert!(action.can_undo());
        action.undo();
        assert_eq!(action.status, Status::SetStartpoint);
    }
}
