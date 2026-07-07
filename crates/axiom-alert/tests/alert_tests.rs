#[cfg(test)]
mod tests {
    use axiom_alert::{
        alert::{Alert, AlertStatus, Severity},
        router::AlertRouter,
        silence::Silence,
        store::MemoryAlertStore,
        threshold::{Threshold, ThresholdKind, Window, WindowKind},
        GovernanceMapper,
    };
    use axiom_oversight::entropy_governor::{EntropyGovernorCell, GovernanceAction};

    #[test]
    fn test_threshold_matches() {
        let t = Threshold { kind: ThresholdKind::Gt, value: 10.0 };
        assert!(t.matches(11.0));
        assert!(!t.matches(10.0));
    }

    #[test]
    fn test_window_default() {
        let w = Window::default();
        assert_eq!(w.kind, WindowKind::Count(1));
        assert_eq!(w.min_hits, 1);
    }

    #[test]
    fn test_alert_router() {
        let mut router = AlertRouter::default();
        router.add_route("cpu", vec![Severity::Critical]);
        let alert = Alert::new("cpu", Severity::Critical, "high cpu");
        let routed = router.route(&alert);
        assert_eq!(routed, vec![Severity::Critical]);
    }

    #[test]
    fn test_silence_matches() {
        let silence = Silence::new(vec![("rule".into(), "cpu".into())], 0, "tester", "silence");
        let mut alert = Alert::new("cpu", Severity::Warn, "high cpu");
        alert.labels = vec![("rule".into(), "cpu".into())];
        assert!(silence.matches(&alert));
    }

    #[test]
    fn test_memory_store_roundtrip() {
        let mut store = MemoryAlertStore::new();
        let alert = Alert::new("cpu", Severity::Critical, "high cpu");
        store.insert(alert.clone());
        let results = store.query(None);
        assert_eq!(results.len(), 1);
        store.update_status(&alert.id, AlertStatus::Resolved);
        let resolved = store.query(Some(AlertStatus::Resolved));
        assert_eq!(resolved.len(), 1);
    }

    #[test]
    fn test_governance_mapper_critical() {
        let governor = EntropyGovernorCell::new(10.0, 20.0, 30.0, 40.0);
        let mut alert = Alert::new("test", Severity::Critical, "critical alert");
        alert.status = AlertStatus::Firing;
        let action = GovernanceMapper::map(&alert, &governor);
        assert!(matches!(action, Some(GovernanceAction::Emergency { .. })));
    }
}
