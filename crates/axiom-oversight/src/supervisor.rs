use axiom_core::id::CellId;
use std::sync::Arc;

pub struct OversightSupervisor {
    _id: CellId,
    pub(crate) architecture_guardian: Arc<super::ArchitectureGuardianCell>,
    pub(crate) entropy_governor: Arc<super::EntropyGovernorCell>,
    pub(crate) resource_manager: Arc<super::ResourceManagerCell>,
    pub(crate) intent_auditor: Arc<super::IntentAuditorCell>,
    pub(crate) compliance_guard: Arc<super::ComplianceGuardCell>,
    pub(crate) meta_oversight: Arc<super::MetaOversightCell>,
    pub(crate) health_collector: Arc<super::HealthCollectorCell>,
    pub(crate) loop_detector: Arc<super::LoopDetector>,
    pub(crate) startup: Arc<super::StartupVerification>,
}

impl OversightSupervisor {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            _id: CellId::new("oversight:supervisor"),
            architecture_guardian: Arc::new(super::ArchitectureGuardianCell::new()),
            entropy_governor: Arc::new(super::EntropyGovernorCell::new(30.0, 70.0, 150.0, 300.0)),
            resource_manager: Arc::new(super::ResourceManagerCell::new(1000, 100.0, 64)),
            intent_auditor: Arc::new(super::IntentAuditorCell::new()),
            compliance_guard: Arc::new(super::ComplianceGuardCell::new()),
            meta_oversight: Arc::new(super::MetaOversightCell::new()),
            health_collector: Arc::new(super::HealthCollectorCell::new()),
            loop_detector: Arc::new(super::LoopDetector::new()),
            startup: Arc::new(crate::startup::builtin_startup_verification()),
        })
    }

    pub fn architecture_guardian(&self) -> Arc<super::ArchitectureGuardianCell> {
        self.architecture_guardian.clone()
    }
    pub fn entropy_governor(&self) -> Arc<super::EntropyGovernorCell> {
        self.entropy_governor.clone()
    }
    pub fn resource_manager(&self) -> Arc<super::ResourceManagerCell> {
        self.resource_manager.clone()
    }
    pub fn intent_auditor(&self) -> Arc<super::IntentAuditorCell> {
        self.intent_auditor.clone()
    }
    pub fn compliance_guard(&self) -> Arc<super::ComplianceGuardCell> {
        self.compliance_guard.clone()
    }
    pub fn meta_oversight(&self) -> Arc<super::MetaOversightCell> {
        self.meta_oversight.clone()
    }
    pub fn health_collector(&self) -> Arc<super::HealthCollectorCell> {
        self.health_collector.clone()
    }
    pub fn loop_detector(&self) -> Arc<super::LoopDetector> {
        self.loop_detector.clone()
    }
    pub fn startup(&self) -> &super::StartupVerification {
        &self.startup
    }
}
