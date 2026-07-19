//! Task pipeline expressed only with four primitives.

use crate::store::{InMemoryTaskStore, StoredTask};
use axiom_isa::{
    run_adapter, run_atom, run_port, AdapterFn, AtomFn, Composer, IsaError, IsaResult, Port,
    SeqComposer, WitnessJournal,
};
use axiom_resilience::{CircuitBreaker, CircuitConfig, CircuitState, Retry, RetryConfig};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailMode {
    None,
    /// Execute Port always fails (drives retry + circuit).
    ExecuteAlways,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawTask {
    pub title: String,
    pub priority: u8,
    pub payload: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub priority: u8,
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedTask {
    pub task: Task,
    pub plan: String,
}

#[derive(Debug, Clone)]
#[allow(dead_code)] // fields read via Debug in CLI output
pub struct TaskResult {
    pub id: String,
    pub plan: String,
    pub stored: bool,
}

/// Execute Port with optional failure injection.
struct ExecutePort {
    mode: FailMode,
    hits: u32,
}

impl Port<PlannedTask, PlannedTask> for ExecutePort {
    fn name(&self) -> &str {
        "execute"
    }

    fn call(&mut self, input: PlannedTask) -> IsaResult<PlannedTask> {
        self.hits = self.hits.saturating_add(1);
        match self.mode {
            FailMode::None => Ok(input),
            FailMode::ExecuteAlways => Err(IsaError::port("execute", "injected failure")),
        }
    }
}

/// Persist Port → InMemoryTaskStore.
struct PersistPort {
    store: InMemoryTaskStore,
}

impl Port<PlannedTask, TaskResult> for PersistPort {
    fn name(&self) -> &str {
        "persist"
    }

    fn call(&mut self, input: PlannedTask) -> IsaResult<TaskResult> {
        let id = input.task.id.clone();
        let plan = input.plan.clone();
        self.store
            .save(StoredTask {
                id: id.clone(),
                title: input.task.title,
                priority: input.task.priority,
                plan: plan.clone(),
                body: input.task.body,
            })
            .map_err(|e| IsaError::port("persist", e))?;
        Ok(TaskResult {
            id,
            plan,
            stored: true,
        })
    }
}

/// Full task pipeline composer (held so circuit state survives across submits).
pub struct TaskPipeline {
    composer: SeqComposer<Value, TaskResult>,
    /// Shared view of circuit for demo printing.
    circuit_state: std::sync::Arc<std::sync::Mutex<CircuitState>>,
}

impl TaskPipeline {
    pub fn new(store: InMemoryTaskStore, fail: FailMode) -> Self {
        let circuit_state = std::sync::Arc::new(std::sync::Mutex::new(CircuitState::Closed));
        let circuit_state_for_port = circuit_state.clone();

        // Build resilient execute: Retry(Circuit(Execute))
        // Actually: Circuit(Retry(Execute)) — failures count after retries.
        let execute = ExecutePort { mode: fail, hits: 0 };
        let retried = Retry::new(
            execute,
            RetryConfig {
                max_attempts: 3,
            },
        );
        let mut protected = CircuitBreaker::new(
            retried,
            CircuitConfig {
                failure_threshold: 2,
                reset_after_ms: 60_000,
            },
        );

        let mut persist = PersistPort { store };

        let composer = SeqComposer::new("task-pipeline", move |raw: Value, journal| {
            // Adapter: external JSON → RawTask
            let parse = AdapterFn::new("parse_json", |v: Value| {
                serde_json::from_value::<RawTask>(v)
                    .map_err(|e| IsaError::adapter("parse_json", e.to_string()))
            });
            let raw_task: RawTask = run_adapter(&parse, raw, journal)?;

            // Atom: validate
            let validate = AtomFn::new("validate", |r: RawTask| {
                if r.title.trim().is_empty() {
                    return Err(IsaError::atom("validate", "title required"));
                }
                if r.priority == 0 || r.priority > 5 {
                    return Err(IsaError::atom("validate", "priority must be 1..=5"));
                }
                Ok(r)
            });
            let raw_task = run_atom(&validate, raw_task, journal)?;

            // Atom: assign id + normalize
            let normalize = AtomFn::new("normalize", |r: RawTask| {
                Ok(Task {
                    id: Uuid::new_v4().to_string(),
                    title: r.title.trim().to_string(),
                    priority: r.priority,
                    body: r.payload,
                })
            });
            let task = run_atom(&normalize, raw_task, journal)?;

            // Atom: plan
            let plan_atom = AtomFn::new("plan", |t: Task| {
                let plan = format!("P{}: execute `{}`", t.priority, t.title);
                Ok(PlannedTask { task: t, plan })
            });
            let planned = run_atom(&plan_atom, task, journal)?;

            // Port: execute (retry + circuit)
            let planned = match run_port(&mut protected, planned, journal) {
                Ok(p) => {
                    if let Ok(mut g) = circuit_state_for_port.lock() {
                        *g = protected.state();
                    }
                    p
                }
                Err(e) => {
                    if let Ok(mut g) = circuit_state_for_port.lock() {
                        *g = protected.state();
                    }
                    return Err(e);
                }
            };

            // Port: persist
            let result = run_port(&mut persist, planned, journal)?;
            Ok(result)
        });

        Self {
            composer,
            circuit_state,
        }
    }

    pub fn circuit_state(&self) -> CircuitState {
        *self
            .circuit_state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
    }
}

impl Composer<Value, TaskResult> for TaskPipeline {
    fn name(&self) -> &str {
        self.composer.name()
    }

    fn compose(
        &mut self,
        input: Value,
        journal: &mut WitnessJournal<'_>,
    ) -> IsaResult<TaskResult> {
        self.composer.compose(input, journal)
    }
}


