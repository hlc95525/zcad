//! 命令行界面

use crate::state::{Command, EditState, UiState};

/// 渲染命令行
pub fn show_command_line(ctx: &egui::Context, ui_state: &mut UiState) -> Option<Command> {
    let mut command = None;

    egui::TopBottomPanel::bottom("command_line")
        .resizable(false)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                // 状态消息
                ui.label(&ui_state.status_message);

                ui.separator();

                // 命令输入
                // 根据当前状态显示不同的提示
                let hint_text = if let EditState::Drawing { expected_input, .. } = &ui_state.edit_state {
                    if let Some(input_type) = expected_input {
                        input_type.hint()
                    } else {
                        "Enter command or data..."
                    }
                } else {
                    "Enter command..."
                };

                ui.label("Command:");

                let response = ui.add(
                    egui::TextEdit::singleline(&mut ui_state.command_input)
                        .desired_width(300.0)
                        .hint_text(hint_text),
                );

                // 自动聚焦
                if ui_state.should_focus_command_line {
                    response.request_focus();
                    ui_state.should_focus_command_line = false;
                }

                // 回车执行命令
                if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    let input = std::mem::take(&mut ui_state.command_input);
                    command = ui_state.execute_command(&input);
                    response.request_focus();
                }

                // 坐标显示
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!(
                        "X: {:>10.4}  Y: {:>10.4}",
                        ui_state.mouse_world_pos.x, ui_state.mouse_world_pos.y
                    ));

                    ui.separator();

                    // 模式指示器
                    if ui_state.ortho_mode {
                        ui.label(egui::RichText::new("ORTHO").color(egui::Color32::GREEN));
                    }

                    if ui_state.snap_mode.endpoint {
                        ui.label(egui::RichText::new("SNAP").color(egui::Color32::YELLOW));
                    }
                });
            });
        });

    command
}

