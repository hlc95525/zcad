//! ZCAD 核心几何引擎
//!
//! 提供2D/3D几何图元、变换操作和空间查询功能。
//!
//! # 架构设计
//!
//! 采用 Entity-Component 模式：
//! - `Entity`: 唯一标识符
//! - `Geometry`: 几何数据（点、线、圆等）
//! - `Properties`: 视觉属性（颜色、线型、图层）
//!
//! # 示例
//!
//! ```rust
//! use zcad_core::prelude::*;
//!
//! // 创建一条线段
//! let line = Line::new(Point2::origin(), Point2::new(100.0, 50.0));
//!
//! // 计算长度
//! println!("Length: {}", line.length());
//! ```

pub mod async_core;
pub mod buffer;
pub mod entity;
pub mod geometry;
pub mod history;
pub mod input_parser;
pub mod layer;
pub mod math;
pub mod parametric;
pub mod properties;
pub mod snap;
pub mod solver;
pub mod spatial;
pub mod transform;
pub mod version_control;

pub mod prelude {
    //! 常用类型的便捷导入
    pub use crate::async_core::{AsyncCore, Message, MessageBus};
    pub use crate::buffer::{DoubleBufferedEntities, EntityBuffer};
    pub use crate::entity::{Entity, EntityId};
    pub use crate::geometry::{Arc, Circle, Geometry, Line, Point, Polyline, Text, TextAlignment};
    pub use crate::history::{HistoryTree, Operation, OperationId};
    pub use crate::layer::Layer;
    pub use crate::input_parser::{InputParser, InputValue, ParseError};
    pub use crate::math::{Point2, Point3, Vector2, Vector3};
    pub use crate::parametric::{Constraint, ConstraintSystem, Variable};
    pub use crate::properties::{Color, LineType, Properties};
    pub use crate::snap::{SnapConfig, SnapEngine, SnapMask, SnapPoint, SnapType};
    pub use crate::solver::NewtonSolver;
    pub use crate::transform::Transform2D;
    pub use crate::version_control::{VersionControl, Commit, Branch};
}

