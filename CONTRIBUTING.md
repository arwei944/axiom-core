# Contributing to Axiom Core

## Getting Started

1. Clone the repository:
   ```bash
   git clone https://github.com/axiom-core/axiom-core.git
   cd axiom-core
   ```

2. Install dependencies:
   ```bash
   cargo install cargo-audit
   cargo install cargo-edit
   ```

3. Set up git hooks:
   ```bash
   cp .git/hooks/pre-commit.sample .git/hooks/pre-commit
   cp .git/hooks/pre-push.sample .git/hooks/pre-push
   chmod +x .git/hooks/pre-commit .git/hooks/pre-push
   ```

4. Run initial validation:
   ```bash
   axm check
   ```

## Development Workflow

### Before Writing Code

1. **Checkout a new branch**:
   ```bash
   git checkout -b feature/my-feature
   ```

2. **Run preflight checks**:
   ```bash
   axm preflight
   ```

### While Writing Code

1. **Use macros for all components**:
   ```rust
   #[cell(layer = "exec")]    // ✅ Required: specify layer
   #[signal(layer = "exec")]  // ✅ Required: specify layer
   #[axiom]                    // ✅ Required: auto-registration
   #[guard(layer = "exec")]   // ✅ Required: specify layer
   ```

2. **Follow layer conventions**:
   - `exec`: Execution layer (business logic)
   - `validate`: Validation layer (input validation)
   - `agent`: Agent layer (AI orchestration)
   - `oversight`: Oversight layer (monitoring, governance)

3. **Never bypass compile-time constraints**:
   - ❌ Don't use `std::sync::Mutex/RwLock` - use `parking_lot::Mutex/RwLock`
   - ❌ Don't use `async-trait` - use Rust 1.75+ native async fn in traits
   - ❌ Don't use `unwrap()`/`expect()` in non-test code
   - ❌ Don't depend on crates below your layer

4. **Use proper error handling**:
   ```rust
   // ✅ Correct
   fn do_something() -> KernelResult<()> {
       something()?;
       Ok(())
   }

   // ❌ Wrong
   fn do_something() {
       something().unwrap();  // Never use unwrap()!
   }
   ```

5. **Write tests**:
   - Unit tests: `#[cfg(test)]` in source files
   - Integration tests: In `tests/` directory
   - Property tests: Use `proptest` for edge cases

### Before Committing

1. **Run format check**:
   ```bash
   cargo fmt --check
   ```

2. **Run lint check**:
   ```bash
   cargo clippy --workspace -- -D warnings
   ```

3. **Run build check**:
   ```bash
   cargo check --workspace
   ```

4. **Run architecture validation**:
   ```bash
   axm verify
   ```

### Before Pushing

1. **Run all tests**:
   ```bash
   cargo test --workspace
   ```

2. **Run documentation check**:
   ```bash
   cargo doc --workspace --no-deps
   ```

3. **Run security audit**:
   ```bash
   cargo audit
   ```

4. **Run full validation**:
   ```bash
   axm check
   ```

## Code Style

### Formatting

- Use `cargo fmt` for all formatting
- Line width: 100 characters
- Indentation: 4 spaces (never tabs)
- Unix line endings (`\n`)

### Naming

- **Structs/Enums**: PascalCase
- **Functions/Methods**: snake_case
- **Constants**: UPPER_SNAKE_CASE
- **Traits**: PascalCase (adjective form)
- **Modules**: snake_case
- **Type aliases**: PascalCase

### Imports

- Group imports by crate (std, external, internal)
- Use `use crate::` for internal paths
- Avoid glob imports (`use foo::*`)
- Reorder imports alphabetically

### Documentation

- All public items must have documentation comments
- Use `///` for public items
- Use `//!` for module-level documentation
- Include examples where appropriate
- Follow Rust documentation conventions

## Architecture Rules

### Layer Dependencies

Crate at level N can only depend on crates at level >= N:

| Level | Crates |
|-------|--------|
| 0 | axiom-cli, axiom-bench |
| 1 | axiom-viz |
| 2 | axiom-identity, axiom-prompt |
| 3 | axiom-mcp, axiom-alert, axiom-agent, axiom-oversight |
| 4 | axiom-distributed, axiom-planner, axiom-runtime |
| 5 | axiom-llm, axiom-tool, axiom-memory, axiom-store |
| 7 | axiom-kernel, axiom-core (deprecated) |
| 8 | axiom-macros |
| 9 | axiom-plugin-wasm-sdk, axiom-plugin-test, axiom-plugin-example-wasm |

### Layer Communication

Only legal layer transitions are allowed:

- Oversight → All layers
- Agent → Agent, Validate
- Validate → Validate, Exec, Agent
- Exec → Exec

### Forbidden Dependencies

- `async-trait` - Use Rust 1.75+ native async fn in traits
- Any crate not in `AUDITED_DEPS`

### Required Patterns

- Use `parking_lot::Mutex/RwLock` instead of `std::sync`
- Use `futures::FutureExt::catch_unwind` for Cell panic handling
- Use `LayeredCellContext<L>` with `L: CanSendTo<Target>`
- Use `tokio::time::sleep` for exponential backoff in Cell restart
- All third-party dependencies must be in `AUDITED_DEPS`

## Testing Guidelines

### Unit Tests

- Test individual functions/methods
- Use `#[test]` attribute
- Aim for 80%+ coverage
- Test edge cases and error paths

### Integration Tests

- Test interactions between components
- Use `tests/` directory
- Test full workflows
- Include Witness chain verification

### Property Tests

- Use `proptest` for randomized testing
- Test invariants and contracts
- Focus on data structures and algorithms

### Benchmark Tests

- Use `criterion` for benchmarking
- Add benchmarks for performance-critical paths
- Include in `axiom-bench` crate

## Git Commit Messages

### Format

```
<type>(<scope>): <description>

<optional body>

<optional footer>
```

### Types

- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation update
- `refactor`: Code refactoring
- `test`: Test update
- `chore`: Build/tooling update
- `perf`: Performance improvement
- `style`: Code style update

### Examples

```
feat(kernel): add WitnessKernel with SHA-256 hash chain

- Implement Witness struct with hash chain validation
- Add WitnessBuilder for fluent witness creation
- Include WitnessKernel for in-memory witness storage

Closes #123
```

```
fix(runtime): fix Cell restart exponential backoff

- Add tokio::time::sleep for actual delay
- Use saturating arithmetic to prevent overflow
- Fix backoff calculation formula
```

## Pull Request Guidelines

1. **Title**: Follow commit message format
2. **Description**:
   - What changes were made
   - Why the changes were needed
   - How to test the changes
3. **Checklist**:
   - [ ] Code follows style guidelines
   - [ ] All tests pass
   - [ ] Documentation is updated
   - [ ] `axm check` passes
4. **Review**:
   - Request review from at least one maintainer
   - Address all review comments
   - Rebase on latest main before merging

## Debugging Tips

1. **Enable logging**:
   ```bash
   RUST_LOG=trace cargo run
   ```

2. **Use Witness chain for debugging**:
   ```bash
   axm witness inspect <correlation_id>
   ```

3. **Check entropy levels**:
   ```bash
   axm entropy status
   ```

4. **Inspect hotspots**:
   ```bash
   axm heatmap show
   ```

## Common Issues

### Compile-time Architecture Errors

- **REVERSE DEPENDENCY**: Check `architecture.toml` for layer assignments
- **forbidden dependency**: Remove `async-trait` or other forbidden crates
- **unaudited dependency**: Add to `AUDITED_DEPS` in `architecture.toml`

### Runtime Errors

- **LayerViolation**: Check layer communication rules
- **CellCrashed**: Check `catch_unwind` handling in dispatch loop
- **CircuitBreak**: Check Supervisor configuration and failure rates

### Performance Issues

- **High entropy**: Identify and fix recurring failures
- **Slow signal processing**: Check heatmap for bottlenecks
- **Lock contention**: Use `parking_lot` and reduce lock scope

## Resources

- [Rust Book](https://doc.rust-lang.org/book/)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Axiom Core Architecture](docs/ARCHITECTURE.md)
- [Migration Guide](docs/MIGRATION.md)
- [Plugin System](docs/PLUGIN_SYSTEM.md)
- [Heatmap System](docs/HEATMAP_SYSTEM.md)

## Questions

For questions and discussions, join our [Discord server](https://discord.gg/axiom-core).