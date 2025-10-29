//! 性能分析工具
//!
//! 提供系统性能监控、分析和优化建议功能

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use anyhow::{Result, Context};
use tracing::{info, warn, error, debug};

/// 性能监控器
#[derive(Debug)]
pub struct PerformanceMonitor {
    /// 性能指标收集器
    metrics: Arc<RwLock<PerformanceMetrics>>,
    /// 配置
    config: PerformanceConfig,
    /// 历史数据
    history: Arc<RwLock<VecDeque<PerformanceSnapshot>>>,
}

/// 性能指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// CPU使用率
    pub cpu_usage: f64,
    /// 内存使用情况
    pub memory: MemoryMetrics,
    /// 磁盘I/O
    pub disk_io: DiskIOMetrics,
    /// 网络I/O
    pub network_io: NetworkIOMetrics,
    /// 数据库性能
    pub database: DatabaseMetrics,
    /// 应用程序指标
    pub application: ApplicationMetrics,
}

/// 内存指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryMetrics {
    /// 总内存（字节）
    pub total_bytes: u64,
    /// 已使用内存（字节）
    pub used_bytes: u64,
    /// 可用内存（字节）
    pub available_bytes: u64,
    /// 使用率（百分比）
    pub usage_percent: f64,
    /// 缓存使用
    pub cache_bytes: u64,
    /// 交换空间使用
    pub swap_bytes: u64,
}

/// 磁盘I/O指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskIOMetrics {
    /// 读取字节数
    pub read_bytes: u64,
    /// 写入字节数
    pub write_bytes: u64,
    /// 读取操作数
    pub read_operations: u64,
    /// 写入操作数
    pub write_operations: u64,
    /// 平均读取延迟
    pub avg_read_latency: Duration,
    /// 平均写入延迟
    pub avg_write_latency: Duration,
    /// IOPS
    pub iops: u64,
    /// 磁盘使用率
    pub usage_percent: f64,
}

/// 网络I/O指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkIOMetrics {
    /// 接收字节数
    pub rx_bytes: u64,
    /// 发送字节数
    pub tx_bytes: u64,
    /// 接收包数
    pub rx_packets: u64,
    /// 发送包数
    pub tx_packets: u64,
    /// 网络延迟
    pub latency: Duration,
    /// 连接数
    pub connections: u64,
    /// 错误包数
    pub errors: u64,
}

/// 数据库性能指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseMetrics {
    /// 活跃连接数
    pub active_connections: u32,
    /// 空闲连接数
    pub idle_connections: u32,
    /// 查询总数
    pub total_queries: u64,
    /// 慢查询数
    pub slow_queries: u64,
    /// 平均查询时间
    pub avg_query_time: Duration,
    /// 数据库大小（字节）
    pub database_size: u64,
    /// 缓存命中率
    pub cache_hit_rate: f64,
    /// 锁等待时间
    pub lock_wait_time: Duration,
}

/// 应用程序指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationMetrics {
    /// HTTP请求数
    pub http_requests: u64,
    /// 平均响应时间
    pub avg_response_time: Duration,
    /// 错误率
    pub error_rate: f64,
    /// 并发连接数
    pub concurrent_connections: u32,
    /// DICOM操作数
    pub dicom_operations: u64,
    /// 任务队列长度
    pub task_queue_length: usize,
    /// 处理中的任务数
    pub processing_tasks: usize,
}

/// 性能快照
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSnapshot {
    /// 时间戳
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// 性能指标
    pub metrics: PerformanceMetrics,
}

/// 性能配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// 采样间隔
    pub sampling_interval: Duration,
    /// 历史数据保留时间
    pub history_retention: Duration,
    /// 历史数据最大条目数
    pub max_history_entries: usize,
    /// 警告阈值
    pub alert_thresholds: AlertThresholds,
}

/// 警告阈值
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertThresholds {
    /// CPU使用率阈值
    pub cpu_usage_warning: f64,
    pub cpu_usage_critical: f64,
    /// 内存使用率阈值
    pub memory_usage_warning: f64,
    pub memory_usage_critical: f64,
    /// 磁盘使用率阈值
    pub disk_usage_warning: f64,
    pub disk_usage_critical: f64,
    /// 响应时间阈值
    pub response_time_warning: Duration,
    pub response_time_critical: Duration,
    /// 错误率阈值
    pub error_rate_warning: f64,
    pub error_rate_critical: f64,
}

/// 性能分析报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceReport {
    /// 报告生成时间
    pub generated_at: chrono::DateTime<chrono::Utc>,
    /// 分析时间范围
    pub time_range: TimeRange,
    /// 总体健康状态
    pub overall_health: HealthStatus,
    /// 资源使用分析
    pub resource_analysis: ResourceAnalysis,
    /// 性能趋势
    pub trends: Vec<PerformanceTrend>,
    /// 瓶颈分析
    pub bottlenecks: Vec<Bottleneck>,
    /// 优化建议
    pub recommendations: Vec<OptimizationRecommendation>,
}

/// 时间范围
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    /// 开始时间
    pub start: chrono::DateTime<chrono::Utc>,
    /// 结束时间
    pub end: chrono::DateTime<chrono::Utc>,
}

/// 健康状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Warning,
    Critical,
}

/// 资源分析
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAnalysis {
    /// CPU分析
    pub cpu: ResourceAnalysisDetail,
    /// 内存分析
    pub memory: ResourceAnalysisDetail,
    /// 磁盘分析
    pub disk: ResourceAnalysisDetail,
    /// 网络分析
    pub network: ResourceAnalysisDetail,
}

/// 资源分析详情
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAnalysisDetail {
    /// 平均使用率
    pub avg_usage: f64,
    /// 最大使用率
    pub max_usage: f64,
    /// 使用率趋势
    pub usage_trend: TrendDirection,
    /// 预计耗尽时间（如果适用）
    pub estimated_exhaustion: Option<chrono::DateTime<chrono::Utc>>,
}

/// 趋势方向
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrendDirection {
    Increasing,
    Decreasing,
    Stable,
}

/// 性能趋势
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceTrend {
    /// 指标名称
    pub metric_name: String,
    /// 趋势方向
    pub direction: TrendDirection,
    /// 变化率
    pub change_rate: f64,
    /// 统计显著性
    pub significance: SignificanceLevel,
}

/// 显著性水平
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SignificanceLevel {
    Low,
    Medium,
    High,
}

/// 瓶颈分析
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bottleneck {
    /// 瓶颈类型
    pub bottleneck_type: BottleneckType,
    /// 影响程度
    pub impact: ImpactLevel,
    /// 描述
    pub description: String,
    /// 相关指标
    pub affected_metrics: Vec<String>,
    /// 检测时间
    pub detected_at: chrono::DateTime<chrono::Utc>,
}

/// 瓶颈类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BottleneckType {
    Cpu,
    Memory,
    Disk,
    Network,
    Database,
    Application,
}

/// 影响程度
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImpactLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// 优化建议
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationRecommendation {
    /// 建议类型
    pub recommendation_type: RecommendationType,
    /// 优先级
    pub priority: Priority,
    /// 描述
    pub description: String,
    /// 预期效果
    pub expected_impact: String,
    /// 实施难度
    pub implementation_difficulty: Difficulty,
    /// 相关配置
    pub related_config: Option<String>,
}

/// 建议类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecommendationType {
    ScaleUp,
    ScaleOut,
    OptimizeCode,
    TuneConfig,
    AddCaching,
    ImproveIndexing,
    CleanResources,
}

/// 优先级
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

/// 实施难度
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Difficulty {
    Easy,
    Medium,
    Hard,
}

impl PerformanceMonitor {
    /// 创建新的性能监控器
    pub fn new(config: PerformanceConfig) -> Self {
        Self {
            metrics: Arc::new(RwLock::new(PerformanceMetrics::default())),
            config,
            history: Arc::new(RwLock::new(VecDeque::new())),
        }
    }

    /// 收集性能指标
    pub async fn collect_metrics(&self) -> Result<PerformanceMetrics> {
        let metrics = self.gather_system_metrics().await?;

        // 更新当前指标
        {
            let mut current_metrics = self.metrics.write().await;
            *current_metrics = metrics.clone();
        }

        // 创建快照并添加到历史
        let snapshot = PerformanceSnapshot {
            timestamp: chrono::Utc::now(),
            metrics: metrics.clone(),
        };

        self.add_to_history(snapshot).await;

        Ok(metrics)
    }

    /// 获取系统指标
    async fn gather_system_metrics(&self) -> Result<PerformanceMetrics> {
        // 这里应该实际收集系统指标
        // 暂时返回模拟数据
        Ok(PerformanceMetrics {
            cpu_usage: 45.2,
            memory: MemoryMetrics {
                total_bytes: 16 * 1024 * 1024 * 1024, // 16GB
                used_bytes: 8 * 1024 * 1024 * 1024,  // 8GB
                available_bytes: 8 * 1024 * 1024 * 1024, // 8GB
                usage_percent: 50.0,
                cache_bytes: 2 * 1024 * 1024 * 1024,  // 2GB
                swap_bytes: 512 * 1024 * 1024,         // 512MB
            },
            disk_io: DiskIOMetrics {
                read_bytes: 1024 * 1024 * 100,  // 100MB
                write_bytes: 1024 * 1024 * 50,   // 50MB
                read_operations: 1000,
                write_operations: 500,
                avg_read_latency: Duration::from_millis(10),
                avg_write_latency: Duration::from_millis(15),
                iops: 1500,
                usage_percent: 65.5,
            },
            network_io: NetworkIOMetrics {
                rx_bytes: 1024 * 1024 * 200,  // 200MB
                tx_bytes: 1024 * 1024 * 100,  // 100MB
                rx_packets: 150000,
                tx_packets: 75000,
                latency: Duration::from_millis(5),
                connections: 250,
                errors: 2,
            },
            database: DatabaseMetrics {
                active_connections: 15,
                idle_connections: 25,
                total_queries: 10000,
                slow_queries: 5,
                avg_query_time: Duration::from_millis(50),
                database_size: 50 * 1024 * 1024 * 1024, // 50GB
                cache_hit_rate: 95.5,
                lock_wait_time: Duration::from_millis(2),
            },
            application: ApplicationMetrics {
                http_requests: 5000,
                avg_response_time: Duration::from_millis(120),
                error_rate: 0.5,
                concurrent_connections: 50,
                dicom_operations: 100,
                task_queue_length: 25,
                processing_tasks: 8,
            },
        })
    }

    /// 添加到历史记录
    async fn add_to_history(&self, snapshot: PerformanceSnapshot) {
        let mut history = self.history.write().await;

        history.push_back(snapshot);

        // 检查历史记录大小限制
        while history.len() > self.config.max_history_entries {
            history.pop_front();
        }

        // 清理过期数据
        self.cleanup_old_history().await;
    }

    /// 清理过期历史数据
    async fn cleanup_old_history(&self) {
        let mut history = self.history.write().await;
        let cutoff_time = chrono::Utc::now() - chrono::Duration::from_std(self.config.history_retention).unwrap();

        while let Some(front) = history.front() {
            if front.timestamp < cutoff_time {
                history.pop_front();
            } else {
                break;
            }
        }
    }

    /// 获取当前指标
    pub async fn get_current_metrics(&self) -> PerformanceMetrics {
        let metrics = self.metrics.read().await;
        metrics.clone()
    }

    /// 获取历史数据
    pub async fn get_history(&self, time_range: Option<TimeRange>) -> Vec<PerformanceSnapshot> {
        let history = self.history.read().await;

        match time_range {
            Some(range) => history
                .iter()
                .filter(|snapshot| {
                    snapshot.timestamp >= range.start && snapshot.timestamp <= range.end
                })
                .cloned()
                .collect(),
            None => history.iter().cloned().collect(),
        }
    }

    /// 生成性能报告
    pub async fn generate_performance_report(&self, time_range: TimeRange) -> Result<PerformanceReport> {
        let history = self.get_history(Some(time_range.clone())).await;

        if history.is_empty() {
            return Err(anyhow::anyhow!("No performance data available for the specified time range"));
        }

        let overall_health = self.calculate_overall_health(&history).await;
        let resource_analysis = self.analyze_resources(&history).await;
        let trends = self.analyze_trends(&history).await;
        let bottlenecks = self.identify_bottlenecks(&history).await;
        let recommendations = self.generate_recommendations(&bottlenecks, &resource_analysis).await;

        Ok(PerformanceReport {
            generated_at: chrono::Utc::now(),
            time_range,
            overall_health,
            resource_analysis,
            trends,
            bottlenecks,
            recommendations,
        })
    }

    /// 计算总体健康状态
    async fn calculate_overall_health(&self, history: &[PerformanceSnapshot]) -> HealthStatus {
        if history.is_empty() {
            return HealthStatus::Healthy;
        }

        let latest = &history[history.len() - 1];
        let thresholds = &self.config.alert_thresholds;

        // 检查各项指标是否超过阈值
        let critical_conditions = [
            latest.metrics.cpu_usage > thresholds.cpu_usage_critical,
            latest.metrics.memory.usage_percent > thresholds.memory_usage_critical,
            latest.metrics.disk_io.usage_percent > thresholds.disk_usage_critical,
            latest.metrics.application.avg_response_time > thresholds.response_time_critical,
            latest.metrics.application.error_rate > thresholds.error_rate_critical,
        ];

        let warning_conditions = [
            latest.metrics.cpu_usage > thresholds.cpu_usage_warning,
            latest.metrics.memory.usage_percent > thresholds.memory_usage_warning,
            latest.metrics.disk_io.usage_percent > thresholds.disk_usage_warning,
            latest.metrics.application.avg_response_time > thresholds.response_time_warning,
            latest.metrics.application.error_rate > thresholds.error_rate_warning,
        ];

        if critical_conditions.iter().any(|&condition| condition) {
            HealthStatus::Critical
        } else if warning_conditions.iter().any(|&condition| condition) {
            HealthStatus::Warning
        } else {
            HealthStatus::Healthy
        }
    }

    /// 分析资源使用情况
    async fn analyze_resources(&self, history: &[PerformanceSnapshot]) -> ResourceAnalysis {
        if history.is_empty() {
            return ResourceAnalysis {
                cpu: ResourceAnalysisDetail::default(),
                memory: ResourceAnalysisDetail::default(),
                disk: ResourceAnalysisDetail::default(),
                network: ResourceAnalysisDetail::default(),
            };
        }

        let cpu_usage: Vec<f64> = history.iter().map(|s| s.metrics.cpu_usage).collect();
        let memory_usage: Vec<f64> = history.iter().map(|s| s.metrics.memory.usage_percent).collect();
        let disk_usage: Vec<f64> = history.iter().map(|s| s.metrics.disk_io.usage_percent).collect();
        let network_latency: Vec<Duration> = history.iter().map(|s| s.metrics.network_io.latency).collect();

        ResourceAnalysis {
            cpu: self.analyze_resource_detail(&cpu_usage, &[]),
            memory: self.analyze_resource_detail(&memory_usage, &[]),
            disk: self.analyze_resource_detail(&disk_usage, &[]),
            network: ResourceAnalysisDetail {
                avg_usage: network_latency.iter().map(|d| d.as_millis() as f64).sum::<f64>() / network_latency.len() as f64,
                max_usage: network_latency.iter().map(|d| d.as_millis() as f64).max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap_or(0.0),
                usage_trend: TrendDirection::Stable, // 简化实现
                estimated_exhaustion: None,
            },
        }
    }

    /// 分析单个资源详情
    fn analyze_resource_detail(&self, usage_values: &[f64], _timestamps: &[chrono::DateTime<chrono::Utc>]) -> ResourceAnalysisDetail {
        if usage_values.is_empty() {
            return ResourceAnalysisDetail::default();
        }

        let avg_usage = usage_values.iter().sum::<f64>() / usage_values.len() as f64;
        let max_usage = *usage_values.iter().max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();

        // 简单的趋势分析
        let usage_trend = if usage_values.len() >= 2 {
            let first_half = &usage_values[..usage_values.len() / 2];
            let second_half = &usage_values[usage_values.len() / 2..];

            let first_avg = first_half.iter().sum::<f64>() / first_half.len() as f64;
            let second_avg = second_half.iter().sum::<f64>() / second_half.len() as f64;

            if second_avg > first_avg * 1.1 {
                TrendDirection::Increasing
            } else if second_avg < first_avg * 0.9 {
                TrendDirection::Decreasing
            } else {
                TrendDirection::Stable
            }
        } else {
            TrendDirection::Stable
        };

        ResourceAnalysisDetail {
            avg_usage,
            max_usage,
            usage_trend,
            estimated_exhaustion: None, // 需要更复杂的预测算法
        }
    }

    /// 分析性能趋势
    async fn analyze_trends(&self, history: &[PerformanceSnapshot]) -> Vec<PerformanceTrend> {
        let mut trends = Vec::new();

        // 分析CPU使用率趋势
        if history.len() >= 2 {
            let cpu_values: Vec<f64> = history.iter().map(|s| s.metrics.cpu_usage).collect();
            let trend = self.calculate_trend("CPU Usage", &cpu_values);
            trends.push(trend);
        }

        // 分析内存使用率趋势
        if history.len() >= 2 {
            let memory_values: Vec<f64> = history.iter().map(|s| s.metrics.memory.usage_percent).collect();
            let trend = self.calculate_trend("Memory Usage", &memory_values);
            trends.push(trend);
        }

        // 分析响应时间趋势
        if history.len() >= 2 {
            let response_times: Vec<f64> = history.iter()
                .map(|s| s.metrics.application.avg_response_time.as_millis() as f64)
                .collect();
            let trend = self.calculate_trend("Response Time", &response_times);
            trends.push(trend);
        }

        trends
    }

    /// 计算单个趋势
    fn calculate_trend(&self, metric_name: &str, values: &[f64]) -> PerformanceTrend {
        if values.len() < 2 {
            return PerformanceTrend {
                metric_name: metric_name.to_string(),
                direction: TrendDirection::Stable,
                change_rate: 0.0,
                significance: SignificanceLevel::Low,
            };
        }

        // 简单线性回归计算趋势
        let n = values.len() as f64;
        let sum_x: f64 = (0..values.len()).map(|i| i as f64).sum();
        let sum_y: f64 = values.iter().sum();
        let sum_xy: f64 = values.iter().enumerate().map(|(i, &y)| i as f64 * y).sum();
        let sum_x2: f64 = (0..values.len()).map(|i| (i as f64).powi(2)).sum();

        let slope = (n * sum_xy - sum_x * sum_y) / (n * sum_x2 - sum_x.powi(2));
        let avg_y = sum_y / n;
        let change_rate = if avg_y != 0.0 { slope / avg_y } else { 0.0 };

        let direction = if change_rate > 0.1 {
            TrendDirection::Increasing
        } else if change_rate < -0.1 {
            TrendDirection::Decreasing
        } else {
            TrendDirection::Stable
        };

        let significance = if change_rate.abs() > 0.5 {
            SignificanceLevel::High
        } else if change_rate.abs() > 0.2 {
            SignificanceLevel::Medium
        } else {
            SignificanceLevel::Low
        };

        PerformanceTrend {
            metric_name: metric_name.to_string(),
            direction,
            change_rate,
            significance,
        }
    }

    /// 识别性能瓶颈
    async fn identify_bottlenecks(&self, history: &[PerformanceSnapshot]) -> Vec<Bottleneck> {
        let mut bottlenecks = Vec::new();
        let thresholds = &self.config.alert_thresholds;

        if let Some(latest) = history.last() {
            // CPU瓶颈
            if latest.metrics.cpu_usage > thresholds.cpu_usage_critical {
                bottlenecks.push(Bottleneck {
                    bottleneck_type: BottleneckType::Cpu,
                    impact: ImpactLevel::High,
                    description: format!("CPU usage is critically high at {:.1}%", latest.metrics.cpu_usage),
                    affected_metrics: vec!["CPU Usage".to_string()],
                    detected_at: latest.timestamp,
                });
            }

            // 内存瓶颈
            if latest.metrics.memory.usage_percent > thresholds.memory_usage_critical {
                bottlenecks.push(Bottleneck {
                    bottleneck_type: BottleneckType::Memory,
                    impact: ImpactLevel::High,
                    description: format!("Memory usage is critically high at {:.1}%", latest.metrics.memory.usage_percent),
                    affected_metrics: vec!["Memory Usage".to_string()],
                    detected_at: latest.timestamp,
                });
            }

            // 磁盘瓶颈
            if latest.metrics.disk_io.usage_percent > thresholds.disk_usage_critical {
                bottlenecks.push(Bottleneck {
                    bottleneck_type: BottleneckType::Disk,
                    impact: ImpactLevel::Medium,
                    description: format!("Disk usage is critically high at {:.1}%", latest.metrics.disk_io.usage_percent),
                    affected_metrics: vec!["Disk Usage".to_string(), "IOPS".to_string()],
                    detected_at: latest.timestamp,
                });
            }

            // 数据库瓶颈
            if latest.metrics.database.slow_queries > 10 {
                bottlenecks.push(Bottleneck {
                    bottleneck_type: BottleneckType::Database,
                    impact: ImpactLevel::Medium,
                    description: format!("High number of slow queries: {}", latest.metrics.database.slow_queries),
                    affected_metrics: vec!["Slow Queries".to_string(), "Query Time".to_string()],
                    detected_at: latest.timestamp,
                });
            }
        }

        bottlenecks
    }

    /// 生成优化建议
    async fn generate_recommendations(&self, bottlenecks: &[Bottleneck], resource_analysis: &ResourceAnalysis) -> Vec<OptimizationRecommendation> {
        let mut recommendations = Vec::new();

        for bottleneck in bottlenecks {
            match bottleneck.bottleneck_type {
                BottleneckType::Cpu => {
                    recommendations.push(OptimizationRecommendation {
                        recommendation_type: RecommendationType::ScaleUp,
                        priority: Priority::High,
                        description: "Consider upgrading CPU or adding more CPU cores".to_string(),
                        expected_impact: "Improved processing capacity and reduced response times".to_string(),
                        implementation_difficulty: Difficulty::Medium,
                        related_config: Some("cpu_cores".to_string()),
                    });
                }
                BottleneckType::Memory => {
                    recommendations.push(OptimizationRecommendation {
                        recommendation_type: RecommendationType::ScaleUp,
                        priority: Priority::High,
                        description: "Consider adding more RAM to the system".to_string(),
                        expected_impact: "Reduced memory pressure and improved performance".to_string(),
                        implementation_difficulty: Difficulty::Medium,
                        related_config: Some("memory_size".to_string()),
                    });
                }
                BottleneckType::Disk => {
                    recommendations.push(OptimizationRecommendation {
                        recommendation_type: RecommendationType::AddCaching,
                        priority: Priority::Medium,
                        description: "Implement disk caching or use faster storage".to_string(),
                        expected_impact: "Reduced I/O wait times and improved throughput".to_string(),
                        implementation_difficulty: Difficulty::Easy,
                        related_config: Some("disk_cache".to_string()),
                    });
                }
                BottleneckType::Database => {
                    recommendations.push(OptimizationRecommendation {
                        recommendation_type: RecommendationType::ImproveIndexing,
                        priority: Priority::High,
                        description: "Optimize database indexes and query performance".to_string(),
                        expected_impact: "Faster query execution and reduced database load".to_string(),
                        implementation_difficulty: Difficulty::Medium,
                        related_config: Some("database_indexing".to_string()),
                    });
                }
                _ => {}
            }
        }

        recommendations
    }
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            cpu_usage: 0.0,
            memory: MemoryMetrics::default(),
            disk_io: DiskIOMetrics::default(),
            network_io: NetworkIOMetrics::default(),
            database: DatabaseMetrics::default(),
            application: ApplicationMetrics::default(),
        }
    }
}

impl Default for MemoryMetrics {
    fn default() -> Self {
        Self {
            total_bytes: 0,
            used_bytes: 0,
            available_bytes: 0,
            usage_percent: 0.0,
            cache_bytes: 0,
            swap_bytes: 0,
        }
    }
}

impl Default for DiskIOMetrics {
    fn default() -> Self {
        Self {
            read_bytes: 0,
            write_bytes: 0,
            read_operations: 0,
            write_operations: 0,
            avg_read_latency: Duration::ZERO,
            avg_write_latency: Duration::ZERO,
            iops: 0,
            usage_percent: 0.0,
        }
    }
}

impl Default for NetworkIOMetrics {
    fn default() -> Self {
        Self {
            rx_bytes: 0,
            tx_bytes: 0,
            rx_packets: 0,
            tx_packets: 0,
            latency: Duration::ZERO,
            connections: 0,
            errors: 0,
        }
    }
}

impl Default for DatabaseMetrics {
    fn default() -> Self {
        Self {
            active_connections: 0,
            idle_connections: 0,
            total_queries: 0,
            slow_queries: 0,
            avg_query_time: Duration::ZERO,
            database_size: 0,
            cache_hit_rate: 0.0,
            lock_wait_time: Duration::ZERO,
        }
    }
}

impl Default for ApplicationMetrics {
    fn default() -> Self {
        Self {
            http_requests: 0,
            avg_response_time: Duration::ZERO,
            error_rate: 0.0,
            concurrent_connections: 0,
            dicom_operations: 0,
            task_queue_length: 0,
            processing_tasks: 0,
        }
    }
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            sampling_interval: Duration::from_secs(30),
            history_retention: Duration::from_secs(24 * 60 * 60), // 24 hours
            max_history_entries: 2880, // 24 hours at 30-second intervals
            alert_thresholds: AlertThresholds {
                cpu_usage_warning: 70.0,
                cpu_usage_critical: 90.0,
                memory_usage_warning: 75.0,
                memory_usage_critical: 90.0,
                disk_usage_warning: 80.0,
                disk_usage_critical: 95.0,
                response_time_warning: Duration::from_millis(500),
                response_time_critical: Duration::from_millis(2000),
                error_rate_warning: 1.0,
                error_rate_critical: 5.0,
            },
        }
    }
}

impl Default for ResourceAnalysisDetail {
    fn default() -> Self {
        Self {
            avg_usage: 0.0,
            max_usage: 0.0,
            usage_trend: TrendDirection::Stable,
            estimated_exhaustion: None,
        }
    }
}

impl Default for PerformanceMonitor {
    fn default() -> Self {
        Self::new(PerformanceConfig::default())
    }
}