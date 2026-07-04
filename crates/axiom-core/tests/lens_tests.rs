//! Integration tests for Lens primitive - end-to-end verification.

use axiom_core::id::LensId;
use axiom_core::lens::{
    InMemoryProjectionCache, IncrementalProjectionCache, Lens, LensAccessor, LensEvent,
    LensRegistry, Projection, ProjectionCache,
};
use axiom_core::signal::VectorClock;
use serde::{Deserialize, Serialize};

// ============================================================
// Test types
// ============================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct CustomerId(pub String);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct OrderSummary {
    order_id: String,
    total: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct OrderItem {
    product_id: String,
    quantity: u32,
    price: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct CustomerSummary {
    customer_id: String,
    total_orders: usize,
    total_spent: f64,
    items: Vec<OrderItem>,
}

// ============================================================
// Test lenses
// ============================================================

struct OrderSummaryLens {
    id: LensId,
}

impl OrderSummaryLens {
    fn new() -> Self {
        Self {
            id: LensId::from("customer-"),
        }
    }
}

impl Lens for OrderSummaryLens {
    type Input = CustomerId;
    type Output = Vec<OrderSummary>;

    fn id(&self) -> &LensId {
        &self.id
    }

    fn project(&self, events: &[LensEvent], input: &CustomerId) -> Vec<OrderSummary> {
        events
            .iter()
            .filter(|e| e.aggregate_id == input.0)
            .filter(|e| e.event_type == "OrderPlaced")
            .filter_map(|e| serde_json::from_value(e.payload.clone()).ok())
            .collect()
    }

    fn cache_key(&self, input: &CustomerId) -> Option<String> {
        Some(input.0.clone())
    }
}

struct CustomerSummaryLens {
    id: LensId,
}

impl CustomerSummaryLens {
    fn new() -> Self {
        Self {
            id: LensId::from("customer-summary"),
        }
    }
}

impl Lens for CustomerSummaryLens {
    type Input = CustomerId;
    type Output = CustomerSummary;

    fn id(&self) -> &LensId {
        &self.id
    }

    fn project(&self, events: &[LensEvent], input: &CustomerId) -> CustomerSummary {
        let order_events: Vec<_> = events
            .iter()
            .filter(|e| e.aggregate_id == input.0)
            .filter(|e| e.event_type == "OrderPlaced")
            .collect();

        let total_orders = order_events.len();
        let mut total_spent = 0.0;
        let mut items = Vec::new();

        for event in &order_events {
            if let Ok(summary) = serde_json::from_value::<OrderSummary>(event.payload.clone()) {
                total_spent += summary.total;
            }
            if let Ok(order_items) = serde_json::from_value::<Vec<OrderItem>>(event.payload.clone())
            {
                items.extend(order_items);
            }
        }

        CustomerSummary {
            customer_id: input.0.clone(),
            total_orders,
            total_spent,
            items,
        }
    }

    fn depends_on(&self) -> &[LensId] {
        use std::sync::LazyLock;
        static DEPENDS: LazyLock<Vec<LensId>> = LazyLock::new(|| vec![LensId::from("customer-")]);
        DEPENDS.as_slice()
    }

    fn cache_key(&self, input: &CustomerId) -> Option<String> {
        Some(input.0.clone())
    }
}

// ============================================================
// Helper functions
// ============================================================

fn create_test_events(customer_id: &str, count: usize) -> Vec<LensEvent> {
    (0..count)
        .map(|i| LensEvent {
            aggregate_id: customer_id.to_string(),
            event_type: "OrderPlaced".to_string(),
            payload: serde_json::json!({
                "order_id": format!("order-{}", i),
                "total": (i + 1) as f64 * 100.0,
                "items": [
                    {"product_id": "prod-1", "quantity": 2, "price": 50.0},
                    {"product_id": "prod-2", "quantity": 1, "price": 100.0}
                ]
            }),
            vector_clock: VectorClock::new(),
            timestamp_ns: i as u64 * 1_000_000,
            sequence_number: i as u64,
        })
        .collect()
}

// ============================================================
// Integration tests
// ============================================================

#[test]
fn full_projection_flow_works() {
    // Setup
    let order_lens = OrderSummaryLens::new();
    let cache = InMemoryProjectionCache::new();
    let events = create_test_events("customer-1", 3);
    let input = CustomerId("customer-1".to_string());

    // Execute projection through cache
    let projection = cache.get_or_compute(&order_lens, &events, &input);

    // Verify projection
    assert_eq!(projection.lens_id.as_str(), "customer-");
    assert_eq!(projection.event_count, 3);
    assert!(!projection.was_cached);

    let result: Vec<OrderSummary> = projection.downcast().unwrap();
    assert_eq!(result.len(), 3);
    assert_eq!(result[0].order_id, "order-0");
    assert_eq!(result[0].total, 100.0);
}

#[test]
fn incremental_projection_flow_works() {
    // Setup - need a 'static lens reference for the incremental cache
    let order_lens = Box::new(OrderSummaryLens::new());
    let order_lens_static: &'static dyn axiom_core::lens::Projectable = Box::leak(order_lens);
    let cache = IncrementalProjectionCache::new();
    let events = create_test_events("customer-1", 3);
    let input_value = serde_json::json!("customer-1");

    // Initial projection
    let projection1 = cache.get_or_compute_projectable(order_lens_static, &events, &input_value);
    assert_eq!(projection1.event_count, 3);
    assert!(!projection1.was_cached);

    // Add new event
    let new_event = LensEvent {
        aggregate_id: "customer-1".to_string(),
        event_type: "OrderPlaced".to_string(),
        payload: serde_json::json!({
            "order_id": "order-3",
            "total": 400.0,
            "items": []
        }),
        vector_clock: VectorClock::new(),
        timestamp_ns: 3_000_000,
        sequence_number: 3,
    };

    cache.on_new_event_for_lenses(&new_event, &[order_lens_static]);

    // Project with same events (cache should use base + delta)
    let projection2 = cache.get_or_compute_projectable(order_lens_static, &events, &input_value);
    assert_eq!(projection2.event_count, 4);
    assert!(projection2.was_cached);

    let result: Vec<OrderSummary> = projection2.downcast().unwrap();
    assert_eq!(result.len(), 4);
    assert_eq!(result[3].order_id, "order-3");
    assert_eq!(result[3].total, 400.0);
}

#[test]
fn lens_accessor_provides_type_safe_api() {
    // LensAccessor requires a 'static reference, so we leak the lens on purpose for testing
    let order_lens = Box::new(OrderSummaryLens::new());
    let accessor = LensAccessor::new(Box::leak(order_lens));
    let events = create_test_events("customer-1", 2);
    let input = CustomerId("customer-1".to_string());

    let projection = accessor
        .project::<CustomerId, Vec<OrderSummary>>(&events, &input)
        .unwrap();

    assert_eq!(projection.lens_id.as_str(), "customer-");
    let result: Vec<OrderSummary> = projection.downcast().unwrap();
    assert_eq!(result.len(), 2);
}

#[test]
fn lens_registry_returns_registered_lenses() {
    // LensRegistry should be populated by #[lens] macros
    // For now, verify the registry API works
    let lenses = LensRegistry::registered_lenses();

    // In a real scenario with #[lens] macros, this would contain registered lenses
    // For unit test, we just verify the API is callable
    let _ = lenses;
}

#[test]
fn cache_invalidation_works() {
    let order_lens = OrderSummaryLens::new();
    let cache = InMemoryProjectionCache::new();
    let events = create_test_events("customer-1", 2);
    let input = CustomerId("customer-1".to_string());

    // Initial projection
    cache.get_or_compute(&order_lens, &events, &input);
    let metrics = cache.metrics();
    assert_eq!(metrics.entries, 1);

    // Invalidate by lens id
    cache.invalidate(&LensId::from("customer-"));
    let metrics = cache.metrics();
    assert_eq!(metrics.entries, 0);
}

#[test]
fn cache_metrics_track_hits_and_misses() {
    let order_lens = OrderSummaryLens::new();
    let cache = InMemoryProjectionCache::new();
    let events = create_test_events("customer-1", 2);
    let input = CustomerId("customer-1".to_string());

    // First call - miss
    cache.get_or_compute(&order_lens, &events, &input);
    // Second call - hit
    cache.get_or_compute(&order_lens, &events, &input);
    // Third call - hit
    cache.get_or_compute(&order_lens, &events, &input);

    let metrics = cache.metrics();
    assert_eq!(metrics.hits, 2);
    assert_eq!(metrics.misses, 1);
    assert!((metrics.hit_rate() - 0.666).abs() < 0.01);
}

#[test]
fn projection_downcast_into_typed_output() {
    let order_lens = OrderSummaryLens::new();
    let cache = InMemoryProjectionCache::new();
    let events = create_test_events("customer-1", 2);
    let input = CustomerId("customer-1".to_string());

    let projection = cache.get_or_compute(&order_lens, &events, &input);
    let summaries: Vec<OrderSummary> = projection.downcast().unwrap();

    assert_eq!(summaries.len(), 2);
    assert_eq!(summaries[0].order_id, "order-0");
    assert_eq!(summaries[1].total, 200.0);
}

#[test]
fn lens_depends_on_reports_dependencies() {
    let order_lens = OrderSummaryLens::new();
    let customer_lens = CustomerSummaryLens::new();

    assert!(order_lens.depends_on().is_empty());
    assert_eq!(customer_lens.depends_on().len(), 1);
    assert_eq!(customer_lens.depends_on()[0].as_str(), "customer-");
}

#[test]
fn incremental_cache_on_new_event_routes_to_correct_lens() {
    let order_lens = Box::new(OrderSummaryLens::new());
    let order_lens_static: &'static dyn axiom_core::lens::Projectable = Box::leak(order_lens);
    let cache = IncrementalProjectionCache::new();
    let events = create_test_events("customer-1", 2);
    let input_value = serde_json::json!("customer-1");

    // Initial projection to establish base
    cache.get_or_compute_projectable(order_lens_static, &events, &input_value);

    // Simulate a new event
    let new_event = LensEvent {
        aggregate_id: "customer-1".to_string(),
        event_type: "OrderPlaced".to_string(),
        payload: serde_json::json!({"order_id": "order-new", "total": 999.0}),
        vector_clock: VectorClock::new(),
        timestamp_ns: 3_000_000,
        sequence_number: 3,
    };

    cache.on_new_event_for_lenses(&new_event, &[order_lens_static]);

    // Project again - should use base + delta
    let projection = cache.get_or_compute_projectable(order_lens_static, &events, &input_value);
    assert!(projection.was_cached);

    let result: Vec<OrderSummary> = projection.downcast().unwrap();
    assert_eq!(result.len(), 3);
    assert_eq!(result[2].order_id, "order-new");
}

#[test]
fn projection_budget_check_works() {
    let projection = Projection {
        lens_id: LensId::from("test"),
        input_hash: [0u8; 32],
        output: serde_json::json!("test"),
        vector_clock: VectorClock::new(),
        token_count: Some(10),
        summary: None,
        projection_time_ms: 0,
        event_count: 0,
        was_cached: false,
        last_sequence_number: None,
    };

    assert!(projection.is_within_budget(100));
    assert!(!projection.is_within_budget(5));
}
