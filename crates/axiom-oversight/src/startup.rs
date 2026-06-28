//! Startup Verification Chain - checks run before the runtime goes live.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum StartupError {
    Blocking(String),
    Warning(String),
}

impl std::fmt::Display for StartupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StartupError::Blocking(m) => write!(f, "BLOCKING: {}", m),
            StartupError::Warning(m) => write!(f, "WARNING: {}", m),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    pub name: String,
    pub passed: bool,
    pub error: Option<StartupError>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationReport {
    pub all_passed: bool,
    pub blocking_failures: Vec<CheckResult>,
    pub warnings: Vec<CheckResult>,
    pub passed: Vec<CheckResult>,
    pub total_duration_ms: u64,
}

pub trait StartupCheck: Send + Sync {
    fn name(&self) -> &'static str;
    fn check(&self) -> Result<(), StartupError>;
}

pub struct StartupVerification {
    checks: Vec<Box<dyn StartupCheck>>,
}

impl StartupVerification {
    pub fn new() -> Self {
        Self { checks: Vec::new() }
    }

    pub fn add_check(&mut self, check: Box<dyn StartupCheck>) {
        self.checks.push(check);
    }

    pub fn run(&self) -> VerificationReport {
        let start = std::time::Instant::now();
        let mut passed = Vec::new();
        let mut warnings = Vec::new();
        let mut blocking = Vec::new();

        for c in &self.checks {
            let t0 = std::time::Instant::now();
            let res = c.check();
            let dur = t0.elapsed().as_millis() as u64;
            let name = c.name().to_string();
            match res {
                Ok(()) => passed.push(CheckResult {
                    name,
                    passed: true,
                    error: None,
                    duration_ms: dur,
                }),
                Err(StartupError::Warning(m)) => warnings.push(CheckResult {
                    name,
                    passed: true,
                    error: Some(StartupError::Warning(m)),
                    duration_ms: dur,
                }),
                Err(StartupError::Blocking(m)) => blocking.push(CheckResult {
                    name,
                    passed: false,
                    error: Some(StartupError::Blocking(m)),
                    duration_ms: dur,
                }),
            }
        }

        VerificationReport {
            all_passed: blocking.is_empty(),
            blocking_failures: blocking,
            warnings,
            passed,
            total_duration_ms: start.elapsed().as_millis() as u64,
        }
    }
}

impl Default for StartupVerification {
    fn default() -> Self {
        Self::new()
    }
}

pub struct LayerCanSendToCheck;
impl StartupCheck for LayerCanSendToCheck {
    fn name(&self) -> &'static str {
        "layer-direction-matrix"
    }
    fn check(&self) -> Result<(), StartupError> {
        use axiom_core::layer::Layer;
        let legal_pairs = [
            (Layer::Oversight, Layer::Oversight),
            (Layer::Oversight, Layer::Agent),
            (Layer::Oversight, Layer::Validate),
            (Layer::Oversight, Layer::Exec),
            (Layer::Agent, Layer::Agent),
            (Layer::Agent, Layer::Validate),
            (Layer::Validate, Layer::Validate),
            (Layer::Validate, Layer::Exec),
            (Layer::Exec, Layer::Exec),
        ];
        for (a, b) in &legal_pairs {
            if !a.can_send_to(*b) {
                return Err(StartupError::Blocking(format!(
                    "expected legal pair {:?} -> {:?} not allowed",
                    a, b
                )));
            }
        }
        let illegal_pairs = [
            (Layer::Exec, Layer::Agent),
            (Layer::Exec, Layer::Oversight),
            (Layer::Validate, Layer::Oversight),
            (Layer::Validate, Layer::Agent),
            (Layer::Agent, Layer::Oversight),
        ];
        for (a, b) in &illegal_pairs {
            if a.can_send_to(*b) {
                return Err(StartupError::Blocking(format!(
                    "expected illegal pair {:?} -> {:?} allowed (violation)",
                    a, b
                )));
            }
        }
        Ok(())
    }
}

pub struct AxiomRegistryCheck;
impl StartupCheck for AxiomRegistryCheck {
    fn name(&self) -> &'static str {
        "axiom-registry"
    }
    fn check(&self) -> Result<(), StartupError> {
        let count = axiom_core::registry::AXIOM_REGISTRY.len();
        if count == 0 {
            Err(StartupError::Warning(
                "no axioms registered - system will run without invariant checks".into(),
            ))
        } else {
            Ok(())
        }
    }
}

pub struct VersionInfoCheck;
impl StartupCheck for VersionInfoCheck {
    fn name(&self) -> &'static str {
        "version-info"
    }
    fn check(&self) -> Result<(), StartupError> {
        let v = axiom_core::version::VersionInfo::current();
        if v.protocol_version.0 == 0 {
            return Err(StartupError::Blocking(
                "protocol version cannot be 0".into(),
            ));
        }
        if v.signal_schema.0 == 0 {
            return Err(StartupError::Blocking(
                "signal schema version cannot be 0".into(),
            ));
        }
        Ok(())
    }
}

pub fn builtin_startup_verification() -> StartupVerification {
    let mut v = StartupVerification::new();
    v.add_check(Box::new(LayerCanSendToCheck));
    v.add_check(Box::new(AxiomRegistryCheck));
    v.add_check(Box::new(VersionInfoCheck));
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    struct AlwaysOk;
    impl StartupCheck for AlwaysOk {
        fn name(&self) -> &'static str {
            "ok"
        }
        fn check(&self) -> Result<(), StartupError> {
            Ok(())
        }
    }

    struct BlockingCheck;
    impl StartupCheck for BlockingCheck {
        fn name(&self) -> &'static str {
            "blocking"
        }
        fn check(&self) -> Result<(), StartupError> {
            Err(StartupError::Blocking("bad".into()))
        }
    }

    struct WarnCheck;
    impl StartupCheck for WarnCheck {
        fn name(&self) -> &'static str {
            "warn"
        }
        fn check(&self) -> Result<(), StartupError> {
            Err(StartupError::Warning("careful".into()))
        }
    }

    #[test]
    fn test_all_ok_passes() {
        let mut v = StartupVerification::new();
        v.add_check(Box::new(AlwaysOk));
        let r = v.run();
        assert!(r.all_passed);
        assert!(r.blocking_failures.is_empty());
        assert_eq!(r.passed.len(), 1);
    }

    #[test]
    fn test_blocking_fails() {
        let mut v = StartupVerification::new();
        v.add_check(Box::new(AlwaysOk));
        v.add_check(Box::new(BlockingCheck));
        let r = v.run();
        assert!(!r.all_passed);
        assert_eq!(r.blocking_failures.len(), 1);
    }

    #[test]
    fn test_warning_does_not_block() {
        let mut v = StartupVerification::new();
        v.add_check(Box::new(WarnCheck));
        let r = v.run();
        assert!(r.all_passed);
        assert_eq!(r.warnings.len(), 1);
    }

    #[test]
    fn test_builtin_checks() {
        let v = builtin_startup_verification();
        let r = v.run();
        assert!(
            r.all_passed,
            "builtin checks should pass: {:?}",
            r.blocking_failures
        );
    }
}
