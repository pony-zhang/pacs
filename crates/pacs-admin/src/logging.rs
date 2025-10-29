//! 日志聚合系统
//!
//! 提供集中化的日志收集、聚合、分析和查询功能

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use anyhow::{Result, Context};
use tracing::{info, warn, error, debug, Level};
use chrono::{DateTime, Utc};
use regex::Regex;

/// 日志级别映射
fn map_tracing_level(level: &Level) -> LogLevel {
    match level {
        Level::ERROR => LogLevel::Error,
        Level::WARN => LogLevel::Warning,
        Level::INFO => LogLevel::Info,
        Level::DEBUG => LogLevel::Debug,
        Level::TRACE => LogLevel::Trace,
    }
}

/// 日志级别
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warning,
    Error,
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Trace => write!(f, "TRACE"),
            LogLevel::Debug => write!(f, "DEBUG"),
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Warning => write!(f, "WARN"),
            LogLevel::Error => write!(f, "ERROR"),
        }
    }
}

/// 日志条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    /// 日志ID
    pub id: String,
    /// 时间戳
    pub timestamp: DateTime<Utc>,
    /// 日志级别
    pub level: LogLevel,
    /// 消息内容
    pub message: String,
    /// 模块名
    pub module: Option<String>,
    /// 目标名
    pub target: Option<String>,
    /// 文件名
    pub file: Option<String>,
    /// 行号
    pub line: Option<u32>,
    /// 线程名
    pub thread: Option<String>,
    /// 上下文字段
    pub fields: HashMap<String, String>,
    /// 堆栈跟踪（错误日志）
    pub stack_trace: Option<String>,
}

/// 日志查询过滤器
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogFilter {
    /// 时间范围
    pub time_range: Option<TimeRange>,
    /// 日志级别过滤
    pub levels: Option<Vec<LogLevel>>,
    /// 模块过滤
    pub modules: Option<Vec<String>>,
    /// 消息内容匹配（正则表达式）
    pub message_pattern: Option<String>,
    /// 字段过滤
    pub field_filters: HashMap<String, String>,
    /// 限制数量
    pub limit: Option<usize>,
    /// 排序方式
    pub sort_order: SortOrder,
}

/// 时间范围
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    /// 开始时间
    pub start: DateTime<Utc>,
    /// 结束时间
    pub end: DateTime<Utc>,
}

/// 排序方式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SortOrder {
    /// 时间升序
    Ascending,
    /// 时间降序
    Descending,
}

/// 日志统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogStats {
    /// 总日志数
    pub total_logs: u64,
    /// 按级别统计
    pub logs_by_level: HashMap<LogLevel, u64>,
    /// 按模块统计
    pub logs_by_module: HashMap<String, u64>,
    /// 时间范围内的日志数量
    pub logs_in_time_range: u64,
    /// 错误日志数量
    pub error_logs: u64,
    /// 警告日志数量
    pub warning_logs: u64,
    /// 最近错误日志
    pub recent_errors: Vec<LogEntry>,
}

/// 日志聚合器
#[derive(Debug)]
pub struct LogAggregator {
    /// 内存中的日志缓存
    log_cache: Arc<RwLock<VecDeque<LogEntry>>>,
    /// 日志索引（按级别）
    index_by_level: Arc<RwLock<HashMap<LogLevel, Vec<usize>>>>,
    /// 日志索引（按模块）
    index_by_module: Arc<RwLock<HashMap<String, Vec<usize>>>>,
    /// 日志索引（按时间）
    index_by_time: Arc<RwLock<Vec<(DateTime<Utc>, usize)>>>>,
    /// 配置
    config: LogConfig,
    /// 正则表达式缓存
    regex_cache: Arc<RwLock<HashMap<String, Regex>>>,
}

/// 日志配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogConfig {
    /// 最大缓存日志数
    pub max_cache_size: usize,
    /// 日志保留时间
    pub retention_period: Duration,
    /// 是否启用索引
    pub enable_indexing: bool,
    /// 索引更新间隔
    pub index_update_interval: Duration,
    /// 压缩旧日志
    pub compress_old_logs: bool,
    /// 日志轮转配置
    pub rotation: LogRotationConfig,
}

/// 日志轮转配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogRotationConfig {
    /// 是否启用轮转
    pub enabled: bool,
    /// 最大文件大小
    pub max_file_size: u64,
    /// 最大文件数量
    pub max_files: u32,
    /// 轮转间隔
    pub rotation_interval: Duration,
}

/// 日志分析器
pub struct LogAnalyzer {
    /// 错误模式检测器
    error_patterns: Vec<ErrorPattern>,
    /// 性能分析器
    performance_analyzer: PerformanceAnalyzer,
}

/// 错误模式
#[derive(Debug, Clone)]
pub struct ErrorPattern {
    /// 模式名称
    pub name: String,
    /// 匹配正则表达式
    pub pattern: Regex,
    /// 严重级别
    pub severity: LogLevel,
    /// 描述
    pub description: String,
}

/// 性能分析器
#[derive(Debug)]
pub struct PerformanceAnalyzer {
    /// 慢查询阈值
    pub slow_query_threshold: Duration,
    /// 响应时间统计
    pub response_time_stats: Arc<RwLock<VecDeque<Duration>>>,
}

impl LogAggregator {
    /// 创建新的日志聚合器
    pub fn new(config: LogConfig) -> Self {
        Self {
            log_cache: Arc::new(RwLock::new(VecDeque::new())),
            index_by_level: Arc::new(RwLock::new(HashMap::new())),
            index_by_module: Arc::new(RwLock::new(HashMap::new())),
            index_by_time: Arc::new(RwLock::new(Vec::new())),
            config,
            regex_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 添加日志条目
    pub async fn add_log_entry(&self, entry: LogEntry) -> Result<()> {
        let mut cache = self.log_cache.write().await;

        // 检查缓存大小限制
        if cache.len() >= self.config.max_cache_size {
            cache.pop_front();
        }

        let index = cache.len();
        cache.push_back(entry.clone());

        // 更新索引
        if self.config.enable_indexing {
            self.update_indices(&entry, index).await;
        }

        Ok(())
    }

    /// 更新索引
    async fn update_indices(&self, entry: &LogEntry, index: usize) {
        // 更新级别索引
        let mut level_index = self.index_by_level.write().await;
        level_index.entry(entry.level.clone()).or_insert_with(Vec::new).push(index);

        // 更新模块索引
        if let Some(module) = &entry.module {
            let mut module_index = self.index_by_module.write().await;
            module_index.entry(module.clone()).or_insert_with(Vec::new).push(index);
        }

        // 更新时间索引
        let mut time_index = self.index_by_time.write().await;
        time_index.push((entry.timestamp, index));

        // 保持时间索引有序
        time_index.sort_by_key(|(time, _)| *time);
    }

    /// 查询日志
    pub async fn query_logs(&self, filter: &LogFilter) -> Result<Vec<LogEntry>> {
        let cache = self.log_cache.read().await;
        let mut results = Vec::new();

        // 使用索引进行过滤
        let candidate_indices = self.get_candidate_indices(filter).await;

        for &index in &candidate_indices {
            if let Some(entry) = cache.get(index) {
                if self.matches_filter(entry, filter).await {
                    results.push(entry.clone());
                }
            }
        }

        // 应用排序
        match filter.sort_order {
            SortOrder::Ascending => results.sort_by(|a, b| a.timestamp.cmp(&b.timestamp)),
            SortOrder::Descending => results.sort_by(|a, b| b.timestamp.cmp(&a.timestamp)),
        }

        // 应用限制
        if let Some(limit) = filter.limit {
            results.truncate(limit);
        }

        Ok(results)
    }

    /// 获取候选索引
    async fn get_candidate_indices(&self, filter: &LogFilter) -> Vec<usize> {
        let mut candidates = None;

        // 时间范围过滤
        if let Some(time_range) = &filter.time_range {
            let time_index = self.index_by_time.read().await;
            let mut time_candidates = Vec::new();

            for &(timestamp, index) in time_index.iter() {
                if timestamp >= time_range.start && timestamp <= time_range.end {
                    time_candidates.push(index);
                }
            }

            candidates = Some(time_candidates);
        }

        // 级别过滤
        if let Some(levels) = &filter.levels {
            let level_index = self.index_by_level.read().await;
            let mut level_candidates = Vec::new();

            for level in levels {
                if let Some(indices) = level_index.get(level) {
                    level_candidates.extend(indices);
                }
            }

            candidates = match candidates {
                Some(existing) => {
                    // 取交集
                    Some(existing.into_iter().filter(|i| level_candidates.contains(i)).collect())
                }
                None => Some(level_candidates),
            };
        }

        // 模块过滤
        if let Some(modules) = &filter.modules {
            let module_index = self.index_by_module.read().await;
            let mut module_candidates = Vec::new();

            for module in modules {
                if let Some(indices) = module_index.get(module) {
                    module_candidates.extend(indices);
                }
            }

            candidates = match candidates {
                Some(existing) => {
                    Some(existing.into_iter().filter(|i| module_candidates.contains(i)).collect())
                }
                None => Some(module_candidates),
            };
        }

        // 如果没有任何过滤条件，返回所有索引
        candidates.unwrap_or_else(|| {
            let cache = self.log_cache.read().await;
            (0..cache.len()).collect()
        })
    }

    /// 检查条目是否匹配过滤器
    async fn matches_filter(&self, entry: &LogEntry, filter: &LogFilter) -> bool {
        // 消息模式匹配
        if let Some(pattern) = &filter.message_pattern {
            let regex = self.get_cached_regex(pattern).await;
            if !regex.is_match(&entry.message) {
                return false;
            }
        }

        // 字段过滤
        for (field_name, field_value) in &filter.field_filters {
            if let Some(value) = entry.fields.get(field_name) {
                if value != field_value {
                    return false;
                }
            } else {
                return false;
            }
        }

        true
    }

    /// 获取缓存的正则表达式
    async fn get_cached_regex(&self, pattern: &str) -> Regex {
        let mut cache = self.regex_cache.write().await;

        if let Some(regex) = cache.get(pattern) {
            regex.clone()
        } else {
            let regex = Regex::new(pattern).unwrap_or_else(|_| {
                warn!("Invalid regex pattern: {}", pattern);
                Regex::new(".*").unwrap()
            });
            cache.insert(pattern.to_string(), regex.clone());
            regex
        }
    }

    /// 获取日志统计信息
    pub async fn get_log_stats(&self, time_range: Option<TimeRange>) -> Result<LogStats> {
        let cache = self.log_cache.read().await;
        let mut logs_by_level = HashMap::new();
        let mut logs_by_module = HashMap::new();
        let mut logs_in_time_range = 0;
        let mut error_logs = 0;
        let mut warning_logs = 0;
        let mut recent_errors = Vec::new();

        let now = Utc::now();
        let error_cutoff = now - Duration::from_secs(3600); // 最近1小时的错误

        for entry in cache.iter() {
            // 级别统计
            *logs_by_level.entry(entry.level.clone()).or_insert(0) += 1;

            // 模块统计
            if let Some(module) = &entry.module {
                *logs_by_module.entry(module.clone()).or_insert(0) += 1;
            }

            // 时间范围统计
            if let Some(range) = &time_range {
                if entry.timestamp >= range.start && entry.timestamp <= range.end {
                    logs_in_time_range += 1;
                }
            }

            // 错误和警告统计
            match entry.level {
                LogLevel::Error => {
                    error_logs += 1;
                    if entry.timestamp >= error_cutoff {
                        recent_errors.push(entry.clone());
                    }
                }
                LogLevel::Warning => {
                    warning_logs += 1;
                }
                _ => {}
            }
        }

        // 限制最近错误数量
        recent_errors.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        recent_errors.truncate(10);

        Ok(LogStats {
            total_logs: cache.len() as u64,
            logs_by_level,
            logs_by_module,
            logs_in_time_range,
            error_logs,
            warning_logs,
            recent_errors,
        })
    }

    /// 清理过期日志
    pub async fn cleanup_old_logs(&self) -> Result<usize> {
        let mut cache = self.log_cache.write().await;
        let cutoff_time = Utc::now() - chrono::Duration::from_std(self.config.retention_period)?;
        let initial_count = cache.len();

        // 移除过期日志
        cache.retain(|entry| entry.timestamp > cutoff_time);

        let removed_count = initial_count - cache.len();
        if removed_count > 0 {
            info!("Cleaned up {} old log entries", removed_count);
            // 重建索引
            self.rebuild_indices().await;
        }

        Ok(removed_count)
    }

    /// 重建索引
    async fn rebuild_indices(&self) {
        let cache = self.log_cache.read().await;

        // 清空现有索引
        self.index_by_level.write().await.clear();
        self.index_by_module.write().await.clear();
        self.index_by_time.write().await.clear();

        // 重建索引
        for (index, entry) in cache.iter().enumerate() {
            if self.config.enable_indexing {
                self.update_indices(entry, index).await;
            }
        }
    }

    /// 导出日志
    pub async fn export_logs(&self, filter: &LogFilter, format: ExportFormat) -> Result<String> {
        let logs = self.query_logs(filter).await?;

        match format {
            ExportFormat::Json => {
                serde_json::to_string_pretty(&logs)
                    .context("Failed to serialize logs to JSON")
            }
            ExportFormat::Csv => {
                let mut csv_output = String::new();
                csv_output.push_str("timestamp,level,module,message\n");

                for log in logs {
                    csv_output.push_str(&format!(
                        "{},{},{},{}\n",
                        log.timestamp,
                        log.level,
                        log.module.unwrap_or_else(|| "unknown".to_string()),
                        log.message.replace('\n', " ")
                    ));
                }

                Ok(csv_output)
            }
            ExportFormat::Text => {
                let mut text_output = String::new();

                for log in logs {
                    text_output.push_str(&format!(
                        "[{}] [{}] [{}] {}\n",
                        log.timestamp.format("%Y-%m-%d %H:%M:%S"),
                        log.level,
                        log.module.unwrap_or_else(|| "unknown".to_string()),
                        log.message
                    ));
                }

                Ok(text_output)
            }
        }
    }
}

/// 导出格式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExportFormat {
    Json,
    Csv,
    Text,
}

impl LogAnalyzer {
    /// 创建新的日志分析器
    pub fn new() -> Self {
        Self {
            error_patterns: vec![
                ErrorPattern {
                    name: "Database Connection Error".to_string(),
                    pattern: Regex::new(r"(?i)database.*connection.*error|failed.*connect.*database").unwrap(),
                    severity: LogLevel::Error,
                    description: "数据库连接错误".to_string(),
                },
                ErrorPattern {
                    name: "Out of Memory".to_string(),
                    pattern: Regex::new(r"(?i)out of memory|cannot allocate|memory exhausted").unwrap(),
                    severity: LogLevel::Error,
                    description: "内存不足".to_string(),
                },
                ErrorPattern {
                    name: "File System Error".to_string(),
                    pattern: Regex::new(r"(?i)no such file|permission denied|disk full").unwrap(),
                    severity: LogLevel::Error,
                    description: "文件系统错误".to_string(),
                },
            ],
            performance_analyzer: PerformanceAnalyzer {
                slow_query_threshold: Duration::from_secs(5),
                response_time_stats: Arc::new(RwLock::new(VecDeque::new())),
            },
        }
    }

    /// 分析日志中的错误模式
    pub async fn analyze_error_patterns(&self, logs: &[LogEntry]) -> Vec<ErrorAnalysis> {
        let mut analyses = Vec::new();

        for pattern in &self.error_patterns {
            let mut matching_logs = Vec::new();

            for log in logs {
                if pattern.pattern.is_match(&log.message) {
                    matching_logs.push(log.clone());
                }
            }

            if !matching_logs.is_empty() {
                analyses.push(ErrorAnalysis {
                    pattern_name: pattern.name.clone(),
                    severity: pattern.severity.clone(),
                    description: pattern.description.clone(),
                    count: matching_logs.len(),
                    first_occurrence: matching_logs.iter().map(|l| l.timestamp).min().unwrap(),
                    last_occurrence: matching_logs.iter().map(|l| l.timestamp).max().unwrap(),
                    sample_logs: matching_logs.into_iter().take(5).collect(),
                });
            }
        }

        analyses
    }

    /// 分析性能趋势
    pub async fn analyze_performance(&self) -> PerformanceAnalysis {
        let stats = self.performance_analyzer.response_time_stats.read().await;

        if stats.is_empty() {
            return PerformanceAnalysis {
                avg_response_time: Duration::ZERO,
                max_response_time: Duration::ZERO,
                min_response_time: Duration::ZERO,
                slow_queries: 0,
                total_queries: 0,
            };
        }

        let total: Duration = stats.iter().sum();
        let avg = total / stats.len() as u32;
        let max = *stats.iter().max().unwrap();
        let min = *stats.iter().min().unwrap();
        let slow_count = stats.iter().filter(|&&d| d > self.performance_analyzer.slow_query_threshold).count();

        PerformanceAnalysis {
            avg_response_time: avg,
            max_response_time: max,
            min_response_time: min,
            slow_queries: slow_count,
            total_queries: stats.len(),
        }
    }
}

/// 错误分析结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorAnalysis {
    /// 模式名称
    pub pattern_name: String,
    /// 严重级别
    pub severity: LogLevel,
    /// 描述
    pub description: String,
    /// 匹配数量
    pub count: usize,
    /// 首次出现
    pub first_occurrence: DateTime<Utc>,
    /// 最后出现
    pub last_occurrence: DateTime<Utc>,
    /// 示例日志
    pub sample_logs: Vec<LogEntry>,
}

/// 性能分析结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceAnalysis {
    /// 平均响应时间
    pub avg_response_time: Duration,
    /// 最大响应时间
    pub max_response_time: Duration,
    /// 最小响应时间
    pub min_response_time: Duration,
    /// 慢查询数量
    pub slow_queries: usize,
    /// 总查询数量
    pub total_queries: usize,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            max_cache_size: 100000,
            retention_period: Duration::from_secs(7 * 24 * 60 * 60), // 7天
            enable_indexing: true,
            index_update_interval: Duration::from_secs(60),
            compress_old_logs: true,
            rotation: LogRotationConfig {
                enabled: true,
                max_file_size: 100 * 1024 * 1024, // 100MB
                max_files: 10,
                rotation_interval: Duration::from_secs(24 * 60 * 60), // 1天
            },
        }
    }
}

impl Default for LogAggregator {
    fn default() -> Self {
        Self::new(LogConfig::default())
    }
}

impl Default for LogAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}