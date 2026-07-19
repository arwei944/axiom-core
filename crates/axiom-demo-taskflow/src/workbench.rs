//! Controlled Workbench: propose → execute (allow-list sandbox Port).
//!
//! T11 product floor — not unrestricted LLM code execution:
//! - Intent allow-list ([`is_allowed_intent`])
//! - [`WorkbenchLimits`] resource envelope
//! - Deterministic propose Atom **or** mock LLM propose Port
//! - Per-step Witness via ISA journal
//! - Optional plugin sandbox path (`plugin_echo`)

use axiom_isa::{
    is_allowed_intent, run_atom, run_port, AtomFn, Composer, HandoffRequest, HandoffResult,
    IsaError, IsaResult, Port, SeqComposer, StepKind, WitnessJournal, WorkbenchLimits,
    WorkbenchProposal,
};
use uuid::Uuid;

/// Mock LLM propose Port — deterministic, no network (product floor).
///
/// Real LLM providers remain behind Port + Witness; this stands in so the
/// commercial path exercises a **Port-shaped** propose without SaaS keys.
struct MockLlmProposePort {
    limits: WorkbenchLimits,
}

impl Port<HandoffRequest, WorkbenchProposal> for MockLlmProposePort {
    fn name(&self) -> &str {
        "llm_propose_mock"
    }

    fn call(&mut self, input: HandoffRequest) -> IsaResult<WorkbenchProposal> {
        if input.payload.len() > self.limits.max_payload_bytes {
            return Err(IsaError::port(
                "llm_propose_mock",
                format!(
                    "payload {} bytes exceeds limit {}",
                    input.payload.len(),
                    self.limits.max_payload_bytes
                ),
            ));
        }
        let plan_id = Uuid::new_v4().to_string();
        let mut steps = vec![
            format!("llm_mock:accept handoff {}", input.token),
            format!("llm_mock:route {} -> {}", input.source_agent, input.target_agent),
            format!("llm_mock:intent={}", input.intent),
        ];
        // Intent-specific plan expansion (still pure relative to external world).
        match input.intent.as_str() {
            "summarize" => {
                let preview: String = input.payload.chars().take(48).collect();
                steps.push(format!("llm_mock:summary_preview={preview}"));
                steps.push("llm_mock:emit_summary".into());
            }
            "task_plan" => {
                steps.push("llm_mock:decompose".into());
                steps.push("llm_mock:order_steps".into());
                steps.push(format!("llm_mock:body={}", input.payload));
            }
            "validate_manifest" => {
                steps.push("llm_mock:parse_manifest".into());
                steps.push("llm_mock:schema_check".into());
            }
            "plugin_echo" => {
                steps.push("llm_mock:dispatch_plugin_sandbox".into());
                steps.push(format!("llm_mock:plugin_payload={}", input.payload));
            }
            _ => {
                steps.push(format!("llm_mock:body={}", input.payload));
            }
        }
        if steps.len() as u32 > self.limits.max_steps {
            steps.truncate(self.limits.max_steps as usize);
        }
        Ok(WorkbenchProposal {
            plan_id,
            steps,
            allowed_action: input.intent.clone(),
        })
    }
}

/// Sandbox Port: only allow-listed intents execute, with resource envelope.
struct SandboxPort {
    limits: WorkbenchLimits,
}

impl Port<WorkbenchProposal, String> for SandboxPort {
    fn name(&self) -> &str {
        "sandbox_execute"
    }

    fn call(&mut self, input: WorkbenchProposal) -> IsaResult<String> {
        if !is_allowed_intent(&input.allowed_action) {
            return Err(IsaError::port(
                "sandbox_execute",
                format!("action `{}` not on allow-list", input.allowed_action),
            ));
        }
        if input.steps.len() as u32 > self.limits.max_steps {
            return Err(IsaError::port(
                "sandbox_execute",
                format!(
                    "steps {} exceed max_steps {}",
                    input.steps.len(),
                    self.limits.max_steps
                ),
            ));
        }

        // Per-step controlled execution (in-process product floor).
        let mut step_results = Vec::with_capacity(input.steps.len());
        for (i, step) in input.steps.iter().enumerate() {
            let out = execute_step(&input.allowed_action, i, step, &self.limits)?;
            step_results.push(out);
        }

        Ok(format!(
            "sandbox ok plan={} action={} mem_mb={} steps={} results=[{}]",
            input.plan_id,
            input.allowed_action,
            self.limits.memory_limit_mb,
            input.steps.len(),
            step_results.join(" | ")
        ))
    }
}

fn execute_step(
    action: &str,
    index: usize,
    step: &str,
    limits: &WorkbenchLimits,
) -> IsaResult<String> {
    // Plugin path: invoke named sandbox semantic (no unrestricted code).
    if action == "plugin_echo" {
        let plugin = limits
            .plugin_id
            .as_deref()
            .unwrap_or("builtin.echo");
        return Ok(format!(
            "step{index}:plugin[{plugin}] mem_mb={} echo=`{}`",
            limits.memory_limit_mb,
            step.chars().take(64).collect::<String>()
        ));
    }

    match action {
        "echo" => Ok(format!("step{index}:echo=`{}`", step.chars().take(64).collect::<String>())),
        "summarize" => {
            let s: String = step.chars().take(32).collect();
            Ok(format!("step{index}:summary={s}…"))
        }
        "task_plan" => Ok(format!("step{index}:plan_node=`{step}`")),
        "validate_manifest" => {
            if step.contains("FAIL") {
                return Err(IsaError::port(
                    "sandbox_execute",
                    format!("manifest step {index} failed validation"),
                ));
            }
            Ok(format!("step{index}:manifest_ok"))
        }
        other => Err(IsaError::port(
            "sandbox_execute",
            format!("no handler for action `{other}`"),
        )),
    }
}

/// Build a Composer that runs the Workbench closed loop for one Handoff.
pub fn workbench_composer() -> SeqComposer<HandoffRequest, HandoffResult> {
    workbench_composer_with_limits(WorkbenchLimits::commercial_default())
}

/// Workbench composer with explicit sandbox envelope (T11).
pub fn workbench_composer_with_limits(
    limits: WorkbenchLimits,
) -> SeqComposer<HandoffRequest, HandoffResult> {
    let mut llm = MockLlmProposePort {
        limits: limits.clone(),
    };
    let mut sandbox = SandboxPort {
        limits: limits.clone(),
    };

    SeqComposer::new("workbench", move |req: HandoffRequest, journal| {
        // Atom: shape + permission check
        let validate = AtomFn::new("wb_validate", |r: HandoffRequest| {
            r.validate_shape()
                .map_err(|e| IsaError::atom("wb_validate", e))?;
            if !r.permissions.iter().any(|p| p == "workbench.execute") {
                return Err(IsaError::atom(
                    "wb_validate",
                    "missing permission workbench.execute",
                ));
            }
            if !is_allowed_intent(&r.intent) {
                return Err(IsaError::atom(
                    "wb_validate",
                    format!("intent `{}` not allowed", r.intent),
                ));
            }
            if r.payload.len() > 64 * 1024 {
                return Err(IsaError::atom("wb_validate", "payload too large"));
            }
            Ok(r)
        });
        let req = run_atom(&validate, req, journal)?;

        // Port: mock LLM propose (Port-shaped, Witness-recorded)
        let proposal = run_port(&mut llm, req.clone(), journal)?;

        // Atom: gate proposal against allow-list + limits (pure)
        let limits_check = limits.clone();
        let gate = AtomFn::new("wb_gate_proposal", move |p: WorkbenchProposal| {
            if !is_allowed_intent(&p.allowed_action) {
                return Err(IsaError::atom(
                    "wb_gate_proposal",
                    format!("proposal action `{}` not allowed", p.allowed_action),
                ));
            }
            if p.steps.is_empty() {
                return Err(IsaError::atom("wb_gate_proposal", "empty proposal steps"));
            }
            if p.steps.len() as u32 > limits_check.max_steps {
                return Err(IsaError::atom(
                    "wb_gate_proposal",
                    format!("too many steps: {}", p.steps.len()),
                ));
            }
            Ok(p)
        });
        let proposal = run_atom(&gate, proposal, journal)?;

        // Record each step intent in Witness before sandbox Port (audit trail).
        for (i, step) in proposal.steps.iter().enumerate() {
            journal.record_ok(
                StepKind::Atom,
                "wb_step_plan",
                &format!("{i}:{}", step.chars().take(48).collect::<String>()),
            )?;
        }

        // Port: controlled sandbox execute
        let summary = run_port(&mut sandbox, proposal.clone(), journal)?;

        Ok(HandoffResult {
            success: true,
            token: req.token,
            proposal: proposal.plan_id,
            execution_summary: summary,
            message: format!(
                "workbench closed loop ok (limits: steps≤{} mem≤{}MB timeout≤{}ms)",
                limits.max_steps, limits.memory_limit_mb, limits.timeout_ms
            ),
        })
    })
}

/// Run workbench without owning a long-lived composer (tests).
pub fn run_workbench(
    req: HandoffRequest,
    journal: &mut WitnessJournal<'_>,
) -> IsaResult<HandoffResult> {
    let mut c = workbench_composer();
    c.compose(req, journal)
}

/// Run with custom limits.
pub fn run_workbench_with_limits(
    req: HandoffRequest,
    limits: WorkbenchLimits,
    journal: &mut WitnessJournal<'_>,
) -> IsaResult<HandoffResult> {
    let mut c = workbench_composer_with_limits(limits);
    c.compose(req, journal)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allow_list_includes_plugin_echo() {
        assert!(is_allowed_intent("plugin_echo"));
        assert!(!is_allowed_intent("rm_rf"));
    }

    #[test]
    fn execute_step_echo() {
        let lim = WorkbenchLimits::default();
        let r = execute_step("echo", 0, "hello", &lim).unwrap();
        assert!(r.contains("echo"));
    }

    #[test]
    fn execute_step_plugin() {
        let mut lim = WorkbenchLimits::default();
        lim.plugin_id = Some("builtin.echo".into());
        let r = execute_step("plugin_echo", 0, "payload-x", &lim).unwrap();
        assert!(r.contains("plugin[builtin.echo]"), "{r}");
    }
}
