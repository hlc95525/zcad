//! 绘制多段线 Action

use crate::action::{
    Action, ActionContext, ActionHistory, ActionResult, ActionType, MouseButton, PreviewGeometry,
};
use zcad_core::geometry::{Geometry, Line, Polyline, PolylineVertex};
use zcad_core::math::Point2;

/// 多段线绘制状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Status {
    /// 等待第一点
    SetFirstPoint,
    /// 等待下一点
    SetNextPoint,
}

/// 历史动作
#[derive(Debug, Clone)]
enum HistoryAction {
    AddPoint { point: Point2 },
    Close,
}

/// 绘制多段线 Action
pub struct DrawPolylineAction {
    status: Status,
    vertices: Vec<Point2>,
    history: ActionHistory<HistoryAction>,
}

impl DrawPolylineAction {
    pub fn new() -> Self {
        Self {
            status: Status::SetFirstPoint,
            vertices: Vec::new(),
            history: ActionHistory::new(),
        }
    }

    fn close(&mut self) -> ActionResult {
        if self.vertices.len() >= 3 {
            let vertices: Vec<PolylineVertex> = self.vertices
                .iter()
                .map(|&p| PolylineVertex::new(p))
                .collect();
            let polyline = Polyline::new(vertices, true);
            self.reset();
            return ActionResult::CreateEntities(vec![Geometry::Polyline(polyline)]);
        }
        ActionResult::Continue
    }

    fn finish(&mut self) -> ActionResult {
        if self.vertices.len() >= 2 {
            let vertices: Vec<PolylineVertex> = self.vertices
                .iter()
                .map(|&p| PolylineVertex::new(p))
                .collect();
            let polyline = Polyline::new(vertices, false);
            self.reset();
            return ActionResult::CreateEntities(vec![Geometry::Polyline(polyline)]);
        }
        ActionResult::Continue
    }
}

impl Default for DrawPolylineAction {
    fn default() -> Self {
        Self::new()
    }
}

impl Action for DrawPolylineAction {
    fn action_type(&self) -> ActionType {
        ActionType::DrawPolyline
    }

    fn reset(&mut self) {
        self.status = Status::SetFirstPoint;
        self.vertices.clear();
        self.history.clear();
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
                // 右键结束多段线（不闭合）
                if self.status == Status::SetNextPoint {
                    self.finish()
                } else {
                    ActionResult::Cancel
                }
            }
            MouseButton::Middle => ActionResult::Continue,
        }
    }

    fn on_coordinate(&mut self, _ctx: &ActionContext, coord: Point2) -> ActionResult {
        match self.status {
            Status::SetFirstPoint => {
                self.vertices.push(coord);
                self.history.push(HistoryAction::AddPoint { point: coord });
                self.status = Status::SetNextPoint;
                ActionResult::Continue
            }
            Status::SetNextPoint => {
                // 检查是否与上一个点重合
                if let Some(&last) = self.vertices.last() {
                    if (coord - last).norm() < 1e-6 {
                        return ActionResult::Continue;
                    }
                }
                self.vertices.push(coord);
                self.history.push(HistoryAction::AddPoint { point: coord });
                ActionResult::Continue
            }
        }
    }

    fn on_command(&mut self, _ctx: &ActionContext, cmd: &str) -> Option<ActionResult> {
        let cmd_upper = cmd.to_uppercase();
        
        match cmd_upper.as_str() {
            "C" | "CLOSE" => {
                if self.vertices.len() >= 3 {
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
            _ => None,
        }
    }

    fn get_prompt(&self) -> &str {
        match self.status {
            Status::SetFirstPoint => "指定起点:",
            Status::SetNextPoint => {
                if self.vertices.len() >= 3 {
                    "指定下一点 或 [闭合(C)/放弃(U)]:"
                } else {
                    "指定下一点 或 [放弃(U)]:"
                }
            }
        }
    }

    fn get_available_commands(&self) -> Vec<&str> {
        match self.status {
            Status::SetFirstPoint => vec![],
            Status::SetNextPoint => {
                let mut cmds = vec!["undo"];
                if self.vertices.len() >= 3 {
                    cmds.push("close");
                }
                cmds
            }
        }
    }

    fn get_preview(&self, ctx: &ActionContext) -> Vec<PreviewGeometry> {
        let mut previews = Vec::new();
        
        // 已确定的线段
        for i in 0..self.vertices.len().saturating_sub(1) {
            let line = Line::new(self.vertices[i], self.vertices[i + 1]);
            previews.push(PreviewGeometry::new(Geometry::Line(line)));
        }
        
        // 当前正在绘制的线段
        if self.status == Status::SetNextPoint {
            if let Some(&last) = self.vertices.last() {
                let current = ctx.effective_point();
                let line = Line::new(last, current);
                previews.push(PreviewGeometry::new(Geometry::Line(line)));
            }
        }
        
        previews
    }

    fn can_undo(&self) -> bool {
        self.history.can_undo() && self.vertices.len() > 1
    }

    fn undo(&mut self) {
        if self.vertices.len() > 1 {
            self.vertices.pop();
            self.history.undo();
        } else if self.vertices.len() == 1 {
            self.vertices.pop();
            self.status = Status::SetFirstPoint;
            self.history.undo();
        }
    }
}
