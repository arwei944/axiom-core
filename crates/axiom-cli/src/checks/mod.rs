pub mod branch_check;
pub mod cargo_build;
pub mod cargo_clippy;
pub mod cargo_fmt;
pub mod cargo_test;
pub mod constraints_hash;
pub mod deps_audit;
pub mod git_status;
pub mod todo_scan;
pub mod unsafe_audit;
pub mod verify;

pub struct CheckResult {
    pub name: &'static str,
    pub passed: bool,
    pub blocking: bool,
    pub message: String,
}

pub trait Check {
    fn name(&self) -> &'static str;
    fn blocking(&self) -> bool;
    fn run(&self) -> CheckResult;
}

pub fn run_all_checks(checks: &[&dyn Check]) -> (Vec<CheckResult>, bool) {
    let mut results = Vec::new();
    let mut has_blocking_failure = false;
    for check in checks {
        let result = check.run();
        if !result.passed && result.blocking {
            has_blocking_failure = true;
        }
        results.push(result);
    }
    (results, has_blocking_failure)
}

pub fn run_boxed_checks(checks: &[Box<dyn Check>]) -> (Vec<CheckResult>, bool) {
    let refs: Vec<&dyn Check> = checks.iter().map(|b| b.as_ref()).collect();
    run_all_checks(&refs)
}

pub fn print_results(results: &[CheckResult]) {
    for r in results {
        if r.passed {
            println!("  ✓ {}", r.name);
        } else if r.blocking {
            println!("  ✗ BLOCKING: {} - {}", r.name, r.message);
        } else {
            println!("  ⚠ WARNING: {} - {}", r.name, r.message);
        }
    }
}

pub fn all_checks() -> Vec<Box<dyn Check>> {
    vec![
        Box::new(branch_check::BranchCheck),
        Box::new(cargo_fmt::CargoFmtCheck),
        Box::new(cargo_build::CargoBuildCheck),
        Box::new(cargo_clippy::CargoClippyCheck),
        Box::new(cargo_test::CargoTestCheck),
        Box::new(constraints_hash::ConstraintsHashCheck),
        Box::new(todo_scan::TodoScanCheck),
        Box::new(unsafe_audit::UnsafeAuditCheck),
        Box::new(deps_audit::DepsAuditCheck),
        Box::new(verify::VerifyCheck),
        Box::new(git_status::GitStatusCheck),
    ]
}

pub fn verify_checks() -> Vec<Box<dyn Check>> {
    vec![
        Box::new(constraints_hash::ConstraintsHashCheck),
        Box::new(todo_scan::TodoScanCheck),
        Box::new(unsafe_audit::UnsafeAuditCheck),
        Box::new(deps_audit::DepsAuditCheck),
        Box::new(verify::VerifyCheck),
    ]
}

pub fn preflight_checks() -> Vec<Box<dyn Check>> {
    vec![
        Box::new(branch_check::BranchCheck),
        Box::new(constraints_hash::ConstraintsHashCheck),
        Box::new(git_status::GitStatusCheck),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockCheck {
        name: &'static str,
        passed: bool,
        blocking: bool,
    }

    impl Check for MockCheck {
        fn name(&self) -> &'static str {
            self.name
        }
        fn blocking(&self) -> bool {
            self.blocking
        }
        fn run(&self) -> CheckResult {
            CheckResult {
                name: self.name,
                passed: self.passed,
                blocking: self.blocking,
                message: if self.passed {
                    "ok".into()
                } else {
                    "failed".into()
                },
            }
        }
    }

    #[test]
    fn test_all_pass() {
        let checks: Vec<&dyn Check> = vec![
            &MockCheck {
                name: "a",
                passed: true,
                blocking: true,
            },
            &MockCheck {
                name: "b",
                passed: true,
                blocking: true,
            },
        ];
        let (results, blocking) = run_all_checks(&checks);
        assert_eq!(results.len(), 2);
        assert!(!blocking);
        assert!(results.iter().all(|r| r.passed));
    }

    #[test]
    fn test_blocking_failure() {
        let checks: Vec<&dyn Check> = vec![
            &MockCheck {
                name: "a",
                passed: true,
                blocking: true,
            },
            &MockCheck {
                name: "b",
                passed: false,
                blocking: true,
            },
        ];
        let (_, blocking) = run_all_checks(&checks);
        assert!(blocking);
    }

    #[test]
    fn test_non_blocking_failure() {
        let checks: Vec<&dyn Check> = vec![
            &MockCheck {
                name: "a",
                passed: true,
                blocking: true,
            },
            &MockCheck {
                name: "b",
                passed: false,
                blocking: false,
            },
        ];
        let (_, blocking) = run_all_checks(&checks);
        assert!(!blocking);
    }

    #[test]
    fn test_all_checks_instantiates() {
        let checks = all_checks();
        assert_eq!(checks.len(), 11);
    }

    #[test]
    fn test_preflight_checks_instantiates() {
        let checks = preflight_checks();
        assert_eq!(checks.len(), 3);
    }

    #[test]
    fn test_verify_checks_instantiates() {
        let checks = verify_checks();
        assert_eq!(checks.len(), 5);
    }
}
