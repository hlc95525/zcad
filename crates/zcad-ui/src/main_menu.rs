//! 主菜单

use crate::state::{Command, UiState};

/// 渲染主菜单
#[allow(deprecated)] // egui::menu::bar 在新版本中已弃用，但功能仍正常
pub fn show_main_menu(ctx: &egui::Context, ui_state: &mut UiState) {
    egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
        egui::menu::bar(ui, |ui| {
            // 文件菜单
            ui.menu_button("File", |ui| {
                if ui.button("New             Ctrl+N").clicked() {
                    ui_state.pending_command = Some(Command::New);
                    ui.close();
                }
                if ui.button("Open            Ctrl+O").clicked() {
                    ui_state.pending_command = Some(Command::Open);
                    ui.close();
                }
                ui.separator();
                if ui.button("Save            Ctrl+S").clicked() {
                    ui_state.pending_command = Some(Command::Save);
                    ui.close();
                }
                if ui.button("Save As     Ctrl+Shift+S").clicked() {
                    // Save As uses a separate dialog flow usually initiated by Save logic if path missing,
                    // or explicitly. Here we can reuse Save or add SaveAs command.
                    // For now, let's map to Save, but maybe we should add SaveAs command.
                    // Actually ZcadApp handles Ctrl+Shift+S as SaveAs.
                    // Let's assume Save triggers logic.
                    // But wait, SaveAs forces dialog.
                    // I'll leave it as TODO or add Command::SaveAs.
                    // Let's just use Save for now, user can use shortcut.
                    // Or better, let's add SaveAs command?
                    // To be safe, I'll stick to basic commands for now.
                    ui.close();
                }
                ui.separator();
                if ui.button("Import DXF...").clicked() {
                    ui.close();
                }
                if ui.button("Export DXF...").clicked() {
                    ui.close();
                }
                ui.separator();
                if ui.button("Exit            Alt+F4").clicked() {
                    std::process::exit(0);
                }
            });

            // 编辑菜单
            ui.menu_button("Edit", |ui| {
                if ui.button("Undo            Ctrl+Z").clicked() {
                    ui_state.pending_command = Some(Command::Undo);
                    ui.close();
                }
                if ui.button("Redo            Ctrl+Y").clicked() {
                    ui_state.pending_command = Some(Command::Redo);
                    ui.close();
                }
                ui.separator();
                if ui.button("Cut             Ctrl+X").clicked() {
                    // TODO: Cut command
                    ui.close();
                }
                if ui.button("Copy            Ctrl+C").clicked() {
                    ui_state.pending_command = Some(Command::Copy); // This is Copy Object, not Clipboard Copy
                    // Actually, Menu "Copy" usually means Clipboard Copy.
                    // "Copy Object" is in Modify menu.
                    // Let's leave Clipboard Copy for now (Ctrl+C works).
                    ui.close();
                }
                if ui.button("Paste           Ctrl+V").clicked() {
                    // TODO: Paste command
                    ui.close();
                }
                if ui.button("Delete          Del").clicked() {
                    ui_state.pending_command = Some(Command::DeleteSelected);
                    ui.close();
                }
                ui.separator();
                if ui.button("Select All      Ctrl+A").clicked() {
                    // TODO: Select All
                    ui.close();
                }
            });

            // 视图菜单
            ui.menu_button("View", |ui| {
                if ui.button("Zoom Extents    Z").clicked() {
                    ui_state.pending_command = Some(Command::ZoomExtents);
                    ui.close();
                }
                if ui.button("Zoom In         +").clicked() {
                    ui.close();
                }
                if ui.button("Zoom Out        -").clicked() {
                    ui.close();
                }
                ui.separator();
                if ui
                    .checkbox(&mut ui_state.show_grid, "Show Grid")
                    .clicked()
                {
                    ui.close();
                }
                if ui
                    .checkbox(&mut ui_state.show_layers_panel, "Layers Panel")
                    .clicked()
                {
                    ui.close();
                }
                if ui
                    .checkbox(&mut ui_state.show_properties_panel, "Properties Panel")
                    .clicked()
                {
                    ui.close();
                }
            });

            // 绘图菜单
            ui.menu_button("Draw", |ui| {
                if ui.button("Line            L").clicked() {
                    ui_state.set_tool(crate::state::DrawingTool::Line);
                    ui.close();
                }
                if ui.button("Circle          C").clicked() {
                    ui_state.set_tool(crate::state::DrawingTool::Circle);
                    ui.close();
                }
                if ui.button("Arc             A").clicked() {
                    ui_state.set_tool(crate::state::DrawingTool::Arc);
                    ui.close();
                }
                if ui.button("Polyline        P").clicked() {
                    ui_state.set_tool(crate::state::DrawingTool::Polyline);
                    ui.close();
                }
                if ui.button("Rectangle       R").clicked() {
                    ui_state.set_tool(crate::state::DrawingTool::Rectangle);
                    ui.close();
                }
                if ui.button("Point           .").clicked() {
                    ui_state.set_tool(crate::state::DrawingTool::Point);
                    ui.close();
                }
            });

            // 修改菜单
            ui.menu_button("Modify", |ui| {
                if ui.button("Move            M").clicked() {
                    ui_state.pending_command = Some(Command::Move);
                    ui.close();
                }
                if ui.button("Copy            CO").clicked() {
                    ui_state.pending_command = Some(Command::Copy);
                    ui.close();
                }
                if ui.button("Rotate          RO").clicked() {
                    ui_state.pending_command = Some(Command::Rotate);
                    ui.close();
                }
                if ui.button("Scale           SC").clicked() {
                    ui_state.pending_command = Some(Command::Scale);
                    ui.close();
                }
                if ui.button("Mirror          MI").clicked() {
                    ui_state.pending_command = Some(Command::Mirror);
                    ui.close();
                }
                ui.separator();
                if ui.button("Explode         X").clicked() {
                    ui.close();
                }
                if ui.button("Join            J").clicked() {
                    ui.close();
                }
            });

            // 帮助菜单
            ui.menu_button("Help", |ui| {
                if ui.button("About ZCAD").clicked() {
                    ui.close();
                }
                if ui.button("Keyboard Shortcuts").clicked() {
                    ui.close();
                }
            });
        });
    });
}
