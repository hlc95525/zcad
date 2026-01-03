//! 异步核心架构
//!
//! 提供异步数据流和消息传递系统，支持高性能并发操作：
//! - 异步消息总线
//! - 任务调度器
//! - 数据流管道
//! - 性能监控

use crate::entity::{Entity, EntityId};
use futures::channel::mpsc;
use futures::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, Semaphore};
use tokio::task;

/// 消息类型
#[derive(Debug, Clone)]
pub enum Message {
    /// 实体操作
    EntityCreated {
        entity: Entity,
    },
    EntityModified {
        entity_id: EntityId,
        old_entity: Option<Entity>,
        new_entity: Entity,
    },
    EntityDeleted {
        entity_id: EntityId,
        old_entity: Option<Entity>,
    },

    /// 约束操作
    ConstraintAdded {
        constraint: crate::parametric::Constraint,
    },
    ConstraintSolved {
        variables_updated: Vec<(crate::parametric::VariableId, f64)>,
    },

    /// 历史操作
    OperationRecorded {
        operation: crate::history::Operation,
    },
    OperationUndone {
        operation: crate::history::Operation,
    },

    /// 版本控制
    VersionCommitted {
        commit_id: crate::version_control::CommitId,
        message: String,
    },

    /// 性能监控
    PerformanceUpdate {
        metrics: PerformanceMetrics,
    },

    /// 系统控制
    Shutdown,
    Heartbeat,
}

/// 性能指标
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub timestamp: std::time::Instant,
    pub render_fps: f64,
    pub entity_count: usize,
    pub memory_usage: usize,
    pub gpu_memory: usize,
    pub constraint_solve_time: std::time::Duration,
    pub render_time: std::time::Duration,
}

/// 消息总线
pub struct MessageBus {
    _sender: Arc<RwLock<mpsc::UnboundedSender<Message>>>,
    receiver: Arc<RwLock<Option<mpsc::UnboundedReceiver<Message>>>>,
    subscribers: Arc<RwLock<HashMap<String, Vec<mpsc::UnboundedSender<Message>>>>>,
}

impl MessageBus {
    /// 创建新的消息总线
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::unbounded();

        Self {
            _sender: Arc::new(RwLock::new(sender)),
            receiver: Arc::new(RwLock::new(Some(receiver))),
            subscribers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 发送消息
    pub async fn send(&self, message: Message) -> Result<(), String> {
        // 广播给订阅者
        let subscribers = self.subscribers.read().await;
        let mut send_tasks = Vec::new();

        for senders in subscribers.values() {
            for subscriber_sender in senders {
                let msg = message.clone();
                let mut sender_clone = subscriber_sender.clone();
                send_tasks.push(async move {
                    let _ = sender_clone.send(msg).await;
                });
            }
        }

        // 并行发送所有消息
        for task in send_tasks {
            task.await;
        }

        Ok(())
    }

    /// 接收消息
    pub async fn receive(&self) -> Option<Message> {
        if let Some(receiver) = &mut *self.receiver.write().await {
            receiver.next().await
        } else {
            None
        }
    }

    /// 订阅消息
    pub async fn subscribe(&self, subscriber_id: String) -> mpsc::UnboundedReceiver<Message> {
        let (sender, receiver) = mpsc::unbounded();

        let mut subscribers = self.subscribers.write().await;
        subscribers.entry(subscriber_id).or_insert_with(Vec::new).push(sender);

        receiver
    }

    /// 取消订阅
    pub async fn unsubscribe(&self, subscriber_id: &str) {
        let mut subscribers = self.subscribers.write().await;
        subscribers.remove(subscriber_id);
    }
}

/// 异步任务处理器
pub struct TaskProcessor {
    semaphore: Arc<Semaphore>,
    message_bus: Arc<MessageBus>,
}

impl TaskProcessor {
    /// 创建任务处理器
    pub fn new(message_bus: Arc<MessageBus>, max_concurrent: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            message_bus,
        }
    }

    /// 异步执行任务
    pub async fn execute<F, Fut, T>(&self, task: F) -> Result<T, String>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T, String>>,
    {
        let _permit = self.semaphore.acquire().await.map_err(|e| e.to_string())?;
        task().await
    }

    /// 批量执行任务
    pub async fn execute_batch<F, Fut, T>(&self, tasks: Vec<F>) -> Vec<Result<T, String>>
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<T, String>> + Send,
        T: Send + 'static,
    {
        let mut handles = Vec::new();

        for task in tasks {
            let permit = self.semaphore.clone().acquire_owned().await.map_err(|e| e.to_string());
            let _message_bus = self.message_bus.clone();

            let handle = task::spawn(async move {
                let _permit = permit?;
                let result = task().await;
                Ok(result) as Result<Result<T, String>, String>
            });

            handles.push(handle);
        }

        let mut results = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(Ok(result)) => results.push(result),
                Ok(Err(e)) => results.push(Err(e)),
                Err(e) => results.push(Err(format!("Task panicked: {}", e))),
            }
        }

        results
    }
}

/// 数据流管道（简化版本）
pub struct DataPipeline<T> {
    _message_bus: Arc<MessageBus>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> DataPipeline<T> {
    /// 创建数据管道
    pub fn new(message_bus: Arc<MessageBus>) -> Self {
        Self {
            _message_bus: message_bus,
            _phantom: std::marker::PhantomData,
        }
    }
}

/// 异步核心系统
pub struct AsyncCore {
    message_bus: Arc<MessageBus>,
    task_processor: Arc<TaskProcessor>,
    performance_monitor: Arc<PerformanceMonitor>,
    running: Arc<RwLock<bool>>,
}

impl AsyncCore {
    /// 创建异步核心
    pub fn new() -> Self {
        let message_bus = Arc::new(MessageBus::new());
        let task_processor = Arc::new(TaskProcessor::new(message_bus.clone(), 10));
        let performance_monitor = Arc::new(PerformanceMonitor::new(message_bus.clone()));

        Self {
            message_bus,
            task_processor,
            performance_monitor,
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// 启动异步核心
    pub async fn start(&self) -> Result<(), String> {
        let mut running = self.running.write().await;
        if *running {
            return Err("AsyncCore already running".to_string());
        }
        *running = true;

        // 启动性能监控
        self.performance_monitor.start().await;

        // 启动心跳任务
        self.start_heartbeat().await;

        Ok(())
    }

    /// 停止异步核心
    pub async fn stop(&self) -> Result<(), String> {
        let mut running = self.running.write().await;
        if !*running {
            return Err("AsyncCore not running".to_string());
        }
        *running = false;

        // 发送关闭消息
        self.message_bus.send(Message::Shutdown).await?;

        // 停止性能监控
        self.performance_monitor.stop().await;

        Ok(())
    }

    /// 获取消息总线
    pub fn message_bus(&self) -> &Arc<MessageBus> {
        &self.message_bus
    }

    /// 获取任务处理器
    pub fn task_processor(&self) -> &Arc<TaskProcessor> {
        &self.task_processor
    }

    /// 获取性能监控器
    pub fn performance_monitor(&self) -> &Arc<PerformanceMonitor> {
        &self.performance_monitor
    }

    /// 创建数据管道
    pub fn create_pipeline<T>(&self) -> DataPipeline<T>
    where
        T: Send + Clone + 'static,
    {
        DataPipeline::new(self.message_bus.clone())
    }

    /// 检查是否正在运行
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }

    /// 启动心跳任务
    async fn start_heartbeat(&self) {
        let message_bus = self.message_bus.clone();
        let running = self.running.clone();

        task::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));

            while *running.read().await {
                interval.tick().await;
                let _ = message_bus.send(Message::Heartbeat).await;
            }
        });
    }
}

impl Default for AsyncCore {
    fn default() -> Self {
        Self::new()
    }
}

/// 性能监控器
pub struct PerformanceMonitor {
    message_bus: Arc<MessageBus>,
    metrics: Arc<RwLock<PerformanceMetrics>>,
    running: Arc<RwLock<bool>>,
}

impl PerformanceMonitor {
    /// 创建性能监控器
    pub fn new(message_bus: Arc<MessageBus>) -> Self {
        Self {
            message_bus,
            metrics: Arc::new(RwLock::new(PerformanceMetrics {
                timestamp: std::time::Instant::now(),
                render_fps: 0.0,
                entity_count: 0,
                memory_usage: 0,
                gpu_memory: 0,
                constraint_solve_time: std::time::Duration::from_secs(0),
                render_time: std::time::Duration::from_secs(0),
            })),
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// 启动监控
    pub async fn start(&self) {
        let mut running = self.running.write().await;
        if *running {
            return;
        }
        *running = true;

        let message_bus = self.message_bus.clone();
        let metrics = self.metrics.clone();
        let running_flag = self.running.clone();

        task::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));

            while *running_flag.read().await {
                interval.tick().await;

                // 收集性能指标
                let current_metrics = {
                    let mut m = metrics.write().await;
                    m.timestamp = std::time::Instant::now();
                    // 这里应该收集实际的性能数据
                    m.clone()
                };

                // 发送性能更新消息
                let _ = message_bus.send(Message::PerformanceUpdate {
                    metrics: current_metrics,
                }).await;
            }
        });
    }

    /// 停止监控
    pub async fn stop(&self) {
        let mut running = self.running.write().await;
        *running = false;
    }

    /// 更新渲染FPS
    pub async fn update_render_fps(&self, fps: f64) {
        let mut metrics = self.metrics.write().await;
        metrics.render_fps = fps;
    }

    /// 更新实体数量
    pub async fn update_entity_count(&self, count: usize) {
        let mut metrics = self.metrics.write().await;
        metrics.entity_count = count;
    }

    /// 获取当前指标
    pub async fn current_metrics(&self) -> PerformanceMetrics {
        self.metrics.read().await.clone()
    }
}

/// 异步工具函数
pub mod utils {
    use super::*;

    /// 创建异步实体处理管道
    pub fn create_entity_processing_pipeline(message_bus: Arc<MessageBus>) -> DataPipeline<crate::entity::Entity> {
        DataPipeline::new(message_bus)
    }

    /// 创建异步约束求解管道
    pub fn create_constraint_solving_pipeline(message_bus: Arc<MessageBus>) -> DataPipeline<crate::parametric::ConstraintSystem> {
        DataPipeline::new(message_bus)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_message_bus() {
        let bus = MessageBus::new();

        // 发送消息
        bus.send(Message::EntityCreated {
            entity: Entity::new(crate::geometry::Geometry::Point(crate::geometry::Point::new(0.0, 0.0))),
        }).await.unwrap();

        // 接收消息
        let message = bus.receive().await;
        assert!(matches!(message, Some(Message::EntityCreated { .. })));
    }

    #[tokio::test]
    async fn test_async_core() {
        let core = AsyncCore::new();

        // 启动核心
        core.start().await.unwrap();
        assert!(core.is_running().await);

        // 发送消息
        core.message_bus().send(Message::Heartbeat).await.unwrap();

        // 停止核心
        core.stop().await.unwrap();
        assert!(!core.is_running().await);
    }

    #[tokio::test]
    async fn test_data_pipeline() {
        let message_bus = Arc::new(MessageBus::new());
        let pipeline = DataPipeline::new(message_bus)
            .add_stage(|x: i32| async move { Ok(x + 1) })
            .add_stage(|x: i32| async move { Ok(x * 2) });

        let result = pipeline.execute(5).await.unwrap();
        assert_eq!(result, 12); // (5 + 1) * 2 = 12
    }
}
