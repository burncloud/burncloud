use crate::{
    collectors::{CpuCollector, DiskCollector, MemoryCollector},
    types::{MonitorError, SystemMetrics},
};
use std::sync::Arc;
use tokio::{
    sync::{Mutex, RwLock},
    time::Duration,
};

/// 系统监控服务
pub struct SystemMonitorService {
    cpu_collector: Arc<Mutex<CpuCollector>>,
    memory_collector: Arc<MemoryCollector>,
    disk_collector: Arc<DiskCollector>,
    cached_metrics: Arc<RwLock<Option<SystemMetrics>>>,
    update_interval: Duration,
}

impl SystemMonitorService {
    /// 创建新的监控服务
    pub fn new() -> Self {
        Self {
            cpu_collector: Arc::new(Mutex::new(CpuCollector::new())),
            memory_collector: Arc::new(MemoryCollector::new()),
            disk_collector: Arc::new(DiskCollector::new()),
            cached_metrics: Arc::new(RwLock::new(None)),
            update_interval: Duration::from_secs(1), // 默认1秒更新间隔
        }
    }

    /// 设置更新间隔
    pub fn with_update_interval(mut self, interval: Duration) -> Self {
        self.update_interval = interval;
        self
    }

    /// 获取当前系统指标
    pub async fn get_metrics(&self) -> Result<SystemMetrics, MonitorError> {
        // 尝试从缓存获取
        {
            let cached = self.cached_metrics.read().await;
            if let Some(metrics) = cached.as_ref() {
                // 如果缓存的数据还比较新鲜，直接返回
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();

                if now - metrics.timestamp < self.update_interval.as_secs() {
                    return Ok(metrics.clone());
                }
            }
        }

        // 收集新数据
        let metrics = self.collect_fresh_metrics().await?;

        // 更新缓存
        {
            let mut cached = self.cached_metrics.write().await;
            *cached = Some(metrics.clone());
        }

        Ok(metrics)
    }

    /// 强制刷新并获取最新指标
    pub async fn refresh_metrics(&self) -> Result<SystemMetrics, MonitorError> {
        let metrics = self.collect_fresh_metrics().await?;

        // 更新缓存
        {
            let mut cached = self.cached_metrics.write().await;
            *cached = Some(metrics.clone());
        }

        Ok(metrics)
    }

    /// 启动自动更新任务
    pub async fn start_auto_update(&self) -> Result<(), MonitorError> {
        let cpu_collector = self.cpu_collector.clone();
        let memory_collector = self.memory_collector.clone();
        let disk_collector = self.disk_collector.clone();
        let cached_metrics = self.cached_metrics.clone();
        let interval = self.update_interval;

        tokio::spawn(async move {
            let mut update_timer = tokio::time::interval(interval);

            loop {
                update_timer.tick().await;

                // 收集系统指标
                let result = Self::collect_metrics_internal(
                    &cpu_collector,
                    &memory_collector,
                    &disk_collector,
                )
                .await;

                if let Ok(metrics) = result {
                    // 更新缓存
                    let mut cached = cached_metrics.write().await;
                    *cached = Some(metrics);
                }
                // 如果收集失败，记录错误但继续运行
            }
        });

        Ok(())
    }

    /// 收集最新的系统指标
    async fn collect_fresh_metrics(&self) -> Result<SystemMetrics, MonitorError> {
        Self::collect_metrics_internal(
            &self.cpu_collector,
            &self.memory_collector,
            &self.disk_collector,
        )
        .await
    }

    /// 内部方法：收集系统指标
    async fn collect_metrics_internal(
        cpu_collector: &Arc<Mutex<CpuCollector>>,
        memory_collector: &Arc<MemoryCollector>,
        disk_collector: &Arc<DiskCollector>,
    ) -> Result<SystemMetrics, MonitorError> {
        // 并行收集各项指标
        let (cpu_result, memory_result, disk_result) = tokio::join!(
            async {
                let mut collector = cpu_collector.lock().await;
                collector.collect().await
            },
            memory_collector.collect(),
            disk_collector.collect_all()
        );

        let cpu = cpu_result?;
        let memory = memory_result?;
        let disks = disk_result?;

        Ok(SystemMetrics::new(cpu, memory, disks))
    }

    /// 获取CPU使用率
    pub async fn get_cpu_usage(&self) -> Result<f32, MonitorError> {
        let mut collector = self.cpu_collector.lock().await;
        let cpu_info = collector.collect().await?;
        Ok(cpu_info.usage_percent)
    }

    /// 获取内存信息
    pub async fn get_memory_info(&self) -> Result<crate::types::MemoryInfo, MonitorError> {
        self.memory_collector.collect().await
    }

    /// 获取磁盘信息
    pub async fn get_disk_info(&self) -> Result<Vec<crate::types::DiskInfo>, MonitorError> {
        self.disk_collector.collect_all().await
    }
}

impl Default for SystemMonitorService {
    fn default() -> Self {
        Self::new()
    }
}

/// 系统监控trait，提供统一的接口
#[async_trait::async_trait]
pub trait SystemMonitor {
    async fn get_cpu_usage(&self) -> Result<f32, MonitorError>;
    async fn get_memory_info(&self) -> Result<crate::types::MemoryInfo, MonitorError>;
    async fn get_system_metrics(&self) -> Result<SystemMetrics, MonitorError>;
}

#[async_trait::async_trait]
impl SystemMonitor for SystemMonitorService {
    async fn get_cpu_usage(&self) -> Result<f32, MonitorError> {
        self.get_cpu_usage().await
    }

    async fn get_memory_info(&self) -> Result<crate::types::MemoryInfo, MonitorError> {
        self.get_memory_info().await
    }

    async fn get_system_metrics(&self) -> Result<SystemMetrics, MonitorError> {
        self.get_metrics().await
    }
}
