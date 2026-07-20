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

1. **Commercial business Cells** (Task, Agent, future verticals) must call:
   - `axiom_isa::product_admit` before Composer work, and/or
   - `axiom_isa::product_decide` for read-only decision snapshots on the surface.
2. **Do not** invent a second public admit API (e.g. `guardian_allow`, `oversight_admit_product`).
3. Runtime entropy cells may **feed metrics / degraded / internal throttle**; they must **not** replace `product_admit` on the commercial write path.
4. Surface JSON always reports `"admit_authority": "governor"` meaning **ISA product Governor**.

## Code anchors

- `crates/axiom-isa/src/lib.rs` — `product_admit` / `product_decide`
- `crates/axiom-demo-taskflow/src/task_cell.rs` / `agent_cell.rs` — sole admit entry
- `crates/axiom-demo-taskflow/src/surface.rs` / `product_gateway.rs` — surface fields

## Agent instruction

If you need to refuse work under high entropy on a product path, raise entropy on **`axiom_isa::Governor`** (or call `trip()` / config) and go through **`product_admit`**. Do not bypass with a parallel runtime-only check that lets Port side effects run.
