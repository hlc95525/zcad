//! 主菜单

use crate::state::UiState;

/// 渲染主菜单
#[allow(deprecated)] // egui::menu::bar 在新版本中已弃用，但功能仍正常
pub fn show_main_menu(ctx: &egui::Context, ui_state: &mut UiState) {
    egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
        egui::menu::bar(ui, |ui| {
            // 文件菜单
            ui.menu_button("File", |ui| {
                if ui.button("New             Ctrl+N").clicked() {
                    ui.close();
                }
                if ui.button("Open            Ctrl+O").clicked() {
                    ui.close();
                }
                ui.separator();
                if ui.button("Save            Ctrl+S").clicked() {
                    ui.close();
                }
                if ui.button("Save As     Ctrl+Shift+S").clicked() {
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
                    // TODO: 退出程序
                }
            });

            // 编辑菜单
            ui.menu_button("Edit", |ui| {
                if ui.button("Undo            Ctrl+Z").clicked() {
                    ui.close();
                }
                if ui.button("Redo            Ctrl+Y").clicked() {
                    ui.close();
                }
                ui.separator();
                if ui.button("Cut             Ctrl+X").clicked() {
                    ui.close();
                }
                if ui.button("Copy            Ctrl+C").clicked() {
                    ui.close();
                }
                if ui.button("Paste           Ctrl+V").clicked() {
                    ui.close();
                }
                if ui.button("Delete          Del").clicked() {
                    ui.close();
                }
                ui.separator();
                if ui.button("Select All      Ctrl+A").clicked() {
                    ui.close();
                }
            });

            // 视图菜单
            ui.menu_button("View", |ui| {
                if ui.button("Zoom Extents    Z").clicked() {
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
                    ui.close();
                }
                if ui.button("Copy            CO").clicked() {
                    ui.close();
                }
                if ui.button("Rotate          RO").clicked() {
                    ui.close();
                }
                if ui.button("Scale           SC").clicked() {
                    ui.close();
                }
                if ui.button("Mirror          MI").clicked() {
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
