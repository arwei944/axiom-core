# OpenTelemetry Integration

This guide shows how to export Axiom traces to OpenTelemetry-compatible backends such as Jaeger, Zipkin, or Datadog.

## Prerequisites

- Axiom runtime compiled with the `tracing` feature.
- An OpenTelemetry collector or compatible endpoint.

## Configuration

Set the following environment variables:

```bash
export AXIOM_OTLP_ENDPOINT="http://localhost:4317"
export AXIOM_SERVICE_NAME="axiom-service"
```

## Usage

Enable the `OTLPExporter` in your runtime configuration:

```rust
use axiom_kernel::tracing::{OTLPExporter, TraceExporter};

let exporter = OTLPExporter::new("http://localhost:4317", "axiom-service");
// exporter is automatically picked up by the tracing subsystem
```

## Verification

Send a signal through a cell and verify the span appears in your OTLP backend:

```bash
axm trace view <trace_id>
```

## Troubleshooting

- Ensure the OTLP endpoint uses gRPC and the correct protocol.
- Check that spans contain the `trace_id` and `span_id` attributes.
