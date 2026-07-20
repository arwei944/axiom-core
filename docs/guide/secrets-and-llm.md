# Secrets & LLM credentials (product floor)

## Rules

1. **Never** commit API keys, tokens, or connection strings.
2. Load secrets from **environment** (or a future SecretsPort / KMS) — not from JSON in git.
3. Prefer mock mode in CI: `AXIOM_LLM_MOCK=1`.

## Environment variables

| Variable | Purpose |
|----------|---------|
| `AXIOM_LLM_API_KEY` | Primary product LLM key |
| `OPENAI_API_KEY` | Optional OpenAI fallback (`axiom_llm::credentials`) |
| `ANTHROPIC_API_KEY` | Optional Anthropic fallback |
| `AXIOM_LLM_MOCK` | `1`/`true` → no live credential required |
| `AXIOM_API_KEY` | HTTP API gateway auth (ops) |
| `AXIOM_AUTH_MODE` | `disabled` / `api_key` / `jwt` / … |
| `RUST_LOG` | tracing filter |

## Code anchors

| Piece | Path |
|-------|------|
| Env resolve helper | `crates/axiom-llm/src/credentials.rs` |
| ISA Port shape (demo) | `crates/axiom-demo-taskflow/src/llm_port.rs` (`EnvLlmProposePort`) |
| Workbench still allow-listed | `workbench.rs` + sandbox tests |

## Pattern

```text
Port call
  → EnvSecrets::require("AXIOM_LLM_API_KEY") or mock
  → never log raw key
  → journal via run_port when used inside Composer
```
