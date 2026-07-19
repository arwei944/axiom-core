//! Four business primitives (U1).

use crate::error::{IsaError, IsaResult};
use crate::journal::WitnessJournal;

/// Pure computation: same input → same output; no I/O.
pub trait Atom<In, Out> {
    fn name(&self) -> &str;
    fn run(&self, input: In) -> IsaResult<Out>;
}

/// External boundary: network / DB / clock / side effects.
pub trait Port<In, Out> {
    fn name(&self) -> &str;
    fn call(&mut self, input: In) -> IsaResult<Out>;
}

/// Shape / protocol conversion only.
pub trait Adapter<In, Out> {
    fn name(&self) -> &str;
    fn convert(&self, input: In) -> IsaResult<Out>;
}

/// Orchestration of atoms / ports / adapters (and nested composers).
///
/// Must run inside a Cell; records steps only through [`WitnessJournal`].
pub trait Composer<In, Out> {
    fn name(&self) -> &str;
    fn compose(&mut self, input: In, journal: &mut WitnessJournal<'_>) -> IsaResult<Out>;
}

/// What kind of step produced a journal entry (Witness summary prefix).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepKind {
    Atom,
    Port,
    Adapter,
    Composer,
    Governor,
}

impl StepKind {
    pub fn as_str(self) -> &'static str {
        match self {
            StepKind::Atom => "atom",
            StepKind::Port => "port",
            StepKind::Adapter => "adapter",
            StepKind::Composer => "composer",
            StepKind::Governor => "governor",
        }
    }
}

// --- function adapters -------------------------------------------------------

pub struct AtomFn<F> {
    name: String,
    f: F,
}

impl<F> AtomFn<F> {
    pub fn new(name: impl Into<String>, f: F) -> Self {
        Self {
            name: name.into(),
            f,
        }
    }
}

impl<In, Out, F> Atom<In, Out> for AtomFn<F>
where
    F: Fn(In) -> IsaResult<Out>,
{
    fn name(&self) -> &str {
        &self.name
    }

    fn run(&self, input: In) -> IsaResult<Out> {
        (self.f)(input).map_err(|e| match e {
            IsaError::Atom { .. } => e,
            other => IsaError::atom(&self.name, other.to_string()),
        })
    }
}

pub struct PortFn<F> {
    name: String,
    f: F,
}

impl<F> PortFn<F> {
    pub fn new(name: impl Into<String>, f: F) -> Self {
        Self {
            name: name.into(),
            f,
        }
    }
}

impl<In, Out, F> Port<In, Out> for PortFn<F>
where
    F: FnMut(In) -> IsaResult<Out>,
{
    fn name(&self) -> &str {
        &self.name
    }

    fn call(&mut self, input: In) -> IsaResult<Out> {
        let name = self.name.clone();
        (self.f)(input).map_err(|e| match e {
            IsaError::Port { .. } | IsaError::CircuitOpen { .. } => e,
            other => IsaError::port(name, other.to_string()),
        })
    }
}

pub struct AdapterFn<F> {
    name: String,
    f: F,
}

impl<F> AdapterFn<F> {
    pub fn new(name: impl Into<String>, f: F) -> Self {
        Self {
            name: name.into(),
            f,
        }
    }
}

impl<In, Out, F> Adapter<In, Out> for AdapterFn<F>
where
    F: Fn(In) -> IsaResult<Out>,
{
    fn name(&self) -> &str {
        &self.name
    }

    fn convert(&self, input: In) -> IsaResult<Out> {
        (self.f)(input).map_err(|e| match e {
            IsaError::Adapter { .. } => e,
            other => IsaError::adapter(&self.name, other.to_string()),
        })
    }
}

/// Sequential pipeline: each stage is a boxed closure over the previous output.
///
/// Used by demos; production code can implement [`Composer`] directly.
pub struct SeqComposer<In, Out> {
    name: String,
    run: Box<dyn FnMut(In, &mut WitnessJournal<'_>) -> IsaResult<Out> + Send>,
}

impl<In, Out> SeqComposer<In, Out> {
    pub fn new(
        name: impl Into<String>,
        run: impl FnMut(In, &mut WitnessJournal<'_>) -> IsaResult<Out> + Send + 'static,
    ) -> Self {
        Self {
            name: name.into(),
            run: Box::new(run),
        }
    }
}

impl<In, Out> Composer<In, Out> for SeqComposer<In, Out> {
    fn name(&self) -> &str {
        &self.name
    }

    fn compose(&mut self, input: In, journal: &mut WitnessJournal<'_>) -> IsaResult<Out> {
        let name = self.name.clone();
        journal.record_start(StepKind::Composer, &name, "begin")?;
        match (self.run)(input, journal) {
            Ok(out) => {
                journal.record_ok(StepKind::Composer, &name, "done")?;
                Ok(out)
            }
            Err(e) => {
                let _ = journal.record_err(StepKind::Composer, &name, &e.to_string());
                Err(e)
            }
        }
    }
}

/// Helpers to run a primitive and journal it in one shot.
pub fn run_atom<A, In, Out>(
    atom: &A,
    input: In,
    journal: &mut WitnessJournal<'_>,
) -> IsaResult<Out>
where
    A: Atom<In, Out>,
{
    let name = atom.name().to_string();
    journal.record_start(StepKind::Atom, &name, "run")?;
    match atom.run(input) {
        Ok(out) => {
            journal.record_ok(StepKind::Atom, &name, "ok")?;
            Ok(out)
        }
        Err(e) => {
            let _ = journal.record_err(StepKind::Atom, &name, &e.to_string());
            Err(e)
        }
    }
}

pub fn run_adapter<A, In, Out>(
    adapter: &A,
    input: In,
    journal: &mut WitnessJournal<'_>,
) -> IsaResult<Out>
where
    A: Adapter<In, Out>,
{
    let name = adapter.name().to_string();
    journal.record_start(StepKind::Adapter, &name, "convert")?;
    match adapter.convert(input) {
        Ok(out) => {
            journal.record_ok(StepKind::Adapter, &name, "ok")?;
            Ok(out)
        }
        Err(e) => {
            let _ = journal.record_err(StepKind::Adapter, &name, &e.to_string());
            Err(e)
        }
    }
}

pub fn run_port<P, In, Out>(
    port: &mut P,
    input: In,
    journal: &mut WitnessJournal<'_>,
) -> IsaResult<Out>
where
    P: Port<In, Out>,
{
    let name = port.name().to_string();
    journal.record_start(StepKind::Port, &name, "call")?;
    match port.call(input) {
        Ok(out) => {
            journal.record_ok(StepKind::Port, &name, "ok")?;
            Ok(out)
        }
        Err(e) => {
            let _ = journal.record_err(StepKind::Port, &name, &e.to_string());
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axiom_kernel::context::CellContext;
    use axiom_kernel::id::CellId;
    use axiom_kernel::RuntimeTier;

    #[test]
    fn atom_is_pure_and_named() {
        let a = AtomFn::new("double", |x: i32| Ok(x * 2));
        assert_eq!(a.name(), "double");
        assert_eq!(a.run(21).unwrap(), 42);
    }

    #[test]
    fn seq_composer_journals_steps() {
        let cell_id = CellId::new("test-cell");
        let mut ctx = CellContext::new(&cell_id, RuntimeTier::Exec);
        let mut journal = WitnessJournal::new(&mut ctx);

        let mut c = SeqComposer::new("pipe", |x: i32, j| {
            let doubled = run_atom(&AtomFn::new("double", |v: i32| Ok(v * 2)), x, j)?;
            Ok(doubled + 1)
        });

        let out = c.compose(10, &mut journal).unwrap();
        assert_eq!(out, 21);
        let ws = journal.into_witnesses();
        assert!(ws.len() >= 4, "expected composer+atom journal entries, got {}", ws.len());
        assert!(journal_chain_valid(&ws));
    }

    fn journal_chain_valid(ws: &[axiom_kernel::witness::Witness]) -> bool {
        let mut prev = None;
        for w in ws {
            if w.prev_hash != prev {
                return false;
            }
            prev = Some(w.hash);
        }
        true
    }
}
