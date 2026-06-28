//! Axiom Macros - Procedural macros for compile-time gates and ergonomic API.
//!
//! # Macros
//! - `#[derive(SignalPayload)]`: Auto-implement Signal trait with required metadata
//! - `#[cell(layer = "exec")]`: Register a Cell with compile-time layer enforcement
//! - `#[axiom]`: Register an Axiom for automatic discovery
//! - `#[schema_version(N)]`: Implement Versioned trait with given schema version
//! - `#[migration(from = N)]`: Register a Migration with compile-time gap detection

extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, ItemImpl, ItemStruct, LitInt};

fn parse_layer_variant(lit: &syn::LitStr) -> Result<TokenStream2, syn::Error> {
    match lit.value().as_str() {
        "exec" => Ok(quote! { ::axiom_core::Layer::Exec }),
        "validate" => Ok(quote! { ::axiom_core::Layer::Validate }),
        "agent" => Ok(quote! { ::axiom_core::Layer::Agent }),
        "oversight" => Ok(quote! { ::axiom_core::Layer::Oversight }),
        other => Err(syn::Error::new(
            lit.span(),
            format!("invalid layer '{}': expected exec|validate|agent|oversight", other),
        )),
    }
}

fn parse_signal_kind(lit: &syn::LitStr) -> Result<TokenStream2, syn::Error> {
    match lit.value().as_str() {
        "command" => Ok(quote! { ::axiom_core::SignalKind::Command }),
        "event" => Ok(quote! { ::axiom_core::SignalKind::Event }),
        "query" => Ok(quote! { ::axiom_core::SignalKind::Query }),
        "response" => Ok(quote! { ::axiom_core::SignalKind::Response }),
        other => Err(syn::Error::new(
            lit.span(),
            format!(
                "invalid signal kind '{}': expected command|event|query|response",
                other
            ),
        )),
    }
}

/// Derive macro for Signal types. Auto-generates the Signal trait implementation.
///
/// # Attributes
/// - `#[signal(kind = "command", layer = "exec")]`: required on the struct
///
/// # Required fields
/// - `msg_id: MsgId`
/// - `correlation_id: CorrelationId`
/// - `vector_clock: VectorClock`
///
/// # Optional fields
/// - `trace_id: Option<TraceId>`
#[proc_macro_derive(SignalPayload, attributes(signal))]
pub fn derive_signal_payload(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let mut kind = quote! { ::axiom_core::SignalKind::Command };
    let mut layer = quote! { ::axiom_core::Layer::Exec };

    for attr in &input.attrs {
        if attr.path().is_ident("signal") {
            let _ = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("kind") {
                    let value = meta.value()?;
                    let lit: syn::LitStr = value.parse()?;
                    kind = parse_signal_kind(&lit)?;
                } else if meta.path.is_ident("layer") {
                    let value = meta.value()?;
                    let lit: syn::LitStr = value.parse()?;
                    layer = parse_layer_variant(&lit)?;
                }
                Ok(())
            });
        }
    }

    let expanded = quote! {
        impl #impl_generics ::axiom_core::Signal for #name #ty_generics #where_clause {
            fn signal_type(&self) -> &'static str {
                stringify!(#name)
            }
            fn msg_id(&self) -> &::axiom_core::MsgId {
                &self.msg_id
            }
            fn correlation_id(&self) -> &::axiom_core::CorrelationId {
                &self.correlation_id
            }
            fn vector_clock(&self) -> &::axiom_core::VectorClock {
                &self.vector_clock
            }
            fn timestamp_ns(&self) -> u64 {
                ::axiom_core::signal::now_ns()
            }
            fn kind(&self) -> ::axiom_core::SignalKind {
                #kind
            }
            fn layer(&self) -> ::axiom_core::Layer {
                #layer
            }
        }

        impl #impl_generics ::axiom_core::Schema for #name #ty_generics #where_clause {
            fn validate(&self) -> ::axiom_core::ValidationResult {
                ::axiom_core::ValidationResult::ok()
            }
        }
    };

    TokenStream::from(expanded)
}

/// Attribute macro for Cell implementations. Registers the cell and enforces layer marker.
///
/// # Usage
/// ```ignore
/// #[axiom_macros::cell(layer = "exec")]
/// impl Cell for MyCell { ... }
/// ```
#[proc_macro_attribute]
pub fn cell(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemImpl);
    let layer_lit = parse_macro_input!(attr as syn::LitStr);

    let layer_variant = match parse_layer_variant(&layer_lit) {
        Ok(lv) => lv,
        Err(e) => return e.to_compile_error().into(),
    };

    let struct_type = match &*input.self_ty {
        syn::Type::Path(tp) => &tp.path,
        _ => {
            return syn::Error::new_spanned(&input.self_ty, "cell macro expects impl Cell for Type")
                .to_compile_error()
                .into();
        }
    };

    let marker_impl = match layer_lit.value().as_str() {
        "exec" => quote! { impl ::axiom_core::ExecCell for #struct_type {} },
        "validate" => quote! { impl ::axiom_core::ValidateCell for #struct_type {} },
        "agent" => quote! { impl ::axiom_core::AgentCell for #struct_type {} },
        "oversight" => quote! { impl ::axiom_core::OversightCell for #struct_type {} },
        _ => unreachable!(),
    };

    let layer_of_impl = quote! {
        impl ::axiom_core::cell::LayerOf for #struct_type {
            const LAYER: ::axiom_core::Layer = #layer_variant;
        }
    };

    let expanded = quote! {
        #input

        #marker_impl
        #layer_of_impl
    };

    TokenStream::from(expanded)
}

/// Attribute macro for Axiom implementations. Marks the struct as an axiom for documentation.
#[proc_macro_attribute]
pub fn axiom(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let expanded = quote! {
        #[derive(Debug)]
        #input
    };
    TokenStream::from(expanded)
}

/// Attribute macro for setting schema version on a type.
///
/// # Usage
/// ```ignore
/// #[schema_version(2)]
/// #[derive(Serialize, Deserialize)]
/// struct MySignal { ... }
/// ```
#[proc_macro_attribute]
pub fn schema_version(attr: TokenStream, item: TokenStream) -> TokenStream {
    let version = parse_macro_input!(attr as LitInt);
    let input = parse_macro_input!(item as DeriveInput);
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let version_val: u16 = match version.base10_parse() {
        Ok(v) => v,
        Err(e) => return e.to_compile_error().into(),
    };

    if version_val == 0 {
        return syn::Error::new(version.span(), "schema version must be >= 1")
            .to_compile_error()
            .into();
    }

    let expanded = quote! {
        #input

        impl #impl_generics ::axiom_core::Versioned for #name #ty_generics #where_clause {
            fn schema_version() -> ::axiom_core::SchemaVersion {
                ::axiom_core::SchemaVersion::new(#version_val)
            }
        }
    };

    TokenStream::from(expanded)
}

/// Attribute macro for Migration implementations. Enforces compile-time to = from + 1.
///
/// # Usage
/// ```ignore
/// #[migration(from = 1)]
/// struct MigrateV1toV2;
/// impl Migration for MigrateV1toV2 { ... }
/// ```
#[proc_macro_attribute]
pub fn migration(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let name = &input.ident;

    let attr2: TokenStream2 = attr.into();
    let mut from_val: Option<u16> = None;
    let mut iter = attr2.into_iter();
    while let Some(tt) = iter.next() {
        if let proc_macro2::TokenTree::Ident(ident) = tt {
            if ident == "from" {
                let _eq = iter.next();
                if let Some(proc_macro2::TokenTree::Literal(lit)) = iter.next() {
                    let litstr = lit.to_string();
                    if let Ok(v) = litstr.parse::<u16>() {
                        from_val = Some(v);
                    }
                }
            }
        }
    }

    let from_v = match from_val {
        Some(v) if v >= 1 => v,
        _ => {
            return syn::Error::new_spanned(
                &input,
                "migration requires `from = N` where N >= 1",
            )
            .to_compile_error()
            .into();
        }
    };
    let to_v = from_v + 1;

    let expanded = quote! {
        #input

        impl ::axiom_core::Migration for #name {
            fn source_version(&self) -> ::axiom_core::SchemaVersion {
                ::axiom_core::SchemaVersion::new(#from_v)
            }
            fn target_version(&self) -> ::axiom_core::SchemaVersion {
                ::axiom_core::SchemaVersion::new(#to_v)
            }
        }
    };

    TokenStream::from(expanded)
}
