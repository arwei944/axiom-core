use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemStruct};

use crate::utils::*;

pub fn impl_cell(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let layer_lit = parse_macro_input!(attr as syn::LitStr);
    let name = &input.ident;
    let name_str = name.to_string();

    if let Err(e) = parse_layer_variant(&layer_lit) {
        return e.to_compile_error().into();
    }

    let kernel_cell_kind = match layer_lit.value().as_str() {
        "exec" => quote! { ::axiom_kernel::cell::CellKind::Exec },
        "validate" => quote! { ::axiom_kernel::cell::CellKind::Validate },
        "agent" => quote! { ::axiom_kernel::cell::CellKind::Agent },
        "oversight" => quote! { ::axiom_kernel::cell::CellKind::Oversight },
        _ => unreachable!(),
    };

    let expanded = quote! {
        #input

        impl ::axiom_kernel::cell::Cell for #name {
            fn cell_id(&self) -> ::axiom_kernel::id::CellId {
                self.id.clone()
            }
            fn cell_kind(&self) -> ::axiom_kernel::cell::CellKind {
                #kernel_cell_kind
            }
        }

        impl ::axiom_kernel::witness::WitnessGenerator for #name {
            fn generate_witness(&self, _event: ::axiom_kernel::witness::WitnessEvent) -> ::axiom_kernel::witness::Witness {
                ::axiom_kernel::witness::Witness {
                    witness_id: ::axiom_kernel::id::WitnessId::new(format!("wit-auto-{}", ::axiom_kernel::clock::global_clock().now_ns())),
                    schema_version: <::axiom_kernel::version::WitnessSchema as ::axiom_kernel::version::Versioned>::schema_version(),
                    cell_id: #name_str.to_string(),
                    correlation_id: ::axiom_kernel::id::CorrelationId::new("auto"),
                    trace_id: None,
                    triggering_msg_id: None,
                    vector_clock: ::axiom_kernel::signal::VectorClock::new(),
                    timestamp_ns: ::axiom_kernel::clock::global_clock().now_ns(),
                    prev_hash: None,
                    state_before_hash: None,
                    state_after_hash: None,
                    hash: ::axiom_kernel::witness::WitnessHash::zero(),
                    summary: "auto-generated".to_string(),
                    outcome: ::axiom_kernel::witness::TransitionOutcome::Success,
                    metrics: ::axiom_kernel::witness::WitnessMetrics::default(),
                    version_info: ::axiom_kernel::version::VersionInfo::current(),
                    signal_fingerprint: [0u8; 32],
                    payload_size_bytes: 0,
                    kind: ::axiom_kernel::witness::WitnessKind::StateTransition,
                }
            }
        }
    };

    TokenStream::from(expanded)
}