use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{DeriveInput, LitInt};

pub fn parse_layer_marker(lit: &syn::LitStr) -> Result<TokenStream2, syn::Error> {
    match lit.value().as_str() {
        "exec" => Ok(quote! { ::axiom_kernel::sealed::ExecLayer }),
        "validate" => Ok(quote! { ::axiom_kernel::sealed::ValidateLayer }),
        "agent" => Ok(quote! { ::axiom_kernel::sealed::AgentLayer }),
        "oversight" => Ok(quote! { ::axiom_kernel::sealed::OversightLayer }),
        other => Err(syn::Error::new(
            lit.span(),
            format!(
                "invalid layer '{}': expected exec|validate|agent|oversight",
                other
            ),
        )),
    }
}

pub fn parse_layer_variant(lit: &syn::LitStr) -> Result<TokenStream2, syn::Error> {
    match lit.value().as_str() {
        "exec" => Ok(quote! { ::axiom_kernel::Layer::Exec }),
        "validate" => Ok(quote! { ::axiom_kernel::Layer::Validate }),
        "agent" => Ok(quote! { ::axiom_kernel::Layer::Agent }),
        "oversight" => Ok(quote! { ::axiom_kernel::Layer::Oversight }),
        other => Err(syn::Error::new(
            lit.span(),
            format!(
                "invalid layer '{}': expected exec|validate|agent|oversight",
                other
            ),
        )),
    }
}

pub fn parse_signal_kind(lit: &syn::LitStr) -> Result<TokenStream2, syn::Error> {
    match lit.value().as_str() {
        "command" => Ok(quote! { ::axiom_kernel::signal::SignalKind::Command }),
        "event" => Ok(quote! { ::axiom_kernel::signal::SignalKind::Event }),
        "query" => Ok(quote! { ::axiom_kernel::signal::SignalKind::Query }),
        "response" => Ok(quote! { ::axiom_kernel::signal::SignalKind::Response }),
        other => Err(syn::Error::new(
            lit.span(),
            format!(
                "invalid signal kind '{}': expected command|event|query|response",
                other
            ),
        )),
    }
}

pub fn find_schema_version(attrs: &[syn::Attribute]) -> Option<u16> {
    for attr in attrs {
        if attr.path().is_ident("schema_version") {
            if let Ok(lit) = attr.parse_args::<LitInt>() {
                if let Ok(v) = lit.base10_parse::<u16>() {
                    return Some(v);
                }
            }
        }
    }
    None
}

pub fn has_trace_id_field(input: &DeriveInput) -> bool {
    if let syn::Data::Struct(data) = &input.data {
        for field in &data.fields {
            if let Some(ident) = &field.ident {
                if ident == "trace_id" {
                    return true;
                }
            }
        }
    }
    false
}

pub fn has_sender_field(input: &DeriveInput) -> bool {
    if let syn::Data::Struct(data) = &input.data {
        for field in &data.fields {
            if let Some(ident) = &field.ident {
                if ident == "sender" {
                    return true;
                }
            }
        }
    }
    false
}
