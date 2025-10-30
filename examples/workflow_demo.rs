//! 工作流引擎演示程序
//!
//! 展示工作流引擎的核心功能，包括状态转换、自动路由、工作列表管理和危急值处理

use pacs_core::utils::generate_dicom_uid;
use pacs_core::{Study, StudyStatus};
use pacs_workflow::routing::{RoutingRule, RuleAction, RuleCondition};
use pacs_workflow::{
    CriticalSeverity, CriticalValueType, Radiologist, RadiologistSpecialty, RoutingPriority,
    WorkflowEngine,
};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt::init();

    // 创建工作流引擎
    let mut workflow_engine = WorkflowEngine::new();

    println!("🚀 PACS 工作流引擎演示\n");

    // 1. 设置医生信息
    setup_radiologists(&mut workflow_engine)?;
    println!("✅ 医生信息设置完成");

    // 2. 设置路由规则
    setup_routing_rules(&mut workflow_engine)?;
    println!("✅ 路由规则设置完成");

    // 3. 创建示例检查
    let studies = create_sample_studies()?;
    println!("✅ 创建了 {} 个示例检查", studies.len());

    // 4. 处理新的检查
    for (i, study) in studies.iter().enumerate() {
        let priority = match i {
            0 => RoutingPriority::Emergency,
            1 => RoutingPriority::Urgent,
            2 => RoutingPriority::Routine,
            _ => RoutingPriority::Low,
        };

        println!("📋 处理检查 {} (优先级: {:?})", study.id, priority);
        workflow_engine
            .process_new_study(study.clone(), priority.clone())
            .await?;
    }

    // 5. 显示系统概览
    let overview = workflow_engine.get_system_overview();
    println!("\n📊 系统概览:");
    println!("   活跃工作项: {}", overview.total_active_work_items);
    println!(
        "   未确认危急值: {}",
        overview.total_unacknowledged_critical_values
    );
    println!("   可用医生数: {}", overview.available_radiologists_count);
    println!("   系统负载: {:.1}%", overview.system_load * 100.0);

    // 6. 显示医生工作列表
    let available_radiologists = workflow_engine
        .routing_engine()
        .get_available_radiologists();
    for radiologist in available_radiologists {
        let worklist = workflow_engine.get_radiologist_worklist(radiologist.id)?;
        let stats = workflow_engine.get_worklist_stats(Some(radiologist.id));

        println!("\n👨‍⚕️ 医生 {} 的工作列表:", radiologist.name);
        println!("   总工作项: {}", stats.total_items);
        println!("   待处理: {}", stats.pending_items);
        println!("   进行中: {}", stats.in_progress_items);
        println!("   已完成: {}", stats.completed_items);

        for work_item in worklist {
            println!(
                "   - {} ({}): {:?}",
                work_item.id,
                work_item.tags.join(", "),
                work_item.status
            );
        }
    }

    // 7. 模拟检查状态更新
    if let Some(study) = studies.first() {
        println!(
            "\n🔄 更新检查状态: {:?} -> {:?}",
            StudyStatus::Scheduled,
            StudyStatus::InProgress
        );
        let new_status = workflow_engine
            .update_study_status(
                study.id,
                StudyStatus::Scheduled,
                pacs_workflow::state_machine::StudyEvent::Started,
            )
            .await?;

        println!("✅ 检查状态已更新为: {:?}", new_status);
    }

    // 8. 创建危急值事件
    if let Some(study) = studies.get(1) {
        println!("\n🚨 创建危急值事件");
        workflow_engine
            .create_critical_value(
                study.id,
                study.patient_id,
                CriticalValueType::Emergency,
                "发现急性颅内出血".to_string(),
                Uuid::new_v4(), // 检测者ID
                CriticalSeverity::Critical,
                Some("患者意识模糊，需要立即处理".to_string()),
            )
            .await?;

        println!("✅ 危急值事件已创建");
    }

    // 9. 处理通知队列
    workflow_engine.process_notifications().await?;
    println!("✅ 通知队列已处理");

    // 10. 检查升级条件
    let escalations = workflow_engine.check_escalations()?;
    if !escalations.is_empty() {
        println!("\n⚠️  发现 {} 个需要升级的情况", escalations.len());
    }

    // 11. 显示未确认的危急值
    let unacknowledged = workflow_engine.get_unacknowledged_critical_values();
    if !unacknowledged.is_empty() {
        println!("\n🚨 未确认的危急值事件:");
        for event in unacknowledged {
            println!(
                "   - 事件 {}: {} ({:?})",
                event.id, event.description, event.severity
            );
        }
    }

    println!("\n🎉 工作流引擎演示完成!");
    Ok(())
}

/// 设置医生信息
fn setup_radiologists(
    workflow_engine: &mut WorkflowEngine,
) -> Result<(), Box<dyn std::error::Error>> {
    // 神经放射科医生
    let neuro_radiologist = Radiologist {
        id: Uuid::new_v4(),
        name: "张医生".to_string(),
        specialties: vec![
            RadiologistSpecialty::Neuroradiology,
            RadiologistSpecialty::General,
        ],
        max_workload: 5,
        is_available: true,
    };

    // 骨科放射科医生
    let msk_radiologist = Radiologist {
        id: Uuid::new_v4(),
        name: "李医生".to_string(),
        specialties: vec![
            RadiologistSpecialty::Musculoskeletal,
            RadiologistSpecialty::General,
        ],
        max_workload: 4,
        is_available: true,
    };

    // 通用放射科医生
    let general_radiologist = Radiologist {
        id: Uuid::new_v4(),
        name: "王医生".to_string(),
        specialties: vec![RadiologistSpecialty::General],
        max_workload: 6,
        is_available: true,
    };

    workflow_engine
        .routing_engine_mut()
        .add_radiologist(neuro_radiologist);
    workflow_engine
        .routing_engine_mut()
        .add_radiologist(msk_radiologist);
    workflow_engine
        .routing_engine_mut()
        .add_radiologist(general_radiologist);

    Ok(())
}

/// 设置路由规则
fn setup_routing_rules(
    workflow_engine: &mut WorkflowEngine,
) -> Result<(), Box<dyn std::error::Error>> {
    // 紧急CT扫描规则
    let emergency_ct_rule = RoutingRule {
        id: Uuid::new_v4(),
        name: "紧急CT扫描".to_string(),
        priority: 100,
        conditions: vec![
            RuleCondition::ModalityEquals("CT".to_string()),
            RuleCondition::Emergency,
        ],
        action: RuleAction::AssignToSpecialty(RadiologistSpecialty::Neuroradiology),
        is_active: true,
    };

    // 常规X光规则
    let routine_xray_rule = RoutingRule {
        id: Uuid::new_v4(),
        name: "常规X光检查".to_string(),
        priority: 50,
        conditions: vec![
            RuleCondition::ModalityEquals("XR".to_string()),
            RuleCondition::Routine,
        ],
        action: RuleAction::AssignToSpecialty(RadiologistSpecialty::General),
        is_active: true,
    };

    // MRI神经扫描规则
    let mri_neuro_rule = RoutingRule {
        id: Uuid::new_v4(),
        name: "MRI神经扫描".to_string(),
        priority: 80,
        conditions: vec![
            RuleCondition::ModalityEquals("MR".to_string()),
            RuleCondition::DescriptionContains("头部".to_string()),
        ],
        action: RuleAction::AssignToSpecialty(RadiologistSpecialty::Neuroradiology),
        is_active: true,
    };

    workflow_engine
        .routing_engine_mut()
        .add_rule(emergency_ct_rule);
    workflow_engine
        .routing_engine_mut()
        .add_rule(routine_xray_rule);
    workflow_engine
        .routing_engine_mut()
        .add_rule(mri_neuro_rule);

    Ok(())
}

/// 创建示例检查数据
fn create_sample_studies() -> Result<Vec<Study>, Box<dyn std::error::Error>> {
    let patient1_id = Uuid::new_v4();
    let patient2_id = Uuid::new_v4();
    let patient3_id = Uuid::new_v4();

    let today = chrono::Utc::now().date_naive();

    let study1 = Study {
        id: Uuid::new_v4(),
        patient_id: patient1_id,
        accession_number: "ACC20231030001".to_string(),
        study_uid: generate_dicom_uid(),
        study_date: today,
        study_time: Some(chrono::NaiveTime::from_hms_opt(9, 30, 0).unwrap()),
        modality: "CT".to_string(),
        description: Some("头部CT扫描 - 紧急".to_string()),
        status: StudyStatus::Scheduled,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let study2 = Study {
        id: Uuid::new_v4(),
        patient_id: patient2_id,
        accession_number: "ACC20231030002".to_string(),
        study_uid: generate_dicom_uid(),
        study_date: today,
        study_time: Some(chrono::NaiveTime::from_hms_opt(10, 15, 0).unwrap()),
        modality: "MR".to_string(),
        description: Some("头部MRI检查".to_string()),
        status: StudyStatus::Scheduled,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let study3 = Study {
        id: Uuid::new_v4(),
        patient_id: patient3_id,
        accession_number: "ACC20231030003".to_string(),
        study_uid: generate_dicom_uid(),
        study_date: today,
        study_time: Some(chrono::NaiveTime::from_hms_opt(11, 0, 0).unwrap()),
        modality: "XR".to_string(),
        description: Some("胸部X光检查".to_string()),
        status: StudyStatus::Scheduled,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    Ok(vec![study1, study2, study3])
}
