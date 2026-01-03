//! 参数化设计系统
//!
//! 提供变量、约束和约束求解功能，实现类似SolidWorks的参数化设计。
//!
//! 核心组件：
//! - Variable: 可调整的参数变量
//! - Constraint: 几何元素之间的约束关系
//! - ConstraintSystem: 约束求解系统

use crate::entity::EntityId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 变量ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VariableId(pub u64);

impl VariableId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    pub fn null() -> Self {
        Self(0)
    }

    pub fn is_null(&self) -> bool {
        self.0 == 0
    }
}

/// 参数变量
///
/// 表示可调整的设计参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variable {
    /// 变量ID
    pub id: VariableId,

    /// 变量名称
    pub name: String,

    /// 当前值
    pub value: f64,

    /// 最小值约束
    pub min_value: Option<f64>,

    /// 最大值约束
    pub max_value: Option<f64>,

    /// 是否被锁定
    pub locked: bool,

    /// 变量描述
    pub description: String,
}

impl Variable {
    /// 创建新变量
    pub fn new(name: impl Into<String>, value: f64) -> Self {
        static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

        Self {
            id: VariableId(NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)),
            name: name.into(),
            value,
            min_value: None,
            max_value: None,
            locked: false,
            description: String::new(),
        }
    }

    /// 设置值（检查约束）
    pub fn set_value(&mut self, value: f64) -> Result<(), String> {
        if let Some(min) = self.min_value {
            if value < min {
                return Err(format!("Value {} is below minimum {}", value, min));
            }
        }
        if let Some(max) = self.max_value {
            if value > max {
                return Err(format!("Value {} is above maximum {}", value, max));
            }
        }
        if self.locked {
            return Err("Variable is locked".to_string());
        }

        self.value = value;
        Ok(())
    }

    /// 设置范围
    pub fn set_range(&mut self, min: Option<f64>, max: Option<f64>) {
        self.min_value = min;
        self.max_value = max;
    }

    /// 锁定/解锁变量
    pub fn set_locked(&mut self, locked: bool) {
        self.locked = locked;
    }
}

/// 约束ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConstraintId(pub u64);

impl ConstraintId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    pub fn null() -> Self {
        Self(0)
    }

    pub fn is_null(&self) -> bool {
        self.0 == 0
    }
}

/// 约束类型
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ConstraintType {
    /// 距离约束
    Distance,

    /// 角度约束
    Angle,

    /// 水平约束
    Horizontal,

    /// 垂直约束
    Vertical,

    /// 平行约束
    Parallel,

    /// 垂直约束（两条线）
    Perpendicular,

    /// 相等约束
    Equal,

    /// 共线约束
    Collinear,

    /// 共点约束
    Coincident,

    /// 固定约束
    Fixed,

    /// 对称约束
    Symmetric,
}

/// 约束目标
///
/// 指定约束作用的几何元素
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConstraintTarget {
    /// 点
    Point(EntityId),

    /// 线段
    Line(EntityId),

    /// 圆
    Circle(EntityId),

    /// 圆弧
    Arc(EntityId),

    /// 变量
    Variable(VariableId),

    /// 数值常量
    Constant(f64),
}

/// 约束定义
///
/// 表示几何元素之间的关系约束
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraint {
    /// 约束ID
    pub id: ConstraintId,

    /// 约束类型
    pub constraint_type: ConstraintType,

    /// 约束目标列表
    pub targets: Vec<ConstraintTarget>,

    /// 约束值（对于需要数值的约束）
    pub value: Option<f64>,

    /// 约束权重（用于软约束）
    pub weight: f64,

    /// 是否启用
    pub enabled: bool,

    /// 约束名称
    pub name: String,

    /// 约束描述
    pub description: String,
}

impl Constraint {
    /// 创建新约束
    pub fn new(constraint_type: ConstraintType, targets: Vec<ConstraintTarget>) -> Self {
        static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

        Self {
            id: ConstraintId(NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)),
            constraint_type,
            targets,
            value: None,
            weight: 1.0,
            enabled: true,
            name: String::new(),
            description: String::new(),
        }
    }

    /// 设置约束值
    pub fn with_value(mut self, value: f64) -> Self {
        self.value = Some(value);
        self
    }

    /// 设置权重
    pub fn with_weight(mut self, weight: f64) -> Self {
        self.weight = weight;
        self
    }

    /// 设置名称
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// 启用/禁用约束
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// 检查约束是否有效
    pub fn is_valid(&self) -> bool {
        match self.constraint_type {
            ConstraintType::Distance => self.targets.len() == 2 && self.value.is_some(),
            ConstraintType::Angle => self.targets.len() == 2 && self.value.is_some(),
            ConstraintType::Horizontal => self.targets.len() == 1,
            ConstraintType::Vertical => self.targets.len() == 1,
            ConstraintType::Parallel => self.targets.len() == 2,
            ConstraintType::Perpendicular => self.targets.len() == 2,
            ConstraintType::Equal => self.targets.len() == 2,
            ConstraintType::Collinear => self.targets.len() >= 2,
            ConstraintType::Coincident => self.targets.len() == 2,
            ConstraintType::Fixed => self.targets.len() == 1,
            ConstraintType::Symmetric => self.targets.len() == 3, // 两个对称元素 + 对称轴
        }
    }
}

/// 约束求解结果
#[derive(Debug, Clone)]
pub enum SolveResult {
    /// 成功求解
    Success,

    /// 欠约束（自由度过多）
    UnderConstrained,

    /// 过约束（冲突）
    OverConstrained,

    /// 无法收敛
    DidNotConverge,

    /// 其他错误
    Error(String),
}

/// 布尔运算类型
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum BooleanOp {
    Union = 0,
    Intersection = 1,
    Difference = 2,
    Xor = 3,
}

/// 约束系统
///
/// 管理变量、约束，并提供约束求解功能
pub struct ConstraintSystem {
    /// 变量集合
    variables: HashMap<VariableId, Variable>,

    /// 约束集合
    constraints: HashMap<ConstraintId, Constraint>,

    /// 变量到约束的映射（哪些约束使用了这个变量）
    variable_constraints: HashMap<VariableId, Vec<ConstraintId>>,

    /// 实体到约束的映射（哪些约束作用于这个实体）
    entity_constraints: HashMap<EntityId, Vec<ConstraintId>>,

    /// 求解统计
    stats: SolveStats,
}

#[derive(Debug, Clone, Default)]
pub struct SolveStats {
    /// 总求解次数
    pub solve_count: u64,

    /// 成功求解次数
    pub success_count: u64,

    /// 平均收敛步数
    pub avg_iterations: f64,

    /// 最后求解时间
    pub last_solve_time: Option<std::time::Instant>,

    /// 最后求解结果
    pub last_result: Option<SolveResult>,
}

impl ConstraintSystem {
    /// 创建新的约束系统
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            constraints: HashMap::new(),
            variable_constraints: HashMap::new(),
            entity_constraints: HashMap::new(),
            stats: SolveStats::default(),
        }
    }

    // === 变量管理 ===

    /// 添加变量
    pub fn add_variable(&mut self, variable: Variable) {
        let id = variable.id;
        self.variables.insert(id, variable);
    }

    /// 移除变量
    pub fn remove_variable(&mut self, id: &VariableId) -> Option<Variable> {
        if let Some(var) = self.variables.remove(id) {
            // 移除相关的约束引用
            if let Some(constraint_ids) = self.variable_constraints.remove(id) {
                for constraint_id in constraint_ids {
                    if let Some(constraint) = self.constraints.get_mut(&constraint_id) {
                        constraint.targets.retain(|target| {
                            !matches!(target, ConstraintTarget::Variable(vid) if vid == id)
                        });
                    }
                }
            }
            Some(var)
        } else {
            None
        }
    }

    /// 获取变量
    pub fn get_variable(&self, id: &VariableId) -> Option<&Variable> {
        self.variables.get(id)
    }

    /// 获取变量的可变引用
    pub fn get_variable_mut(&mut self, id: &VariableId) -> Option<&mut Variable> {
        self.variables.get_mut(id)
    }

    /// 设置变量值
    pub fn set_variable_value(&mut self, id: &VariableId, value: f64) -> Result<(), String> {
        if let Some(var) = self.variables.get_mut(id) {
            var.set_value(value)
        } else {
            Err(format!("Variable {:?} not found", id))
        }
    }

    /// 获取所有变量
    pub fn variables(&self) -> impl Iterator<Item = &Variable> {
        self.variables.values()
    }

    // === 约束管理 ===

    /// 添加约束
    pub fn add_constraint(&mut self, constraint: Constraint) {
        let id = constraint.id;
        let targets = constraint.targets.clone();

        self.constraints.insert(id, constraint);

        // 更新反向映射
        for target in targets {
            match target {
                ConstraintTarget::Variable(var_id) => {
                    self.variable_constraints.entry(var_id).or_insert_with(Vec::new).push(id);
                }
                ConstraintTarget::Point(entity_id) |
                ConstraintTarget::Line(entity_id) |
                ConstraintTarget::Circle(entity_id) |
                ConstraintTarget::Arc(entity_id) => {
                    self.entity_constraints.entry(entity_id).or_insert_with(Vec::new).push(id);
                }
                ConstraintTarget::Constant(_) => {} // 常量不需要映射
            }
        }
    }

    /// 移除约束
    pub fn remove_constraint(&mut self, id: &ConstraintId) -> Option<Constraint> {
        if let Some(constraint) = self.constraints.remove(id) {
            // 从反向映射中移除
            for target in &constraint.targets {
                match target {
                    ConstraintTarget::Variable(var_id) => {
                        if let Some(constraints) = self.variable_constraints.get_mut(var_id) {
                            constraints.retain(|cid| cid != id);
                        }
                    }
                    ConstraintTarget::Point(entity_id) |
                    ConstraintTarget::Line(entity_id) |
                    ConstraintTarget::Circle(entity_id) |
                    ConstraintTarget::Arc(entity_id) => {
                        if let Some(constraints) = self.entity_constraints.get_mut(entity_id) {
                            constraints.retain(|cid| cid != id);
                        }
                    }
                    ConstraintTarget::Constant(_) => {}
                }
            }
            Some(constraint)
        } else {
            None
        }
    }

    /// 获取约束
    pub fn get_constraint(&self, id: &ConstraintId) -> Option<&Constraint> {
        self.constraints.get(id)
    }

    /// 获取作用于实体的约束
    pub fn get_entity_constraints(&self, entity_id: &EntityId) -> Vec<&Constraint> {
        self.entity_constraints
            .get(entity_id)
            .map(|ids| ids.iter().filter_map(|id| self.constraints.get(id)).collect())
            .unwrap_or_default()
    }

    /// 获取使用变量的约束
    pub fn get_variable_constraints(&self, var_id: &VariableId) -> Vec<&Constraint> {
        self.variable_constraints
            .get(var_id)
            .map(|ids| ids.iter().filter_map(|id| self.constraints.get(id)).collect())
            .unwrap_or_default()
    }

    /// 获取所有约束
    pub fn constraints(&self) -> impl Iterator<Item = &Constraint> {
        self.constraints.values()
    }

    // === 约束求解 ===

    /// 求解约束系统
    ///
    /// 使用牛顿-拉夫森方法求解约束
    pub fn solve(&mut self) -> SolveResult {
        use crate::solver::{NewtonSolver, SolverParams, SolverResult};

        let start_time = std::time::Instant::now();

        // 创建求解器
        let params = SolverParams::default();
        let mut solver = NewtonSolver::from_constraint_system(self, params);

        // 执行求解
        let result = solver.solve();

        // 将结果转换并更新变量值
        let solve_result = match result {
            SolverResult::Converged => {
                // 更新变量值
                for variable in self.variables.values_mut() {
                    if let Some(new_value) = solver.get_variable_value(&variable.id) {
                        variable.value = new_value;
                    }
                }
                SolveResult::Success
            }
            SolverResult::DidNotConverge => SolveResult::DidNotConverge,
            SolverResult::UnderConstrained => SolveResult::UnderConstrained,
            SolverResult::OverConstrained => SolveResult::OverConstrained,
            SolverResult::Failed => SolveResult::Error("Solver failed".to_string()),
        };

        // 更新统计
        self.stats.solve_count += 1;
        if matches!(solve_result, SolveResult::Success) {
            self.stats.success_count += 1;
        }
        self.stats.last_solve_time = Some(start_time);
        self.stats.last_result = Some(solve_result.clone());

        solve_result
    }

    /// 评估单个约束是否满足（预留给约束求解器）
    fn _evaluate_constraint(&self, constraint: &Constraint) -> bool {
        match constraint.constraint_type {
            ConstraintType::Distance => {
                // 简化的距离约束评估
                // 实际应该计算几何元素之间的距离并与目标值比较
                true // 假设满足
            }
            ConstraintType::Horizontal => {
                // 检查线是否水平
                true // 假设满足
            }
            ConstraintType::Vertical => {
                // 检查线是否垂直
                true // 假设满足
            }
            _ => true, // 其他约束类型暂时假设满足
        }
    }

    /// 获取求解统计
    pub fn stats(&self) -> &SolveStats {
        &self.stats
    }

    /// 重置统计
    pub fn reset_stats(&mut self) {
        self.stats = SolveStats::default();
    }
}

impl Default for ConstraintSystem {
    fn default() -> Self {
        Self::new()
    }
}

/// 预定义约束构造器
pub mod constraints {
    use super::*;

    /// 创建距离约束
    pub fn distance(target1: ConstraintTarget, target2: ConstraintTarget, distance: f64) -> Constraint {
        Constraint::new(ConstraintType::Distance, vec![target1, target2])
            .with_value(distance)
            .with_name("Distance")
    }

    /// 创建角度约束
    pub fn angle(target1: ConstraintTarget, target2: ConstraintTarget, angle: f64) -> Constraint {
        Constraint::new(ConstraintType::Angle, vec![target1, target2])
            .with_value(angle)
            .with_name("Angle")
    }

    /// 创建水平约束
    pub fn horizontal(target: ConstraintTarget) -> Constraint {
        Constraint::new(ConstraintType::Horizontal, vec![target])
            .with_name("Horizontal")
    }

    /// 创建垂直约束
    pub fn vertical(target: ConstraintTarget) -> Constraint {
        Constraint::new(ConstraintType::Vertical, vec![target])
            .with_name("Vertical")
    }

    /// 创建平行约束
    pub fn parallel(target1: ConstraintTarget, target2: ConstraintTarget) -> Constraint {
        Constraint::new(ConstraintType::Parallel, vec![target1, target2])
            .with_name("Parallel")
    }

    /// 创建垂直约束（两条线）
    pub fn perpendicular(target1: ConstraintTarget, target2: ConstraintTarget) -> Constraint {
        Constraint::new(ConstraintType::Perpendicular, vec![target1, target2])
            .with_name("Perpendicular")
    }

    /// 创建相等约束
    pub fn equal(target1: ConstraintTarget, target2: ConstraintTarget) -> Constraint {
        Constraint::new(ConstraintType::Equal, vec![target1, target2])
            .with_name("Equal")
    }

    /// 创建共点约束
    pub fn coincident(target1: ConstraintTarget, target2: ConstraintTarget) -> Constraint {
        Constraint::new(ConstraintType::Coincident, vec![target1, target2])
            .with_name("Coincident")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_variable() {
        let mut var = Variable::new("length", 10.0);
        assert_eq!(var.value, 10.0);

        var.set_range(Some(0.0), Some(100.0));
        assert!(var.set_value(50.0).is_ok());
        assert!(var.set_value(-10.0).is_err());
    }

    #[test]
    fn test_constraint_system() {
        let mut system = ConstraintSystem::new();

        let var = Variable::new("width", 5.0);
        system.add_variable(var.clone());

        let constraint = constraints::distance(
            ConstraintTarget::Variable(var.id),
            ConstraintTarget::Constant(10.0),
            10.0,
        );
        system.add_constraint(constraint);

        assert_eq!(system.variables().count(), 1);
        assert_eq!(system.constraints().count(), 1);

        // 求解约束
        let result = system.solve();
        match result {
            SolveResult::Success => println!("Constraint solved successfully"),
            _ => println!("Constraint solve result: {:?}", result),
        }
    }
}
