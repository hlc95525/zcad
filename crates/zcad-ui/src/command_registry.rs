//! 命令注册表
//!
//! 参考 LibreCAD 的 RS_Commands 实现
//! 支持完整命令、快捷键、别名和 Tab 补全

use crate::action::ActionType;
use std::collections::HashMap;
use std::path::Path;

/// 命令注册表
///
/// 管理所有命令、快捷键和别名的映射
#[derive(Debug, Clone)]
pub struct CommandRegistry {
    /// 完整命令 -> ActionType
    main_commands: HashMap<String, ActionType>,
    /// 快捷键/短命令 -> ActionType
    short_commands: HashMap<String, ActionType>,
    /// 用户别名 -> 完整命令
    aliases: HashMap<String, String>,
    /// ActionType -> 完整命令（反向查找）
    action_to_command: HashMap<ActionType, String>,
}

impl CommandRegistry {
    /// 创建新的命令注册表
    pub fn new() -> Self {
        let mut registry = Self {
            main_commands: HashMap::new(),
            short_commands: HashMap::new(),
            aliases: HashMap::new(),
            action_to_command: HashMap::new(),
        };
        
        // 注册默认命令
        registry.register_defaults();
        
        registry
    }

    /// 注册默认命令
    fn register_defaults(&mut self) {
        // 绘图命令
        self.register(ActionType::DrawLine, "LINE", &["L"]);
        self.register(ActionType::DrawCircle, "CIRCLE", &["C"]);
        self.register(ActionType::DrawArc, "ARC", &["A"]);
        self.register(ActionType::DrawPolyline, "POLYLINE", &["PL", "P", "PLINE"]);
        self.register(ActionType::DrawRectangle, "RECTANGLE", &["REC", "R"]);
        self.register(ActionType::DrawPoint, "POINT", &["."]);
        self.register(ActionType::DrawText, "TEXT", &["T", "DTEXT", "MTEXT"]);
        self.register(ActionType::DrawDimension, "DIMENSION", &["DIM", "D", "DIMLINEAR", "DIMALIGNED"]);
        self.register(ActionType::DrawDimensionRadius, "DIMRADIUS", &["DRA"]);
        self.register(ActionType::DrawDimensionDiameter, "DIMDIAMETER", &["DDI"]);

        // 修改命令
        self.register(ActionType::Move, "MOVE", &["M"]);
        self.register(ActionType::Copy, "COPY", &["CO", "CP"]);
        self.register(ActionType::Rotate, "ROTATE", &["RO"]);
        self.register(ActionType::Scale, "SCALE", &["SC"]);
        self.register(ActionType::Mirror, "MIRROR", &["MI"]);
        self.register(ActionType::Erase, "ERASE", &["E", "DELETE"]);

        // 选择
        self.register(ActionType::Select, "SELECT", &["SEL"]);
    }

    /// 注册命令
    ///
    /// # 参数
    /// - `action`: ActionType
    /// - `full_cmd`: 完整命令名（如 "LINE"）
    /// - `shortcuts`: 快捷键/短命令列表（如 ["L"]）
    pub fn register(&mut self, action: ActionType, full_cmd: &str, shortcuts: &[&str]) {
        let full_cmd_upper = full_cmd.to_uppercase();
        
        // 注册完整命令
        self.main_commands.insert(full_cmd_upper.clone(), action);
        
        // 注册反向映射
        self.action_to_command.insert(action, full_cmd_upper.clone());
        
        // 注册快捷键
        for shortcut in shortcuts {
            let shortcut_upper = shortcut.to_uppercase();
            self.short_commands.insert(shortcut_upper, action);
        }
    }

    /// 查找命令对应的 ActionType
    pub fn lookup(&self, input: &str) -> Option<ActionType> {
        let input_upper = input.to_uppercase();
        
        // 1. 先查完整命令
        if let Some(&action) = self.main_commands.get(&input_upper) {
            return Some(action);
        }
        
        // 2. 再查快捷键
        if let Some(&action) = self.short_commands.get(&input_upper) {
            return Some(action);
        }
        
        // 3. 查别名
        if let Some(cmd) = self.aliases.get(&input_upper) {
            return self.main_commands.get(cmd).copied();
        }
        
        None
    }

    /// Tab 补全
    ///
    /// 返回所有以 prefix 开头的命令
    pub fn complete(&self, prefix: &str) -> Vec<String> {
        let prefix_upper = prefix.to_uppercase();
        let mut results: Vec<String> = self.main_commands
            .keys()
            .filter(|cmd| cmd.starts_with(&prefix_upper))
            .cloned()
            .collect();
        
        results.sort();
        results
    }

    /// 获取命令的完整名称
    pub fn get_command_name(&self, action: ActionType) -> Option<&str> {
        self.action_to_command.get(&action).map(|s| s.as_str())
    }

    /// 添加用户别名
    pub fn add_alias(&mut self, alias: &str, command: &str) {
        let alias_upper = alias.to_uppercase();
        let command_upper = command.to_uppercase();
        
        // 不允许覆盖现有命令
        if self.main_commands.contains_key(&alias_upper) {
            return;
        }
        
        // 确保目标命令存在
        if self.main_commands.contains_key(&command_upper) {
            self.aliases.insert(alias_upper, command_upper);
        }
    }

    /// 移除别名
    pub fn remove_alias(&mut self, alias: &str) {
        let alias_upper = alias.to_uppercase();
        self.aliases.remove(&alias_upper);
    }

    /// 从文件加载别名
    ///
    /// 文件格式：每行 "alias\tcommand"，以 # 开头的行是注释
    pub fn load_aliases(&mut self, path: &Path) -> Result<(), std::io::Error> {
        let content = std::fs::read_to_string(path)?;
        
        for line in content.lines() {
            let line = line.trim();
            
            // 跳过注释和空行
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            
            // 解析 "alias\tcommand" 或 "alias command"
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                self.add_alias(parts[0], parts[1]);
            }
        }
        
        Ok(())
    }

    /// 保存别名到文件
    pub fn save_aliases(&self, path: &Path) -> Result<(), std::io::Error> {
        let mut content = String::new();
        content.push_str("# ZCAD Command Aliases\n");
        content.push_str("# Format: alias\\tcommand\n\n");
        
        for (alias, command) in &self.aliases {
            content.push_str(&format!("{}\t{}\n", alias.to_lowercase(), command.to_lowercase()));
        }
        
        std::fs::write(path, content)
    }

    /// 获取所有命令列表
    pub fn get_all_commands(&self) -> Vec<(&str, ActionType)> {
        self.main_commands
            .iter()
            .map(|(cmd, &action)| (cmd.as_str(), action))
            .collect()
    }

    /// 获取所有快捷键
    pub fn get_all_shortcuts(&self) -> Vec<(&str, ActionType)> {
        self.short_commands
            .iter()
            .map(|(cmd, &action)| (cmd.as_str(), action))
            .collect()
    }
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lookup() {
        let registry = CommandRegistry::new();
        
        // 完整命令
        assert_eq!(registry.lookup("LINE"), Some(ActionType::DrawLine));
        assert_eq!(registry.lookup("line"), Some(ActionType::DrawLine));
        
        // 快捷键
        assert_eq!(registry.lookup("L"), Some(ActionType::DrawLine));
        assert_eq!(registry.lookup("l"), Some(ActionType::DrawLine));
        
        // 不存在的命令
        assert_eq!(registry.lookup("NOTEXIST"), None);
    }

    #[test]
    fn test_complete() {
        let registry = CommandRegistry::new();
        
        let completions = registry.complete("DIM");
        assert!(completions.contains(&"DIMENSION".to_string()));
        assert!(completions.contains(&"DIMRADIUS".to_string()));
        assert!(completions.contains(&"DIMDIAMETER".to_string()));
    }

    #[test]
    fn test_alias() {
        let mut registry = CommandRegistry::new();
        
        registry.add_alias("LL", "LINE");
        assert_eq!(registry.lookup("LL"), Some(ActionType::DrawLine));
        
        registry.remove_alias("LL");
        assert_eq!(registry.lookup("LL"), None);
    }
}
