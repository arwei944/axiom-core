use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, ItemStruct};

pub fn impl_guard(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let name = &input.ident;
    let name_str = name.to_string();

    let mut layer = None;
    let attr2: TokenStream2 = attr.into();
    let mut iter = attr2.into_iter();
    while let Some(tt) = iter.next() {
        if let proc_macro2::TokenTree::Ident(ident) = tt {
            if ident == "layer" {
                let _eq = iter.next();
                if let Some(proc_macro2::TokenTree::Literal(lit)) = iter.next() {
                    let s = lit.to_string();
                    let s = s.trim_matches('"').to_string();
                    layer = Some(s);
                }
            }
        }
    }

    let layer_str = layer.clone().unwrap_or_else(|| "all".to_string());

    let expanded = quote! {
        #[derive(Debug, Default)]
        #input

        impl ::axiom_kernel::guard::Guard for #name {
            fn id(&self) -> &'static str {
                #name_str
            }
            fn layer(&self) -> Option<::axiom_kernel::Layer> {
                match #layer_str {
                    "exec" => Some(::axiom_kernel::Layer::Exec),
                    "validate" => Some(::axiom_kernel::Layer::Validate),
                    "agent" => Some(::axiom_kernel::Layer::Agent),
                    "oversight" => Some(::axiom_kernel::Layer::Oversight),
                    _ => None,
                }
            }
            fn check(&self, signal: &dyn ::axiom_kernel::signal::Signal) -> ::axiom_kernel::KernelResult<()> {
                let result = self.check_inner(signal);

                let _ = ::axiom_kernel::registry::WITNESS_REGISTRY.record(::axiom_kernel::witness::Witness {
                    witness_id: ::axiom_kernel::id::WitnessId::new(format!("guard-wit-{}", ::axiom_kernel::clock::global_clock().now_ns())),
                    schema_version: <::axiom_kernel::version::WitnessSchema as ::axiom_kernel::version::Versioned>::schema_version(),
                    cell_id: "guard-executor".to_string(),
                    correlation_id: ::axiom_kernel::id::CorrelationId::new("auto"),
                    trace_id: None,
                    triggering_msg_id: None,
                    vector_clock: ::axiom_kernel::signal::VectorClock::new(),
                    timestamp_ns: ::axiom_kernel::clock::global_clock().now_ns(),
                    prev_hash: None,
                    state_before_hash: None,
                    state_after_hash: None,
                    hash: ::axiom_kernel::witness::WitnessHash::zero(),
                    summary: format!("guard {} checked signal {}", #name_str, signal.signal_type()),
                    outcome: match &result {
                        Ok(()) => ::axiom_kernel::witness::TransitionOutcome::Success,
                        Err(e) => ::axiom_kernel::witness::TransitionOutcome::Failed {
                            reason: e.to_string()
                        }
                    },
                    metrics: ::axiom_kernel::witness::WitnessMetrics::default(),
                    version_info: ::axiom_kernel::version::VersionInfo::current(),
                    signal_fingerprint: [0u8; 32],
                    payload_size_bytes: 0,
                    kind: ::axiom_kernel::witness::WitnessKind::GuardCheck,
                });

                result
            }
        }

        impl #name {
            pub fn check_inner(&self, signal: &dyn ::axiom_kernel::signal::Signal) -> ::axiom_kernel::KernelResult<()> {
                Ok(())
            }
        }
    };

    TokenStream::from(expanded)
}
