//! 系统管理和监控演示程序
//!
//! 展示PACS系统管理和监控模块的各种功能

use pacs_admin::{SystemManager, monitoring::*, alerting::*, logging::*, performance::*};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("🏥 PACS系统管理和监控演示");
    println!("================================");

    // 创建系统管理器
    println!("\n📊 初始化系统管理器...");
    let system_manager = SystemManager::new("config/default.toml").await?;

    // 启动系统管理器
    system_manager.start().await?;

    // 演示监控功能
    demo_monitoring(&system_manager).await?;

    // 演示告警功能
    demo_alerting(&system_manager).await?;

    // 演示日志聚合功能
    demo_logging(&system_manager).await?;

    // 演示性能分析功能
    demo_performance_analysis(&system_manager).await?;

    // 演示配置管理功能
    demo_config_management(&system_manager).await?;

    // 生成系统状态报告
    println!("\n📋 生成系统状态报告...");
    let status_report = system_manager.generate_status_report().await?;
    print_status_report(&status_report);

    // 运行一段时间以展示实时监控
    println!("\n⏰ 运行实时监控（30秒）...");
    sleep(Duration::from_secs(30)).await;

    // 停止系统管理器
    system_manager.stop().await?;

    println!("\n✅ 演示完成！");
    Ok(())
}

/// 演示监控功能
async fn demo_monitoring(system_manager: &SystemManager) -> anyhow::Result<()> {
    println!("\n🔍 监控功能演示");
    println!("------------------");

    let monitor = system_manager.system_monitor();

    // 模拟HTTP请求
    monitor.record_http_request("GET", "/api/patients", 200, Duration::from_millis(150));
    monitor.record_http_request("POST", "/api/studies", 201, Duration::from_millis(250));
    monitor.record_http_request("GET", "/api/instances", 404, Duration::from_millis(50));

    // 更新活跃连接数
    monitor.update_active_connections(15);

    // 记录DICOM操作
    monitor.record_dicom_operation("C-STORE");
    monitor.record_dicom_operation("C-FIND");
    monitor.record_dicom_operation("C-ECHO");

    // 更新数据库连接池状态
    monitor.update_db_connections(8, 12);

    // 更新存储使用情况
    monitor.update_storage_usage(1024 * 1024 * 1024 * 500); // 500GB

    // 更新系统资源使用情况
    monitor.update_system_metrics(45.2, 1024 * 1024 * 1024 * 8, 65.8);

    // 获取系统健康状态
    let health_status = monitor.get_health_status().await;
    print_health_status(&health_status);

    // 获取Prometheus指标
    let metrics = monitor.get_prometheus_metrics()?;
    println!("\n📈 Prometheus指标（前200字符）:");
    println!("{}", &metrics[..200.min(metrics.len())]);

    Ok(())
}

/// 演示告警功能
async fn demo_alerting(system_manager: &SystemManager) -> anyhow::Result<()> {
    println!("\n🚨 告警功能演示");
    println!("------------------");

    let alert_manager = system_manager.alert_manager();

    // 添加告警规则
    let cpu_rule = AlertRule {
        name: "High CPU Usage".to_string(),
        metric: "cpu_usage".to_string(),
        threshold: 80.0,
        operator: ComparisonOperator::GreaterThan,
        severity: AlertSeverity::Warning,
        duration: Duration::from_secs(300),
        message_template: "CPU usage is ${current}%, exceeding threshold of ${threshold}%".to_string(),
        enabled: true,
    };

    let memory_rule = AlertRule {
        name: "High Memory Usage".to_string(),
        metric: "memory_usage".to_string(),
        threshold: 90.0,
        operator: ComparisonOperator::GreaterThan,
        severity: AlertSeverity::Critical,
        duration: Duration::from_secs(180),
        message_template: "Memory usage is critically high at ${current}%".to_string(),
        enabled: true,
    };

    alert_manager.add_rule(cpu_rule).await?;
    alert_manager.add_rule(memory_rule).await?;

    println!("✅ 已添加告警规则：High CPU Usage, High Memory Usage");

    // 评估告警规则
    let triggered_alerts = alert_manager.evaluate_rules().await?;
    println!("\n🔔 触发的告警数量: {}", triggered_alerts.len());

    // 获取告警统计
    let alert_stats = alert_manager.get_alert_stats().await;
    print_alert_stats(&alert_stats);

    Ok(())
}

/// 演示日志聚合功能
async fn demo_logging(system_manager: &SystemManager) -> anyhow::Result<()> {
    println!("\n📝 日志聚合功能演示");
    println!("----------------------");

    let log_aggregator = system_manager.log_aggregator();

    // 添加一些示例日志条目
    let log_entries = vec![
        LogEntry {
            id: "log-001".to_string(),
            timestamp: chrono::Utc::now(),
            level: LogLevel::Info,
            message: "DICOM C-STORE operation completed successfully".to_string(),
            module: Some("pacs-dicom".to_string()),
            target: Some("dicom_service".to_string()),
            file: Some("src/services.rs".to_string()),
            line: Some(123),
            thread: Some("main".to_string()),
            fields: {
                let mut fields = std::collections::HashMap::new();
                fields.insert("operation".to_string(), "C-STORE".to_string());
                fields.insert("patient_id".to_string(), "PAT-001".to_string());
                fields
            },
            stack_trace: None,
        },
        LogEntry {
            id: "log-002".to_string(),
            timestamp: chrono::Utc::now(),
            level: LogLevel::Warning,
            message: "Database connection pool approaching maximum capacity".to_string(),
            module: Some("pacs-database".to_string()),
            target: Some("connection_pool".to_string()),
            file: Some("src/connection.rs".to_string()),
            line: Some(456),
            thread: Some("worker-1".to_string()),
            fields: {
                let mut fields = std::collections::HashMap::new();
                fields.insert("active_connections".to_string(), "18".to_string());
                fields.insert("max_connections".to_string(), "20".to_string());
                fields
            },
            stack_trace: None,
        },
        LogEntry {
            id: "log-003".to_string(),
            timestamp: chrono::Utc::now(),
            level: LogLevel::Error,
            message: "Failed to process DICOM file: Invalid transfer syntax".to_string(),
            module: Some("pacs-dicom".to_string()),
            target: Some("parser".to_string()),
            file: Some("src/parser.rs".to_string()),
            line: Some(789),
            thread: Some("worker-2".to_string()),
            fields: {
                let mut fields = std::collections::HashMap::new();
                fields.insert("file_path".to_string(), "/data/invalid.dcm".to_string());
                fields.insert("error_code".to_string(), "INVALID_SYNTAX".to_string());
                fields
            },
            stack_trace: Some("  at parser::parse_file (src/parser.rs:789)\n  at service::handle_store (src/services.rs:234)".to_string()),
        },
    ];

    for entry in log_entries {
        log_aggregator.add_log_entry(entry).await?;
    }

    println!("✅ 已添加3条示例日志条目");

    // 查询日志
    let filter = LogFilter {
        time_range: None,
        levels: Some(vec![LogLevel::Error, LogLevel::Warning]),
        modules: None,
        message_pattern: None,
        field_filters: std::collections::HashMap::new(),
        limit: Some(10),
        sort_order: SortOrder::Descending,
    };

    let filtered_logs = log_aggregator.query_logs(&filter).await?;
    println!("\n🔍 查询结果（错误和警告日志）: {} 条", filtered_logs.len());

    // 获取日志统计
    let log_stats = log_aggregator.get_log_stats(None).await?;
    print_log_stats(&log_stats);

    Ok(())
}

/// 演示性能分析功能
async fn demo_performance_analysis(system_manager: &SystemManager) -> anyhow::Result<()> {
    println!("\n⚡ 性能分析功能演示");
    println!("----------------------");

    let performance_monitor = system_manager.performance_monitor();

    // 收集性能指标
    let metrics = performance_monitor.collect_metrics().await?;
    print_performance_metrics(&metrics);

    // 生成性能报告
    let time_range = TimeRange {
        start: chrono::Utc::now() - chrono::Duration::hours(1),
        end: chrono::Utc::now(),
    };

    match performance_monitor.generate_performance_report(time_range).await {
        Ok(report) => {
            print_performance_report(&report);
        }
        Err(e) => {
            println!("⚠️  无法生成性能报告: {}", e);
        }
    }

    Ok(())
}

/// 演示配置管理功能
async fn demo_config_management(system_manager: &SystemManager) -> anyhow::Result<()> {
    println!("\n⚙️  配置管理功能演示");
    println!("----------------------");

    let config_manager = system_manager.config_manager();

    // 获取配置
    let config = config_manager.get_config().await;
    println!("📋 当前配置:");
    println!("  服务器名称: {}", config.server.name);
    println!("  监听端口: {}", config.server.port);
    println!("  数据库连接: {}", config.database.connection_string);
    println!("  监控状态: {}", if config.monitoring.enabled { "启用" } else { "禁用" });

    // 验证配置
    match config_manager.validate_config().await {
        Ok(()) => println!("✅ 配置验证通过"),
        Err(e) => println!("❌ 配置验证失败: {}", e),
    }

    // 获取特定配置值
    match config_manager.get_value::<String>("server.name").await {
        Ok(value) => println!("📖 服务器名称: {}", value),
        Err(e) => println!("❌ 获取配置失败: {}", e),
    }

    Ok(())
}

/// 打印健康状态
fn print_health_status(health_status: &HealthStatus) {
    println!("\n🏥 系统健康状态:");
    println!("  总体状态: {:?}", health_status.status);
    println!("  运行时间: {:?}", health_status.uptime);
    println!("  检查时间: {}", health_status.timestamp);

    for (component_name, component_health) in &health_status.components {
        println!("  {}: {:?} - {}", component_name, component_health.status, component_health.message);
        if let Some(response_time) = component_health.response_time {
            println!("    响应时间: {:?}", response_time);
        }
    }
}

/// 打印告警统计
fn print_alert_stats(alert_stats: &AlertStats) {
    println!("\n📊 告警统计:");
    println!("  总告警数: {}", alert_stats.total_alerts);
    println!("  活跃告警数: {}", alert_stats.active_alerts);
    println!("  今日告警数: {}", alert_stats.alerts_today);
    println!("  本周告警数: {}", alert_stats.alerts_this_week);

    println!("  按严重级别统计:");
    for (severity, count) in &alert_stats.alerts_by_severity {
        println!("    {:?}: {}", severity, count);
    }
}

/// 打印日志统计
fn print_log_stats(log_stats: &LogStats) {
    println!("\n📊 日志统计:");
    println!("  总日志数: {}", log_stats.total_logs);
    println!("  错误日志数: {}", log_stats.error_logs);
    println!("  警告日志数: {}", log_stats.warning_logs);

    println!("  按级别统计:");
    for (level, count) in &log_stats.logs_by_level {
        println!("    {}: {}", level, count);
    }

    if !log_stats.recent_errors.is_empty() {
        println!("  最近错误日志（{}条）:", log_stats.recent_errors.len());
        for (i, log) in log_stats.recent_errors.iter().take(3).enumerate() {
            println!("    {}. [{}] {}", i + 1, log.timestamp.format("%H:%M:%S"), log.message);
        }
    }
}

/// 打印性能指标
fn print_performance_metrics(metrics: &PerformanceMetrics) {
    println!("\n📊 性能指标:");
    println!("  CPU使用率: {:.1}%", metrics.cpu_usage);
    println!("  内存使用: {:.1}% ({}GB / {}GB)",
        metrics.memory.usage_percent,
        metrics.memory.used_bytes as f64 / (1024.0 * 1024.0 * 1024.0),
        metrics.memory.total_bytes as f64 / (1024.0 * 1024.0 * 1024.0)
    );
    println!("  磁盘使用: {:.1}%", metrics.disk_io.usage_percent);
    println!("  磁盘IOPS: {}", metrics.disk_io.iops);
    println!("  网络连接数: {}", metrics.network_io.connections);
    println!("  数据库连接: {} 活跃, {} 空闲", metrics.database.active_connections, metrics.database.idle_connections);
    println!("  HTTP请求数: {}", metrics.application.http_requests);
    println!("  平均响应时间: {:?}", metrics.application.avg_response_time);
    println!("  错误率: {:.2}%", metrics.application.error_rate);
}

/// 打印性能报告
fn print_performance_report(report: &PerformanceReport) {
    println!("\n📊 性能分析报告:");
    println!("  生成时间: {}", report.generated_at);
    println!("  总体健康状态: {:?}", report.overall_health);

    println!("  资源使用情况:");
    println!("    CPU: 平均 {:.1}%, 最大 {:.1}%, 趋势: {:?}",
        report.resource_analysis.cpu.avg_usage,
        report.resource_analysis.cpu.max_usage,
        report.resource_analysis.cpu.usage_trend
    );
    println!("    内存: 平均 {:.1}%, 最大 {:.1}%, 趋势: {:?}",
        report.resource_analysis.memory.avg_usage,
        report.resource_analysis.memory.max_usage,
        report.resource_analysis.memory.usage_trend
    );

    if !report.trends.is_empty() {
        println!("  性能趋势:");
        for trend in &report.trends {
            println!("    {}: {:?} ({:.1}% 变化率)", trend.metric_name, trend.direction, trend.change_rate * 100.0);
        }
    }

    if !report.bottlenecks.is_empty() {
        println!("  识别的瓶颈:");
        for bottleneck in &report.bottlenecks {
            println!("    {:?}: {:?} - {}", bottleneck.bottleneck_type, bottleneck.impact, bottleneck.description);
        }
    }

    if !report.recommendations.is_empty() {
        println!("  优化建议:");
        for recommendation in &report.recommendations {
            println!("    {:?}: {} ({})", recommendation.recommendation_type, recommendation.description, recommendation.expected_impact);
        }
    }
}

/// 打印系统状态报告
fn print_status_report(report: &SystemStatusReport) {
    println!("\n📋 系统状态报告");
    println!("================");
    println!("生成时间: {}", report.timestamp);

    print_health_status(&report.health_status);
    print_performance_metrics(&report.performance_metrics);
    print_alert_stats(&report.alert_stats);
    print_log_stats(&report.log_stats);
}