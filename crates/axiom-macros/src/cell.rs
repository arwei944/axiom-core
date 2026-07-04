use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemImpl};

use crate::utils::*;

pub fn impl_cell(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(item as ItemImpl);
    let layer_lit = parse_macro_input!(attr as syn::LitStr);

    let layer_variant = match parse_layer_variant(&layer_lit) {
        Ok(lv) => lv,
        Err(e) => return e.to_compile_error().into(),
    };
    let layer_marker = match parse_layer_marker(&layer_lit) {
        Ok(lm) => lm,
        Err(e) => return e.to_compile_error().into(),
    };

    let struct_type = match &*input.self_ty {
        syn::Type::Path(tp) => &tp.path,
        _ => {
            return syn::Error::new_spanned(
                &input.self_ty,
                "cell macro expects impl Cell for Type",
            )
            .to_compile_error()
            .into();
        }
    };

    let layer_assoc: syn::ImplItem = syn::parse_quote! {
        type Layer = #layer_marker;
    };
    input.items.insert(0, layer_assoc);

    let marker_impl = match layer_lit.value().as_str() {
        "exec" => quote! { impl ::axiom_core::cell::ExecCell for #struct_type {} },
        "validate" => quote! { impl ::axiom_core::cell::ValidateCell for #struct_type {} },
        "agent" => quote! { impl ::axiom_core::cell::AgentCell for #struct_type {} },
        "oversight" => quote! { impl ::axiom_core::cell::OversightCell for #struct_type {} },
        _ => unreachable!(),
    };

    let layer_of_impl = quote! {
        impl ::axiom_core::cell::LayerOf for #struct_type {
            const LAYER: ::axiom_core::Layer = #layer_variant;
        }
    };

    let witness_trait_impl = quote! {
        impl ::axiom_core::witness::WitnessGenerator for #struct_type {
            fn generate_witness(&self, _event: ::axiom_core::witness::WitnessEvent) -> ::axiom_core::witness::Witness {
                ::axiom_core::witness::Witness {
                    witness_id: ::axiom_core::id::WitnessId::new(format!("wit-auto-{}", ::axiom_core::clock::global_clock().now_ns())),
                    schema_version: <::axiom_core::version::WitnessSchema as ::axiom_core::Versioned>::schema_version(),
                    cell_id: stringify!(#struct_type).to_string(),
                    correlation_id: ::axiom_core::id::CorrelationId::new("auto"),
                    trace_id: None,
                    triggering_msg_id: None,
                    vector_clock: ::axiom_core::signal::VectorClock::new(),
                    timestamp_ns: ::axiom_core::clock::global_clock().now_ns(),
                    prev_hash: None,
                    state_before_hash: None,
                    state_after_hash: None,
                    hash: ::axiom_core::witness::WitnessHash::zero(),
                    summary: "auto-generated".to_string(),
                    outcome: ::axiom_core::witness::TransitionOutcome::Success,
                    metrics: ::axiom_core::witness::WitnessMetrics::default(),
                    version_info: ::axiom_core::version::VersionInfo::current(),
                    signal_fingerprint: [0u8; 32],
                    payload_size_bytes: 0,
                    kind: ::axiom_core::witness::WitnessKind::StateTransition,
                }
            }
        }
    };

    let expanded = quote! {
        #input

        #marker_impl
        #layer_of_impl
        #witness_trait_impl
    };

    TokenStream::from(expanded)
}
