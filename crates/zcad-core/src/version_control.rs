//! 版本控制集成
//!
//! 提供原生Git支持，实现：
//! - 自动提交
//! - 历史版本管理
//! - 分支管理
//! - 差异比较
//! - 冲突解决

use crate::history::OperationId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// 提交ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CommitId([u8; 20]); // SHA-1 hash

impl CommitId {
    pub fn null() -> Self {
        Self([0; 20])
    }

    pub fn from_bytes(bytes: [u8; 20]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 20] {
        &self.0
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    pub fn from_hex(hex: &str) -> Result<Self, String> {
        let bytes = hex::decode(hex).map_err(|e| e.to_string())?;
        if bytes.len() != 20 {
            return Err("Invalid commit ID length".to_string());
        }
        let mut arr = [0u8; 20];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
    }
}

/// 分支ID
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BranchId(String);

impl BranchId {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    pub fn main() -> Self {
        Self("main".to_string())
    }

    pub fn master() -> Self {
        Self("master".to_string())
    }

    pub fn name(&self) -> &str {
        &self.0
    }
}

/// 提交
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Commit {
    /// 提交ID
    pub id: CommitId,

    /// 提交消息
    pub message: String,

    /// 作者
    pub author: String,

    /// 时间戳
    pub timestamp: std::time::SystemTime,

    /// 父提交
    pub parents: Vec<CommitId>,

    /// 关联的操作ID
    pub operation_ids: Vec<OperationId>,

    /// 变更的文件
    pub changed_files: Vec<String>,

    /// 提交类型
    pub commit_type: CommitType,
}

/// 提交类型
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum CommitType {
    /// 自动提交
    Auto,

    /// 用户手动提交
    Manual,

    /// 合并提交
    Merge,

    /// 分支创建
    Branch,

    /// 标签
    Tag,
}

/// 分支
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Branch {
    /// 分支ID
    pub id: BranchId,

    /// 当前提交
    pub head: CommitId,

    /// 创建时间
    pub created_at: std::time::SystemTime,

    /// 描述
    pub description: String,

    /// 是否跟踪远程分支
    pub tracking: Option<String>,
}

/// 版本控制系统
pub struct VersionControl {
    /// Git仓库路径
    repo_path: PathBuf,

    /// 当前分支
    current_branch: BranchId,

    /// 分支映射
    branches: HashMap<BranchId, Branch>,

    /// 提交缓存
    commit_cache: HashMap<CommitId, Commit>,

    /// 配置
    config: VCConfig,

    /// 统计信息
    stats: VCStats,
}

#[derive(Debug, Clone)]
pub struct VCConfig {
    /// 自动提交间隔（秒）
    pub auto_commit_interval: u64,

    /// 最大缓存提交数
    pub max_cache_size: usize,

    /// 是否启用自动提交
    pub auto_commit_enabled: bool,

    /// 作者名称
    pub author_name: String,

    /// 作者邮箱
    pub author_email: String,

    /// 提交消息模板
    pub commit_message_template: String,
}

impl Default for VCConfig {
    fn default() -> Self {
        Self {
            auto_commit_interval: 300, // 5分钟
            max_cache_size: 1000,
            auto_commit_enabled: true,
            author_name: "ZCAD User".to_string(),
            author_email: "user@zcad.local".to_string(),
            commit_message_template: "Auto-commit: {count} operations".to_string(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct VCStats {
    /// 总提交数
    pub total_commits: usize,

    /// 总分支数
    pub total_branches: usize,

    /// 最后提交时间
    pub last_commit_time: Option<std::time::SystemTime>,

    /// 自动提交次数
    pub auto_commits: usize,

    /// 手动提交次数
    pub manual_commits: usize,
}

impl VersionControl {
    /// 初始化版本控制系统
    pub fn init(repo_path: PathBuf) -> Result<Self, VCError> {
        // 检查是否已经是Git仓库
        let git_path = repo_path.join(".git");
        if git_path.exists() {
            return Err(VCError::AlreadyInitialized);
        }

        // 初始化Git仓库
        std::process::Command::new("git")
            .args(&["init"])
            .current_dir(&repo_path)
            .output()
            .map_err(|e| VCError::GitError(e.to_string()))?;

        // 配置Git
        let config = VCConfig::default();
        Self::configure_git(&repo_path, &config)?;

        let mut vc = Self {
            repo_path,
            current_branch: BranchId::main(),
            branches: HashMap::new(),
            commit_cache: HashMap::new(),
            config,
            stats: VCStats::default(),
        };

        // 创建初始分支
        vc.create_branch_internal(BranchId::main(), "Main branch".to_string())?;

        // 创建初始提交
        vc.initial_commit()?;

        Ok(vc)
    }

    /// 打开现有版本控制系统
    pub fn open(repo_path: PathBuf) -> Result<Self, VCError> {
        let git_path = repo_path.join(".git");
        if !git_path.exists() {
            return Err(VCError::NotInitialized);
        }

        let config = VCConfig::default(); // 应该从配置文件加载

        let mut vc = Self {
            repo_path,
            current_branch: BranchId::main(),
            branches: HashMap::new(),
            commit_cache: HashMap::new(),
            config,
            stats: VCStats::default(),
        };

        // 加载分支信息
        vc.load_branches()?;

        // 加载当前分支
        vc.load_current_branch()?;

        Ok(vc)
    }

    /// 配置Git
    fn configure_git(repo_path: &Path, config: &VCConfig) -> Result<(), VCError> {
        let commands = vec![
            vec!["config", "user.name", &config.author_name],
            vec!["config", "user.email", &config.author_email],
            vec!["config", "core.autocrlf", "false"],
            vec!["config", "core.safecrlf", "false"],
        ];

        for args in commands {
            std::process::Command::new("git")
                .args(&args)
                .current_dir(repo_path)
                .output()
                .map_err(|e| VCError::GitError(e.to_string()))?;
        }

        Ok(())
    }

    /// 创建初始提交
    fn initial_commit(&mut self) -> Result<(), VCError> {
        // 创建.zcadignore文件
        let ignore_content = "*.tmp\n*.bak\n.cache/\n";
        std::fs::write(self.repo_path.join(".zcadignore"), ignore_content)
            .map_err(|e| VCError::IoError(e.to_string()))?;

        // 创建README
        let readme_content = "# ZCAD Project\n\nCreated with ZCAD.\n";
        std::fs::write(self.repo_path.join("README.md"), readme_content)
            .map_err(|e| VCError::IoError(e.to_string()))?;

        // 添加文件
        self.git_command(&["add", "."])?;

        // 初始提交
        self.git_command(&["commit", "-m", "Initial commit"])?;

        Ok(())
    }

    /// 自动提交
    pub fn auto_commit(&mut self, operation_count: usize, description: &str) -> Result<CommitId, VCError> {
        if !self.config.auto_commit_enabled {
            return Err(VCError::AutoCommitDisabled);
        }

        let message = self.config.commit_message_template
            .replace("{count}", &operation_count.to_string())
            .replace("{description}", description);

        self.commit(&message, vec![], CommitType::Auto)
    }

    /// 手动提交
    pub fn manual_commit(&mut self, message: &str, operation_ids: Vec<OperationId>) -> Result<CommitId, VCError> {
        self.commit(message, operation_ids, CommitType::Manual)
    }

    /// 执行提交
    fn commit(&mut self, message: &str, operation_ids: Vec<OperationId>, commit_type: CommitType) -> Result<CommitId, VCError> {
        // 检查是否有变更
        let status = self.git_command(&["status", "--porcelain"])?;
        if status.stdout.is_empty() {
            return Err(VCError::NoChanges);
        }

        // 添加所有变更
        self.git_command(&["add", "."])?;

        // 提交
        self.git_command(&["commit", "-m", message])?;

        // 获取提交ID
        let log_output = self.git_command(&["log", "-1", "--format=%H"])?;
        let commit_hash = String::from_utf8_lossy(&log_output.stdout).trim().to_string();

        let commit_id = CommitId::from_hex(&commit_hash)?;

        // 创建Commit对象
        let commit = Commit {
            id: commit_id,
            message: message.to_string(),
            author: self.config.author_name.clone(),
            timestamp: std::time::SystemTime::now(),
            parents: self.get_current_parents()?,
            operation_ids,
            changed_files: self.get_changed_files()?,
            commit_type,
        };

        // 缓存提交
        self.commit_cache.insert(commit_id, commit.clone());

        // 更新分支
        if let Some(branch) = self.branches.get_mut(&self.current_branch) {
            branch.head = commit_id;
        }

        // 更新统计
        self.stats.total_commits += 1;
        self.stats.last_commit_time = Some(commit.timestamp);

        match commit_type {
            CommitType::Auto => self.stats.auto_commits += 1,
            CommitType::Manual => self.stats.manual_commits += 1,
            _ => {}
        }

        // 清理缓存
        if self.commit_cache.len() > self.config.max_cache_size {
            self.cleanup_cache();
        }

        Ok(commit_id)
    }

    /// 创建分支
    pub fn create_branch(&mut self, branch_id: BranchId, description: String) -> Result<(), VCError> {
        if self.branches.contains_key(&branch_id) {
            return Err(VCError::BranchExists(branch_id.0));
        }

        self.create_branch_internal(branch_id, description)
    }

    fn create_branch_internal(&mut self, branch_id: BranchId, description: String) -> Result<(), VCError> {
        // 创建Git分支
        self.git_command(&["checkout", "-b", &branch_id.0])?;

        let branch = Branch {
            id: branch_id.clone(),
            head: self.get_current_commit()?,
            created_at: std::time::SystemTime::now(),
            description,
            tracking: None,
        };

        self.branches.insert(branch_id.clone(), branch);
        self.stats.total_branches += 1;

        // 提交分支创建
        self.commit(
            &format!("Create branch '{}'", branch_id.0),
            vec![],
            CommitType::Branch,
        )?;

        Ok(())
    }

    /// 切换分支
    pub fn switch_branch(&mut self, branch_id: &BranchId) -> Result<(), VCError> {
        if !self.branches.contains_key(branch_id) {
            return Err(VCError::BranchNotFound(branch_id.0.clone()));
        }

        self.git_command(&["checkout", &branch_id.0])?;
        self.current_branch = branch_id.clone();

        Ok(())
    }

    /// 合并分支
    pub fn merge_branch(&mut self, source_branch: &BranchId, message: &str) -> Result<CommitId, VCError> {
        if !self.branches.contains_key(source_branch) {
            return Err(VCError::BranchNotFound(source_branch.0.clone()));
        }

        // 切换到目标分支
        let target_branch = self.current_branch.clone();
        self.switch_branch(source_branch)?;

        // 合并
        self.git_command(&["merge", &target_branch.0, "--no-ff", "-m", message])?;

        // 切换回原分支
        self.switch_branch(&target_branch)?;

        // 创建合并提交记录
        self.commit(message, vec![], CommitType::Merge)
    }

    /// 获取提交历史
    pub fn get_commit_history(&self, limit: usize) -> Result<Vec<Commit>, VCError> {
        let log_output = self.git_command(&[
            "log",
            "--oneline",
            "-n",
            &limit.to_string(),
            "--format=%H|%s|%an|%at",
        ])?;

        let mut commits = Vec::new();
        for line in String::from_utf8_lossy(&log_output.stdout).lines() {
            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() >= 4 {
                let commit_id = CommitId::from_hex(parts[0])?;
                let message = parts[1].to_string();
                let author = parts[2].to_string();
                let timestamp = std::time::UNIX_EPOCH + std::time::Duration::from_secs(parts[3].parse().unwrap_or(0));

                let commit = Commit {
                    id: commit_id,
                    message,
                    author,
                    timestamp,
                    parents: vec![], // 简化为不加载父提交
                    operation_ids: vec![],
                    changed_files: vec![],
                    commit_type: CommitType::Manual, // 简化处理
                };

                commits.push(commit);
            }
        }

        Ok(commits)
    }

    /// 获取两个提交之间的差异
    pub fn get_diff(&self, from: &CommitId, to: &CommitId) -> Result<String, VCError> {
        let output = self.git_command(&[
            "diff",
            &from.to_hex(),
            &to.to_hex(),
        ])?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// 撤销到指定提交
    pub fn revert_to_commit(&mut self, commit_id: &CommitId) -> Result<(), VCError> {
        self.git_command(&["reset", "--hard", &commit_id.to_hex()])?;
        Ok(())
    }

    /// 获取当前分支
    pub fn current_branch(&self) -> &BranchId {
        &self.current_branch
    }

    /// 获取所有分支
    pub fn branches(&self) -> &HashMap<BranchId, Branch> {
        &self.branches
    }

    /// 获取统计信息
    pub fn stats(&self) -> &VCStats {
        &self.stats
    }

    /// 更新配置
    pub fn update_config(&mut self, config: VCConfig) -> Result<(), VCError> {
        self.config = config;
        Self::configure_git(&self.repo_path, &self.config)?;
        Ok(())
    }

    // === 内部辅助方法 ===

    /// 执行Git命令
    fn git_command(&self, args: &[&str]) -> Result<std::process::Output, VCError> {
        let output = std::process::Command::new("git")
            .args(args)
            .current_dir(&self.repo_path)
            .output()
            .map_err(|e| VCError::GitError(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(VCError::GitError(stderr.to_string()));
        }

        Ok(output)
    }

    /// 获取当前提交ID
    fn get_current_commit(&self) -> Result<CommitId, VCError> {
        let output = self.git_command(&["rev-parse", "HEAD"])?;
        let hash = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(CommitId::from_hex(&hash)?)
    }

    /// 获取父提交
    fn get_current_parents(&self) -> Result<Vec<CommitId>, VCError> {
        let output = self.git_command(&["rev-parse", "HEAD~1"])?;
        if output.status.success() {
            let hash = String::from_utf8_lossy(&output.stdout).trim().to_string();
            Ok(vec![CommitId::from_hex(&hash)?])
        } else {
            Ok(vec![]) // 初始提交没有父提交
        }
    }

    /// 获取变更的文件
    fn get_changed_files(&self) -> Result<Vec<String>, VCError> {
        let output = self.git_command(&["diff", "--name-only", "HEAD~1"])?;
        let files = String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(|s| s.to_string())
            .collect();
        Ok(files)
    }

    /// 加载分支信息
    fn load_branches(&mut self) -> Result<(), VCError> {
        let _output = self.git_command(&["branch", "-a"])?;
        // 解析分支信息（简化实现）
        // 实际应该解析输出并创建Branch对象
        Ok(())
    }

    /// 加载当前分支
    fn load_current_branch(&mut self) -> Result<(), VCError> {
        let output = self.git_command(&["rev-parse", "--abbrev-ref", "HEAD"])?;
        let branch_name = String::from_utf8_lossy(&output.stdout).trim().to_string();
        self.current_branch = BranchId::new(branch_name);
        Ok(())
    }

    /// 清理缓存
    fn cleanup_cache(&mut self) {
        // 简单的LRU清理策略
        if self.commit_cache.len() > self.config.max_cache_size {
            let to_remove: Vec<_> = self.commit_cache.keys()
                .skip(self.config.max_cache_size / 2)
                .cloned()
                .collect();

            for key in to_remove {
                self.commit_cache.remove(&key);
            }
        }
    }
}

/// 版本控制错误
#[derive(Debug, thiserror::Error)]
pub enum VCError {
    #[error("String error: {0}")]
    StringError(String),
    #[error("Version control not initialized")]
    NotInitialized,

    #[error("Version control already initialized")]
    AlreadyInitialized,

    #[error("Git command failed: {0}")]
    GitError(String),

    #[error("IO error: {0}")]
    IoError(String),

    #[error("Branch '{0}' already exists")]
    BranchExists(String),

    #[error("Branch '{0}' not found")]
    BranchNotFound(String),

    #[error("Auto-commit is disabled")]
    AutoCommitDisabled,

    #[error("No changes to commit")]
    NoChanges,

    #[error("Invalid commit ID")]
    InvalidCommitId,
}

impl From<String> for VCError {
    fn from(error: String) -> Self {
        VCError::StringError(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_version_control_init() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path().to_path_buf();

        // 初始化版本控制
        let vc = VersionControl::init(repo_path.clone()).unwrap();

        // 检查Git仓库是否创建
        assert!(repo_path.join(".git").exists());

        // 检查初始文件
        assert!(repo_path.join("README.md").exists());
        assert!(repo_path.join(".zcadignore").exists());

        // 检查统计
        let stats = vc.stats();
        assert_eq!(stats.total_branches, 1);
        assert!(stats.last_commit_time.is_some());
    }

    #[test]
    fn test_commit_id() {
        let id1 = CommitId::null();
        let id2 = CommitId::from_bytes([1; 20]);

        assert_eq!(id1, CommitId::null());
        assert_ne!(id1, id2);

        let hex = id2.to_hex();
        let id3 = CommitId::from_hex(&hex).unwrap();
        assert_eq!(id2, id3);
    }

    #[test]
    fn test_branch_id() {
        let main = BranchId::main();
        let custom = BranchId::new("feature-x");

        assert_eq!(main.name(), "main");
        assert_eq!(custom.name(), "feature-x");
    }
}
