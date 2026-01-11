//! ZCAD 用户界面
//!
//! 基于egui的即时模式GUI。

pub mod action;
pub mod actions;
pub mod command_line;
pub mod command_registry;
pub mod layers_panel;
pub mod main_menu;
pub mod properties_panel;
pub mod state;
pub mod toolbar;

pub use action::{Action, ActionContext, ActionResult, ActionType, MouseButton, PreviewGeometry};
pub use actions::create_action;
pub use command_registry::CommandRegistry;
pub use state::{DrawingTool, EditState, SnapMode, SnapState, UiState};

