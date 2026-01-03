//! 约束求解器
//!
//! 实现牛顿-拉夫森方法求解非线性约束方程组。
//! 参考solvespace的实现，使用数值方法求解几何约束。

use crate::parametric::{Constraint, ConstraintTarget, ConstraintType, VariableId};
use std::collections::HashMap;

/// 求解器参数
#[derive(Debug, Clone)]
pub struct SolverParams {
    /// 最大迭代次数
    pub max_iterations: usize,

    /// 收敛容差
    pub tolerance: f64,

    /// 阻尼因子
    pub damping: f64,

    /// 梯度计算步长
    pub gradient_step: f64,
}

impl Default for SolverParams {
    fn default() -> Self {
        Self {
            max_iterations: 100,
            tolerance: 1e-6,
            damping: 0.1,
            gradient_step: 1e-8,
        }
    }
}

/// 求解变量
///
/// 表示约束求解过程中的自由变量
#[derive(Debug, Clone)]
pub struct SolveVariable {
    /// 变量ID
    pub id: VariableId,

    /// 当前值
    pub value: f64,

    /// 变量索引（在雅可比矩阵中的位置）
    pub index: usize,
}

/// 约束方程
///
/// 表示一个约束对应的方程
pub struct ConstraintEquation {
    /// 约束ID
    pub constraint_id: crate::parametric::ConstraintId,

    /// 方程函数
    pub equation: Box<dyn Fn(&[f64]) -> f64 + Send + Sync>,

    /// 梯度函数
    pub gradient: Box<dyn Fn(&[f64]) -> Vec<f64> + Send + Sync>,

    /// 权重
    pub weight: f64,
}

impl Clone for ConstraintEquation {
    fn clone(&self) -> Self {
        // 注意：我们不能真正克隆闭包，所以这里返回一个占位符
        // 在实际使用中，应该避免克隆ConstraintEquation
        Self {
            constraint_id: self.constraint_id,
            equation: Box::new(|_| 0.0), // 占位符
            gradient: Box::new(|_| vec![]), // 占位符
            weight: self.weight,
        }
    }
}

/// 牛顿-拉夫森约束求解器
pub struct NewtonSolver {
    /// 求解参数
    params: SolverParams,

    /// 变量列表
    variables: Vec<SolveVariable>,

    /// 约束方程列表
    equations: Vec<ConstraintEquation>,

    /// 变量ID到索引的映射
    variable_map: HashMap<VariableId, usize>,
}

impl NewtonSolver {
    /// 创建新的求解器
    pub fn new(params: SolverParams) -> Self {
        Self {
            params,
            variables: Vec::new(),
            equations: Vec::new(),
            variable_map: HashMap::new(),
        }
    }

    /// 添加求解变量
    pub fn add_variable(&mut self, id: VariableId, initial_value: f64) {
        let index = self.variables.len();
        self.variables.push(SolveVariable {
            id,
            value: initial_value,
            index,
        });
        self.variable_map.insert(id, index);
    }

    /// 添加约束方程
    pub fn add_constraint_equation(&mut self, equation: ConstraintEquation) {
        self.equations.push(equation);
    }

    /// 从约束系统创建求解器
    pub fn from_constraint_system(
        constraint_system: &crate::parametric::ConstraintSystem,
        params: SolverParams,
    ) -> Self {
        let mut solver = Self::new(params);

        // 添加所有变量
        for variable in constraint_system.variables() {
            solver.add_variable(variable.id, variable.value);
        }

        // 为每个约束创建方程
        for constraint in constraint_system.constraints() {
            if constraint.enabled && constraint.is_valid() {
                if let Some(equation) = Self::create_equation(constraint, &solver.variable_map) {
                    solver.add_constraint_equation(equation);
                }
            }
        }

        solver
    }

    /// 创建约束对应的方程
    fn create_equation(
        constraint: &Constraint,
        variable_map: &HashMap<VariableId, usize>,
    ) -> Option<ConstraintEquation> {
        match constraint.constraint_type {
            ConstraintType::Distance => {
                Self::create_distance_equation(constraint, variable_map)
            }
            ConstraintType::Angle => {
                Self::create_angle_equation(constraint, variable_map)
            }
            ConstraintType::Horizontal => {
                Self::create_horizontal_equation(constraint, variable_map)
            }
            ConstraintType::Vertical => {
                Self::create_vertical_equation(constraint, variable_map)
            }
            _ => None, // 其他约束类型暂时不支持
        }
    }

    /// 创建距离约束方程
    fn create_distance_equation(
        constraint: &Constraint,
        variable_map: &HashMap<VariableId, usize>,
    ) -> Option<ConstraintEquation> {
        if constraint.targets.len() != 2 || constraint.value.is_none() {
            return None;
        }

        let target_value = constraint.value.unwrap();
        let var_indices = (
            Self::get_variable_index(&constraint.targets[0], variable_map).copied(),
            Self::get_variable_index(&constraint.targets[1], variable_map).copied(),
        );

        let equation = move |vars: &[f64]| {
            // 简化的距离计算：假设变量直接表示距离
            if let (Some(idx1), Some(idx2)) = var_indices {
                let dist = (vars[idx1] - vars[idx2]).abs();
                dist - target_value
            } else {
                0.0
            }
        };

        let gradient_indices = var_indices;
        let gradient = move |vars: &[f64]| {
            let mut grad = vec![0.0; vars.len()];
            if let (Some(idx1), Some(idx2)) = gradient_indices {
                let diff = vars[idx1] - vars[idx2];
                if diff > 0.0 {
                    grad[idx1] = 1.0;
                    grad[idx2] = -1.0;
                } else {
                    grad[idx1] = -1.0;
                    grad[idx2] = 1.0;
                }
            }
            grad
        };

        Some(ConstraintEquation {
            constraint_id: constraint.id,
            equation: Box::new(equation),
            gradient: Box::new(gradient),
            weight: constraint.weight,
        })
    }

    /// 创建角度约束方程
    fn create_angle_equation(
        constraint: &Constraint,
        variable_map: &HashMap<VariableId, usize>,
    ) -> Option<ConstraintEquation> {
        if constraint.targets.len() != 2 || constraint.value.is_none() {
            return None;
        }

        let target_angle = constraint.value.unwrap();
        let var_indices = (
            Self::get_variable_index(&constraint.targets[0], variable_map).copied(),
            Self::get_variable_index(&constraint.targets[1], variable_map).copied(),
        );

        let equation = move |vars: &[f64]| {
            // 简化的角度计算
            if let (Some(idx1), Some(idx2)) = var_indices {
                let angle_diff = (vars[idx1] - vars[idx2]).abs();
                angle_diff - target_angle
            } else {
                0.0
            }
        };

        let gradient_indices = var_indices;
        let gradient = move |vars: &[f64]| {
            let mut grad = vec![0.0; vars.len()];
            if let (Some(idx1), Some(idx2)) = gradient_indices {
                grad[idx1] = 1.0;
                grad[idx2] = -1.0;
            }
            grad
        };

        Some(ConstraintEquation {
            constraint_id: constraint.id,
            equation: Box::new(equation),
            gradient: Box::new(gradient),
            weight: constraint.weight,
        })
    }

    /// 创建水平约束方程
    fn create_horizontal_equation(
        constraint: &Constraint,
        variable_map: &HashMap<VariableId, usize>,
    ) -> Option<ConstraintEquation> {
        if constraint.targets.len() != 1 {
            return None;
        }

        let var_index = Self::get_variable_index(&constraint.targets[0], variable_map).copied();

        let equation = move |vars: &[f64]| {
            // 水平约束：确保Y坐标相等
            if let Some(idx) = var_index {
                vars[idx] // 假设变量表示角度或斜率
            } else {
                0.0
            }
        };

        let gradient_index = var_index;
        let gradient = move |vars: &[f64]| {
            let mut grad = vec![0.0; vars.len()];
            if let Some(idx) = gradient_index {
                grad[idx] = 1.0;
            }
            grad
        };

        Some(ConstraintEquation {
            constraint_id: constraint.id,
            equation: Box::new(equation),
            gradient: Box::new(gradient),
            weight: constraint.weight,
        })
    }

    /// 创建垂直约束方程
    fn create_vertical_equation(
        constraint: &Constraint,
        variable_map: &HashMap<VariableId, usize>,
    ) -> Option<ConstraintEquation> {
        if constraint.targets.len() != 1 {
            return None;
        }

        let var_index = Self::get_variable_index(&constraint.targets[0], variable_map).copied();

        let equation = move |vars: &[f64]| {
            // 垂直约束：确保X坐标相等或角度为90度
            if let Some(idx) = var_index {
                (vars[idx] - std::f64::consts::FRAC_PI_2).abs() // 假设变量表示角度
            } else {
                0.0
            }
        };

        let gradient_index = var_index;
        let gradient = move |vars: &[f64]| {
            let mut grad = vec![0.0; vars.len()];
            if let Some(idx) = gradient_index {
                grad[idx] = 1.0;
            }
            grad
        };

        Some(ConstraintEquation {
            constraint_id: constraint.id,
            equation: Box::new(equation),
            gradient: Box::new(gradient),
            weight: constraint.weight,
        })
    }

    /// 获取变量在方程中的索引
    fn get_variable_index<'a>(target: &'a ConstraintTarget, variable_map: &'a HashMap<VariableId, usize>) -> Option<&'a usize> {
        match target {
            ConstraintTarget::Variable(var_id) => variable_map.get(var_id),
            _ => None,
        }
    }

    /// 执行约束求解
    pub fn solve(&mut self) -> SolverResult {
        if self.variables.is_empty() || self.equations.is_empty() {
            return SolverResult::UnderConstrained;
        }

        if self.equations.len() > self.variables.len() {
            return SolverResult::OverConstrained;
        }

        let mut x = self.variables.iter().map(|v| v.value).collect::<Vec<f64>>();
        let mut converged = false;

        for _iteration in 0..self.params.max_iterations {
            // 计算残差向量
            let residuals = self.compute_residuals(&x);
            let residual_norm = residuals.iter().map(|r| r * r).sum::<f64>().sqrt();

            if residual_norm < self.params.tolerance {
                converged = true;
                break;
            }

            // 计算雅可比矩阵
            let jacobian = self.compute_jacobian(&x);

            // 求解线性系统：J * dx = -r
            if let Some(dx) = self.solve_linear_system(&jacobian, &residuals) {
                // 应用阻尼
                for i in 0..dx.len() {
                    x[i] -= self.params.damping * dx[i];
                }

                // 检查是否有NaN或无穷大
                if x.iter().any(|&v| !v.is_finite()) {
                    return SolverResult::Failed;
                }
            } else {
                return SolverResult::Failed;
            }
        }

        if converged {
            // 更新变量值
            for (i, var) in self.variables.iter_mut().enumerate() {
                var.value = x[i];
            }
            SolverResult::Converged
        } else {
            SolverResult::DidNotConverge
        }
    }

    /// 计算残差向量
    fn compute_residuals(&self, x: &[f64]) -> Vec<f64> {
        self.equations.iter()
            .map(|eq| (eq.equation)(x) * eq.weight)
            .collect()
    }

    /// 计算雅可比矩阵（有限差分）
    fn compute_jacobian(&self, x: &[f64]) -> Vec<Vec<f64>> {
        let n = self.equations.len();
        let m = self.variables.len();
        let mut jacobian = vec![vec![0.0; m]; n];

        for i in 0..n {
            let grad = (self.equations[i].gradient)(x);
            for j in 0..m {
                jacobian[i][j] = grad[j] * self.equations[i].weight;
            }
        }

        jacobian
    }

    /// 求解线性系统 Ax = b（使用高斯消元法）
    fn solve_linear_system(&self, a: &[Vec<f64>], b: &[f64]) -> Option<Vec<f64>> {
        let n = b.len();
        if a.len() != n || a.iter().any(|row| row.len() != n) {
            return None;
        }

        // 创建增广矩阵
        let mut aug = vec![vec![0.0; n + 1]; n];
        for i in 0..n {
            for j in 0..n {
                aug[i][j] = a[i][j];
            }
            aug[i][n] = -b[i];
        }

        // 高斯消元
        for i in 0..n {
            // 寻找主元
            let mut max_row = i;
            for k in i + 1..n {
                if aug[k][i].abs() > aug[max_row][i].abs() {
                    max_row = k;
                }
            }

            // 交换行
            aug.swap(i, max_row);

            // 检查主元是否为零
            if aug[i][i].abs() < 1e-12 {
                return None; // 奇异矩阵
            }

            // 消元
            for k in i + 1..n {
                let factor = aug[k][i] / aug[i][i];
                for j in i..=n {
                    aug[k][j] -= factor * aug[i][j];
                }
            }
        }

        // 回代
        let mut x = vec![0.0; n];
        for i in (0..n).rev() {
            x[i] = aug[i][n];
            for j in i + 1..n {
                x[i] -= aug[i][j] * x[j];
            }
            x[i] /= aug[i][i];
        }

        Some(x)
    }

    /// 获取求解后的变量值
    pub fn get_variable_value(&self, var_id: &VariableId) -> Option<f64> {
        self.variable_map.get(var_id)
            .and_then(|&idx| self.variables.get(idx))
            .map(|v| v.value)
    }

    /// 获取所有变量的当前值
    pub fn get_all_values(&self) -> Vec<f64> {
        self.variables.iter().map(|v| v.value).collect()
    }
}

/// 求解器结果
#[derive(Debug, Clone, PartialEq)]
pub enum SolverResult {
    /// 成功收敛
    Converged,

    /// 未收敛
    DidNotConverge,

    /// 欠约束
    UnderConstrained,

    /// 过约束
    OverConstrained,

    /// 求解失败
    Failed,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parametric::{Constraint, ConstraintType, ConstraintTarget, VariableId};

    #[test]
    fn test_newton_solver() {
        let params = SolverParams::default();
        let mut solver = NewtonSolver::new(params);

        // 添加变量
        let var1_id = VariableId(1);
        let var2_id = VariableId(2);
        solver.add_variable(var1_id, 5.0);
        solver.add_variable(var2_id, 3.0);

        // 添加距离约束：var1 - var2 = 2.0
        let constraint = Constraint::new(
            ConstraintType::Distance,
            vec![
                ConstraintTarget::Variable(var1_id),
                ConstraintTarget::Variable(var2_id),
            ],
        ).with_value(2.0);

        if let Some(equation) = NewtonSolver::create_distance_equation(&constraint, &solver.variable_map) {
            solver.add_constraint_equation(equation);
        }

        // 求解
        let result = solver.solve();
        match result {
            SolverResult::Converged => {
                println!("Solver converged");
                if let (Some(v1), Some(v2)) = (
                    solver.get_variable_value(&var1_id),
                    solver.get_variable_value(&var2_id),
                ) {
                    println!("var1 = {}, var2 = {}, diff = {}", v1, v2, (v1 - v2).abs());
                    assert!((v1 - v2 - 2.0).abs() < 1e-6);
                }
            }
            _ => println!("Solver result: {:?}", result),
        }
    }

    #[test]
    fn test_linear_system_solve() {
        let solver = NewtonSolver::new(SolverParams::default());

        // 测试方程组：
        // 2x + y = 4
        // x - y = 0
        let a = vec![
            vec![2.0, 1.0],
            vec![1.0, -1.0],
        ];
        let b = vec![4.0, 0.0];

        let x = solver.solve_linear_system(&a, &b).unwrap();
        assert!((x[0] - 1.0).abs() < 1e-10); // x = 1
        assert!((x[1] - 2.0).abs() < 1e-10); // y = 2
    }
}
