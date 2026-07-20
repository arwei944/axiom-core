# Dual Governor narrative (product admit is still unique)

| Field | Value |
|-------|--------|
| **Status** | Normative clarification |
| **Date** | 2026-07-19 |

## Problem

The monorepo contains more than one type whose name suggests “entropy / governance”:

| Symbol | Crate | Role |
|--------|--------|------|
| **`axiom_isa::Governor`** | `axiom-isa` | **Product admit / decide authority** for commercial Cells (`product_admit` / `product_decide`) |
| **`EntropyGovernorCell` / runtime entropy** | `axiom-runtime` | Runtime-internal entropy bookkeeping, throttling hooks, health fields |

These are **not** two equal product decision engines.

## Rule (MUST)

1. **Commercial business Cells** (Task, Agent, future verticals) must call **`axiom_isa::product_admit` by name** before Composer work (not bare `Governor::admit`).
2. Read-only snapshots use **`product_decide`** only.
3. **Do not** invent a second public admit API (e.g. `guardian_allow`, `oversight_admit_product`).
4. Runtime `EntropyGovernorCell` may **feed metrics / degraded / internal throttle only**; it is **non-authoritative** for commercial Port side effects.
5. Surface JSON always reports `"admit_authority": "governor"` meaning **ISA product Governor**.
6. Path tests: `crates/axiom-demo-taskflow/tests/commercial_admit_path.rs`.

## Code anchors

- `crates/axiom-isa/src/lib.rs` — `product_admit` / `product_decide`
- `crates/axiom-demo-taskflow/src/task_cell.rs` / `agent_cell.rs` — **only** `product_admit`
- `crates/axiom-demo-taskflow/src/surface.rs` / `product_gateway.rs` — surface fields

## Agent instruction

If you need to refuse work under high entropy on a product path, raise entropy on **`axiom_isa::Governor`** (or call `trip()` / config) and go through **`product_admit`**. Do not bypass with a parallel runtime-only check that lets Port side effects run.
