// schema_version(0) should fail because version must be >= 1
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[axiom_macros::schema_version(0)]
struct ZeroVersion {
    msg_id: axiom_kernel::id::MsgId,
    correlation_id: axiom_kernel::id::CorrelationId,
    vector_clock: axiom_kernel::signal::VectorClock,
}

fn main() {}
