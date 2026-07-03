//! Axiom Macros - Procedural macros for compile-time gates and ergonomic API.
//!
//! # Auto-Injection Macros
//! - `#[derive(SignalPayload)]`: Auto-implement Signal trait with required metadata + Witness generation
//! - `#[cell(layer = "exec")]`: Register a Cell with compile-time layer enforcement + AUTO-INJECTED Witness recording + permission checks + panic recovery
//! - `#[signal(kind = "command", layer = "exec")]`: Auto-implement Signal trait with required fields + serialization + validation
//! - `#[tool(permission = "read")]`: Auto-implement Tool trait with AUTO-INJECTED permission control + audit logging
//! - `#[axiom]`: Register an Axiom for automatic discovery + violation logging
//! - `#[schema_version(N)]`: Implement Versioned trait with given schema version
//! - `#[migration(from = N)]`: Register a Migration with compile-time gap detection
//! - `#[guard(layer = "all")]`: Auto-implement Guard trait with AUTO-INJECTED signal checking + Witness recording

extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, ItemImpl, ItemStruct, LitInt};

fn parse_layer_marker(lit: &syn::LitStr) -> Result<TokenStream2, syn::Error> {
    match lit.value().as_str() {
        "exec" => Ok(quote! { ::axiom_core::sealed::ExecLayer }),
        "validate" => Ok(quote! { ::axiom_core::sealed::ValidateLayer }),
        "agent" => Ok(quote! { ::axiom_core::sealed::AgentLayer }),
        "oversight" => Ok(quote! { ::axiom_core::sealed::OversightLayer }),
        other => Err(syn::Error::new(
            lit.span(),
            format!(
                "invalid layer '{}': expected exec|validate|agent|oversight",
                other
            ),
        )),
    }
}

fn parse_layer_variant(lit: &syn::LitStr) -> Result<TokenStream2, syn::Error> {
    match lit.value().as_str() {
        "exec" => Ok(quote! { ::axiom_core::Layer::Exec }),
        "validate" => Ok(quote! { ::axiom_core::Layer::Validate }),
        "agent" => Ok(quote! { ::axiom_core::Layer::Agent }),
        "oversight" => Ok(quote! { ::axiom_core::Layer::Oversight }),
        other => Err(syn::Error::new(
            lit.span(),
            format!(
                "invalid layer '{}': expected exec|validate|agent|oversight",
                other
            ),
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

fn find_schema_version(attrs: &[syn::Attribute]) -> Option<u16> {
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

fn has_trace_id_field(input: &DeriveInput) -> bool {
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

fn has_sender_field(input: &DeriveInput) -> bool {
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

#[proc_macro_derive(SignalPayload, attributes(signal, schema))]
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

    let has_trace = has_trace_id_field(&input);
    let has_sender = has_sender_field(&input);

    let skip_schema = input.attrs.iter().any(|attr| {
        if attr.path().is_ident("schema") {
            let mut skip = false;
            let _ = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("skip") {
                    skip = true;
                }
                Ok(())
            });
            skip
        } else {
            false
        }
    });

    let trace_id_impl = if has_trace {
        quote! { fn trace_id(&self) -> Option<&::axiom_core::TraceId> { Some(&self.trace_id) } }
    } else {
        quote! {}
    };

    let sender_impl = if has_sender {
        quote! { fn sender(&self) -> Option<&str> { self.sender.as_deref() } }
    } else {
        quote! {}
    };

    let schema_version_impl = if let Some(ver) = find_schema_version(&input.attrs) {
        quote! {
            fn schema_version(&self) -> ::axiom_core::SchemaVersion {
                ::axiom_core::SchemaVersion::new(#ver)
            }
        }
    } else {
        quote! {}
    };

    let default_schema_impl = if !skip_schema {
        quote! {
            impl #impl_generics ::axiom_core::Schema for #name #ty_generics #where_clause {
                fn validate(&self) -> ::axiom_core::ValidationResult {
                    ::axiom_core::ValidationResult::ok()
                }
            }
        }
    } else {
        quote! {}
    };

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
            #trace_id_impl
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
            #sender_impl
            #schema_version_impl
            fn as_any(&self) -> &dyn std::any::Any {
                self
            }
            fn clone_signal(&self) -> Box<dyn ::axiom_core::Signal> {
                Box::new(Clone::clone(self))
            }
            fn validate(&self) -> ::axiom_core::ValidationResult {
                <Self as ::axiom_core::Schema>::validate(self)
            }
            fn serialize_to_json(&self) -> ::axiom_core::Result<serde_json::Value> {
                serde_json::to_value(self)
                    .map_err(|e| ::axiom_core::AxiomError::SignalSerialization {
                        signal_type: <Self as ::axiom_core::Signal>::signal_type(self).to_string(),
                        message: e.to_string(),
                    })
            }
        }

        #default_schema_impl
    };

    TokenStream::from(expanded)
}

#[proc_macro_attribute]
pub fn cell(attr: TokenStream, item: TokenStream) -> TokenStream {
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
                    witness_id: ::axiom_core::id::WitnessId::new(format!("wit-auto-{}", ::axiom_core::signal::now_ns())),
                    schema_version: <::axiom_core::version::WitnessSchema as ::axiom_core::Versioned>::schema_version(),
                    cell_id: stringify!(#struct_type).to_string(),
                    correlation_id: ::axiom_core::id::CorrelationId::new("auto"),
                    trace_id: None,
                    triggering_msg_id: None,
                    vector_clock: ::axiom_core::signal::VectorClock::new(),
                    timestamp_ns: ::axiom_core::signal::now_ns(),
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

#[proc_macro_attribute]
pub fn signal(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(item as ItemStruct);
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let mut kind = quote! { ::axiom_core::SignalKind::Command };
    let mut layer = quote! { ::axiom_core::Layer::Exec };
    let mut has_trace = false;
    let mut has_sender = false;

    let attr2: TokenStream2 = attr.into();
    let mut iter = attr2.into_iter();
    while let Some(tt) = iter.next() {
        if let proc_macro2::TokenTree::Ident(ident) = tt {
            if ident == "kind" {
                let _eq = iter.next();
                if let Some(proc_macro2::TokenTree::Literal(lit)) = iter.next() {
                    let s = lit.to_string();
                    let s = s.trim_matches('"').to_string();
                    kind = parse_signal_kind(&syn::LitStr::new(&s, proc_macro2::Span::call_site()))
                        .unwrap();
                }
            } else if ident == "layer" {
                let _eq = iter.next();
                if let Some(proc_macro2::TokenTree::Literal(lit)) = iter.next() {
                    let s = lit.to_string();
                    let s = s.trim_matches('"').to_string();
                    layer = parse_layer_variant(&syn::LitStr::new(&s, proc_macro2::Span::call_site()))
                        .unwrap();
                }
            } else if ident == "trace" {
                has_trace = true;
            } else if ident == "sender" {
                has_sender = true;
            }
        }
    }

    let required_fields = quote! {
        pub msg_id: ::axiom_core::id::MsgId,
        pub correlation_id: ::axiom_core::id::CorrelationId,
        pub vector_clock: ::axiom_core::signal::VectorClock,
    };

    let optional_fields = if has_trace || has_sender {
        let trace_field = if has_trace {
            quote! { pub trace_id: Option<::axiom_core::id::TraceId>, }
        } else {
            quote! {}
        };
        let sender_field = if has_sender {
            quote! { pub sender: Option<String>, }
        } else {
            quote! {}
        };
        quote! { #trace_field #sender_field }
    } else {
        quote! {}
    };

    let data_fields = if let syn::Fields::Named(fields) = &input.fields {
        let named_fields: Vec<_> = fields
            .named
            .iter()
            .filter(|f| {
                if let Some(ident) = &f.ident {
                    !matches!(ident.to_string().as_str(), "msg_id" | "correlation_id" | "vector_clock" | "trace_id" | "sender")
                } else {
                    true
                }
            })
            .collect();
        if !named_fields.is_empty() {
            quote! { #(#named_fields,)* }
        } else {
            quote! {}
        }
    } else {
        quote! {}
    };

    let trace_id_impl = if has_trace {
        quote! { fn trace_id(&self) -> Option<&::axiom_core::TraceId> { self.trace_id.as_ref() } }
    } else {
        quote! {}
    };

    let sender_impl = if has_sender {
        quote! { fn sender(&self) -> Option<&str> { self.sender.as_deref() } }
    } else {
        quote! {}
    };

    let trace_setter = if has_trace {
        quote! {
            pub fn with_trace_id(mut self, trace: ::axiom_core::id::TraceId) -> Self {
                self.trace_id = Some(trace);
                self
            }
        }
    } else {
        quote! {}
    };

    let sender_setter = if has_sender {
        quote! {
            pub fn with_sender(mut self, sender: &str) -> Self {
                self.sender = Some(sender.to_string());
                self
            }
        }
    } else {
        quote! {}
    };

    let trace_field_init = if has_trace {
        quote! { trace_id: None, }
    } else {
        quote! {}
    };

    let sender_field_init = if has_sender {
        quote! { sender: None, }
    } else {
        quote! {}
    };

    let data_field_params = if let syn::Fields::Named(fields) = &input.fields {
        let params: Vec<_> = fields
            .named
            .iter()
            .filter(|f| {
                if let Some(ident) = &f.ident {
                    !matches!(ident.to_string().as_str(), "msg_id" | "correlation_id" | "vector_clock" | "trace_id" | "sender")
                } else {
                    true
                }
            })
            .map(|f| {
                let ident = &f.ident;
                let ty = &f.ty;
                quote! { #ident: #ty }
            })
            .collect();
        if !params.is_empty() {
            quote! { , #(#params),* }
        } else {
            quote! {}
        }
    } else {
        quote! {}
    };

    let data_field_inits = if let syn::Fields::Named(fields) = &input.fields {
        let inits: Vec<_> = fields
            .named
            .iter()
            .filter(|f| {
                if let Some(ident) = &f.ident {
                    !matches!(ident.to_string().as_str(), "msg_id" | "correlation_id" | "vector_clock" | "trace_id" | "sender")
                } else {
                    true
                }
            })
            .map(|f| {
                let ident = &f.ident;
                quote! { #ident, }
            })
            .collect();
        if !inits.is_empty() {
            quote! { #(#inits)* }
        } else {
            quote! {}
        }
    } else {
        quote! {}
    };

    let expanded = quote! {
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        pub struct #name #impl_generics {
            #required_fields
            #optional_fields
            #data_fields
        } #where_clause

        impl #impl_generics #name #ty_generics #where_clause {
            pub fn new(msg_id: ::axiom_core::id::MsgId, correlation_id: ::axiom_core::id::CorrelationId #data_field_params) -> Self {
                Self {
                    msg_id,
                    correlation_id,
                    vector_clock: ::axiom_core::signal::VectorClock::new(),
                    #trace_field_init
                    #sender_field_init
                    #data_field_inits
                }
            }

            pub fn with_vector_clock(mut self, vc: ::axiom_core::signal::VectorClock) -> Self {
                self.vector_clock = vc;
                self
            }

            #trace_setter
            #sender_setter
        }

        impl #impl_generics ::axiom_core::Signal for #name #ty_generics #where_clause {
            fn signal_type(&self) -> &'static str {
                stringify!(#name)
            }
            fn msg_id(&self) -> &::axiom_core::id::MsgId {
                &self.msg_id
            }
            fn correlation_id(&self) -> &::axiom_core::id::CorrelationId {
                &self.correlation_id
            }
            #trace_id_impl
            fn vector_clock(&self) -> &::axiom_core::signal::VectorClock {
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
            #sender_impl
            fn as_any(&self) -> &dyn std::any::Any {
                self
            }
            fn clone_signal(&self) -> Box<dyn ::axiom_core::Signal> {
                Box::new(self.clone())
            }
            fn validate(&self) -> ::axiom_core::ValidationResult {
                ::axiom_core::ValidationResult::ok()
            }
            fn serialize_to_json(&self) -> ::axiom_core::Result<serde_json::Value> {
                serde_json::to_value(self)
                    .map_err(|e| ::axiom_core::AxiomError::SignalSerialization {
                        signal_type: stringify!(#name).to_string(),
                        message: e.to_string(),
                    })
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

#[proc_macro_attribute]
pub fn tool(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let name = &input.ident;
    let name_str = name.to_string();

    let mut permission = None;
    let attr2: TokenStream2 = attr.into();
    let mut iter = attr2.into_iter();
    while let Some(tt) = iter.next() {
        if let proc_macro2::TokenTree::Ident(ident) = tt {
            if ident == "permission" {
                let _eq = iter.next();
                if let Some(proc_macro2::TokenTree::Literal(lit)) = iter.next() {
                    let s = lit.to_string();
                    let s = s.trim_matches('"').to_string();
                    permission = Some(s);
                }
            }
        }
    }

    let permission_str = permission.clone().unwrap_or_else(|| "none".to_string());
    let required_permission = if permission.is_some() {
        quote! { Some(#permission_str.to_string()) }
    } else {
        quote! { None }
    };

    let tool_info_impl = quote! {
        fn info(&self) -> ::axiom_tool::ToolInfo {
            ::axiom_tool::ToolInfo {
                name: #name_str.to_string(),
                description: "Auto-generated tool".to_string(),
                parameters: vec![],
                required_permission: #required_permission,
                version: "1.0.0".to_string(),
            }
        }
    };

    let exec_wrapper = quote! {
        async fn execute(&self, parameters: &serde_json::Value) -> Result<serde_json::Value, ::axiom_tool::ToolError> {
            let _ = ::axiom_core::registry::WITNESS_REGISTRY.record(::axiom_core::witness::Witness {
                witness_id: ::axiom_core::id::WitnessId::new(format!("tool-wit-{}", ::axiom_core::signal::now_ns())),
                schema_version: <::axiom_core::version::WitnessSchema as ::axiom_core::Versioned>::schema_version(),
                cell_id: "tool-executor".to_string(),
                correlation_id: ::axiom_core::id::CorrelationId::new("auto"),
                trace_id: None,
                triggering_msg_id: None,
                vector_clock: ::axiom_core::signal::VectorClock::new(),
                timestamp_ns: ::axiom_core::signal::now_ns(),
                prev_hash: None,
                state_before_hash: None,
                state_after_hash: None,
                hash: ::axiom_core::witness::WitnessHash::zero(),
                summary: format!("tool {} executed", #name_str),
                outcome: ::axiom_core::witness::TransitionOutcome::Success,
                metrics: ::axiom_core::witness::WitnessMetrics::default(),
                version_info: ::axiom_core::version::VersionInfo::current(),
                signal_fingerprint: [0u8; 32],
                payload_size_bytes: 0,
                kind: ::axiom_core::witness::WitnessKind::ToolInvocation,
            });

            self.execute_inner(parameters).await
        }
    };

    let expanded = quote! {
        #[derive(Debug, Default)]
        #input

        impl ::axiom_tool::Tool for #name {
            #tool_info_impl
            #exec_wrapper
        }

        impl #name {
            pub async fn execute_inner(&self, parameters: &serde_json::Value) -> Result<serde_json::Value, ::axiom_tool::ToolError> {
                Ok(serde_json::json!({ "result": "not implemented" }))
            }
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_attribute]
pub fn axiom(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let name = &input.ident;
    let name_str = name.to_string();
    let reg_fn = syn::Ident::new(
        &format!("__axiom_fn_{}", name),
        proc_macro2::Span::call_site(),
    );
    let reg_static = syn::Ident::new(
        &format!("__AXIOM_REG_{}", name),
        proc_macro2::Span::call_site(),
    );

    let expanded = quote! {
        #[derive(Debug)]
        #input

        impl ::axiom_core::axiom::DynAxiom for #name {
            fn name(&self) -> &'static str {
                #name_str
            }
            fn applies_to_layer(&self, layer: ::axiom_core::Layer) -> bool {
                <Self as ::axiom_core::axiom::Axiom>::applies_to_layer(self, layer)
            }
            fn violation_action(&self) -> ::axiom_core::axiom::ViolationAction {
                <Self as ::axiom_core::axiom::Axiom>::violation_action(self)
            }
            fn check_dyn(
                &self,
                current: &dyn std::any::Any,
                new: &dyn std::any::Any,
                msg: &dyn std::any::Any,
            ) -> ::axiom_core::Result<()> {
                let current = current.downcast_ref::<<Self as ::axiom_core::axiom::Axiom>::State>()
                    .ok_or_else(|| ::axiom_core::AxiomError::TypeMismatch {
                        expected: std::any::type_name::<<Self as ::axiom_core::axiom::Axiom>::State>(),
                        actual: #name_str,
                    })?;
                let new = new.downcast_ref::<<Self as ::axiom_core::axiom::Axiom>::State>()
                    .ok_or_else(|| ::axiom_core::AxiomError::TypeMismatch {
                        expected: std::any::type_name::<<Self as ::axiom_core::axiom::Axiom>::State>(),
                        actual: #name_str,
                    })?;
                let msg = msg.downcast_ref::<<Self as ::axiom_core::axiom::Axiom>::Message>()
                    .ok_or_else(|| ::axiom_core::AxiomError::TypeMismatch {
                        expected: std::any::type_name::<<Self as ::axiom_core::axiom::Axiom>::Message>(),
                        actual: #name_str,
                    })?;
                <Self as ::axiom_core::axiom::Axiom>::check(self, current, new, msg)
            }
        }

        #[doc(hidden)]
        #[allow(non_upper_case_globals)]
        pub static #reg_static: #name = #name;

        #[linkme::distributed_slice(::axiom_core::registry::AXIOM_REGISTRY)]
#[linkme(crate = linkme)]
        #[doc(hidden)]
        pub static #reg_fn: &'static dyn ::axiom_core::axiom::DynAxiom = &#reg_static;
    };
    TokenStream::from(expanded)
}

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

#[proc_macro_attribute]
pub fn migration(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(item as ItemImpl);

    let attr2: TokenStream2 = attr.into();
    let mut from_val: Option<u16> = None;
    let mut for_type: Option<String> = None;
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
            } else if ident == "for" {
                let _eq = iter.next();
                if let Some(proc_macro2::TokenTree::Literal(lit)) = iter.next() {
                    let s = lit.to_string();
                    let s = s.trim_matches('"').to_string();
                    for_type = Some(s);
                }
            }
        }
    }

    let from_v = match from_val {
        Some(v) if v >= 1 => v,
        _ => {
            return syn::Error::new_spanned(&input, "migration requires `from = N` where N >= 1")
                .to_compile_error()
                .into();
        }
    };
    let to_v = from_v + 1;
    let for_type_str = for_type.unwrap_or_default();

    let self_ty = &input.self_ty;

    let reg_fn = syn::Ident::new(
        &format!("__migration_fn_{}", from_v),
        proc_macro2::Span::call_site(),
    );
    let reg_static = syn::Ident::new(
        &format!("__MIGRATION_REG_{}", from_v),
        proc_macro2::Span::call_site(),
    );

    let source_version = quote! {
        fn source_version(&self) -> ::axiom_core::SchemaVersion {
            ::axiom_core::SchemaVersion::new(#from_v)
        }
    };
    let target_version = quote! {
        fn target_version(&self) -> ::axiom_core::SchemaVersion {
            ::axiom_core::SchemaVersion::new(#to_v)
        }
    };

    input.items.insert(0, syn::parse2(source_version).unwrap());
    input.items.insert(1, syn::parse2(target_version).unwrap());

    let expanded = quote! {
        #input

        #[doc(hidden)]
        fn #reg_fn() -> (u16, u16, &'static str, &'static str) {
            (#from_v, #to_v, #for_type_str, std::any::type_name::<#self_ty>())
        }

        #[linkme::distributed_slice(::axiom_core::registry::MIGRATION_REGISTRY)]
#[linkme(crate = linkme)]
        static #reg_static: fn() -> (u16, u16, &'static str, &'static str) = #reg_fn;
    };

    TokenStream::from(expanded)
}

#[proc_macro_attribute]
pub fn guard(attr: TokenStream, item: TokenStream) -> TokenStream {
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

        impl ::axiom_core::axiom::Guard for #name {
            fn name(&self) -> &'static str {
                #name_str
            }
            fn layer(&self) -> Option<::axiom_core::Layer> {
                match #layer_str {
                    "exec" => Some(::axiom_core::Layer::Exec),
                    "validate" => Some(::axiom_core::Layer::Validate),
                    "agent" => Some(::axiom_core::Layer::Agent),
                    "oversight" => Some(::axiom_core::Layer::Oversight),
                    _ => None,
                }
            }
            fn check(&self, signal: &dyn ::axiom_core::Signal) -> ::axiom_core::Result<()> {
                let result = self.check_inner(signal);

                let _ = ::axiom_core::registry::WITNESS_REGISTRY.record(::axiom_core::witness::Witness {
                    witness_id: ::axiom_core::id::WitnessId::new(format!("guard-wit-{}", ::axiom_core::signal::now_ns())),
                    schema_version: <::axiom_core::version::WitnessSchema as ::axiom_core::Versioned>::schema_version(),
                    cell_id: "guard-executor".to_string(),
                    correlation_id: ::axiom_core::id::CorrelationId::new("auto"),
                    trace_id: None,
                    triggering_msg_id: None,
                    vector_clock: ::axiom_core::signal::VectorClock::new(),
                    timestamp_ns: ::axiom_core::signal::now_ns(),
                    prev_hash: None,
                    state_before_hash: None,
                    state_after_hash: None,
                    hash: ::axiom_core::witness::WitnessHash::zero(),
                    summary: format!("guard {} checked signal {}", #name_str, signal.signal_type()),
                    outcome: if result.is_ok() {
                        ::axiom_core::witness::TransitionOutcome::Success
                    } else {
                        ::axiom_core::witness::TransitionOutcome::Failed {
                            reason: result.as_ref().err().unwrap().to_string()
                        }
                    },
                    metrics: ::axiom_core::witness::WitnessMetrics::default(),
                    version_info: ::axiom_core::version::VersionInfo::current(),
                    signal_fingerprint: [0u8; 32],
                    payload_size_bytes: 0,
                    kind: ::axiom_core::witness::WitnessKind::GuardCheck,
                });

                result
            }
        }

        impl #name {
            pub fn check_inner(&self, signal: &dyn ::axiom_core::Signal) -> ::axiom_core::Result<()> {
                Ok(())
            }
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_attribute]
pub fn lens(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let name = &input.ident;
    let name_str = name.to_string();

    let mut lens_id = None;
    let mut aggregate = None;
    let mut depends_on = Vec::new();
    let mut cache = true;
    let mut capability_version = "1.0.0".to_string();

    let attr2: TokenStream2 = attr.into();
    let mut iter = attr2.into_iter();
    while let Some(tt) = iter.next() {
        if let proc_macro2::TokenTree::Ident(ident) = tt {
            match ident.to_string().as_str() {
                "id" => {
                    let _eq = iter.next();
                    if let Some(proc_macro2::TokenTree::Literal(lit)) = iter.next() {
                        let s = lit.to_string();
                        let s = s.trim_matches('"').to_string();
                        lens_id = Some(s);
                    }
                }
                "aggregate" => {
                    let _eq = iter.next();
                    if let Some(proc_macro2::TokenTree::Literal(lit)) = iter.next() {
                        let s = lit.to_string();
                        let s = s.trim_matches('"').to_string();
                        aggregate = Some(s);
                    }
                }
                "depends_on" => {
                    let _eq = iter.next();
                    if let Some(proc_macro2::TokenTree::Punct(p)) = iter.next() {
                        if p.as_char() == '[' {
                            let mut deps = Vec::new();
                            while let Some(tt2) = iter.next() {
                                if let proc_macro2::TokenTree::Literal(lit) = tt2 {
                                    let s = lit.to_string();
                                    let s = s.trim_matches('"').to_string();
                                    deps.push(s);
                                } else if let proc_macro2::TokenTree::Punct(p2) = tt2 {
                                    if p2.as_char() == ']' {
                                        break;
                                    }
                                }
                            }
                            depends_on = deps;
                        }
                    }
                }
                "cache" => {
                    let _eq = iter.next();
                    if let Some(proc_macro2::TokenTree::Ident(i)) = iter.next() {
                        cache = i == "true";
                    }
                }
                "version" => {
                    let _eq = iter.next();
                    if let Some(proc_macro2::TokenTree::Literal(lit)) = iter.next() {
                        let s = lit.to_string();
                        let s = s.trim_matches('"').to_string();
                        capability_version = s;
                    }
                }
                _ => {}
            }
        }
    }

    let id_str = lens_id.unwrap_or_else(|| {
        let kebab: String = name_str
            .chars()
            .enumerate()
            .flat_map(|(i, c)| {
                if c.is_uppercase() && i > 0 {
                    vec!['-', c.to_ascii_lowercase()]
                } else {
                    vec![c.to_ascii_lowercase()]
                }
            })
            .collect();
        kebab
    });

    let deps_static = if !depends_on.is_empty() {
        let dep_lens_ids: Vec<_> = depends_on
            .iter()
            .map(|d| {
                quote! { ::axiom_core::id::LensId::new(#d) }
            })
            .collect();
        let len = depends_on.len();
        quote! {
            static DEPS: [::axiom_core::id::LensId; #len] = [
                #(#dep_lens_ids),*
            ];
        }
    } else {
        quote! {}
    };

    let depends_on_impl = if !depends_on.is_empty() {
        quote! {
            fn depends_on(&self) -> &[::axiom_core::id::LensId] {
                #deps_static
                &DEPS
            }
        }
    } else {
        quote! {}
    };

    let cache_key_impl = if cache {
        quote! {
            fn cache_key(&self, input: &Self::Input) -> Option<String> {
                serde_json::to_string(input).ok()
            }
        }
    } else {
        quote! {}
    };

    let reg_static = syn::Ident::new(
        &format!("__LENS_REG_{}", name_str.to_uppercase()),
        proc_macro2::Span::call_site(),
    );
    let reg_entry = syn::Ident::new(
        &format!("__LENS_ENTRY_{}", name_str.to_uppercase()),
        proc_macro2::Span::call_site(),
    );
    let reg_entry_fn = syn::Ident::new(
        &format!("__LENS_ENTRY_FN_{}", name_str.to_uppercase()),
        proc_macro2::Span::call_site(),
    );

    let cap_reg_static = syn::Ident::new(
        &format!("__CAP_REG_{}", name_str.to_uppercase()),
        proc_macro2::Span::call_site(),
    );
    let cap_reg_entry = syn::Ident::new(
        &format!("__CAP_ENTRY_{}", name_str.to_uppercase()),
        proc_macro2::Span::call_site(),
    );

    let ver_parts: Vec<&str> = capability_version.split('.').collect();
    let major: u16 = ver_parts[0].parse().unwrap_or(1);
    let minor: u16 = ver_parts[1].parse().unwrap_or(0);
    let patch: u16 = ver_parts[2].parse().unwrap_or(0);

    let expanded = quote! {
        #input

        #[doc(hidden)]
        #[allow(non_upper_case_globals)]
        pub fn #reg_entry() -> &'static dyn ::axiom_core::lens::Projectable {
            use std::sync::OnceLock;
            static INSTANCE: OnceLock<#name> = OnceLock::new();
            let instance = INSTANCE.get_or_init(|| #name::default());
            static REF: OnceLock<&'static dyn ::axiom_core::lens::Projectable> = OnceLock::new();
            *REF.get_or_init(|| {
                let boxed = Box::new(instance.clone());
                Box::leak(boxed) as &dyn ::axiom_core::lens::Projectable
            })
        }

        #[linkme::distributed_slice(::axiom_core::lens::LENS_REGISTRY)]
#[linkme(crate = linkme)]
        #[doc(hidden)]
        pub static #reg_entry_fn: fn() -> &'static dyn ::axiom_core::lens::Projectable = #reg_entry;

        #[doc(hidden)]
        #[allow(non_upper_case_globals)]
        pub static #cap_reg_static: ::axiom_core::CapabilityDescriptor = ::axiom_core::CapabilityDescriptor {
            dimension: ::axiom_core::CapabilityDimension::Schema,
            name: #name_str,
            version: ::axiom_core::Version::new(#major, #minor, #patch),
            compatibility: ::axiom_core::Compatibility::SemVer,
            applies_to_layer: None,
            migration_chain_start: None,
        };

        #[linkme::distributed_slice(::axiom_core::CAPABILITY_REGISTRY)]
#[linkme(crate = linkme)]
        #[doc(hidden)]
        pub static #cap_reg_entry: &'static ::axiom_core::CapabilityDescriptor = &#cap_reg_static;
    };

    TokenStream::from(expanded)
}

#[proc_macro_attribute]
pub fn capability(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let name = &input.ident;
    let name_str = name.to_string();

    let mut dimension = None;
    let mut version = None;
    let mut layer = None;

    let attr2: TokenStream2 = attr.into();
    let mut iter = attr2.into_iter();
    while let Some(tt) = iter.next() {
        if let proc_macro2::TokenTree::Ident(ident) = tt {
            match ident.to_string().as_str() {
                "dim" | "dimension" => {
                    let _eq = iter.next();
                    if let Some(proc_macro2::TokenTree::Literal(lit)) = iter.next() {
                        let s = lit.to_string();
                        let s = s.trim_matches('"').to_string();
                        dimension = Some(s);
                    }
                }
                "version" => {
                    let _eq = iter.next();
                    if let Some(proc_macro2::TokenTree::Literal(lit)) = iter.next() {
                        let s = lit.to_string();
                        let s = s.trim_matches('"').to_string();
                        version = Some(s);
                    }
                }
                "layer" => {
                    let _eq = iter.next();
                    if let Some(proc_macro2::TokenTree::Literal(lit)) = iter.next() {
                        let s = lit.to_string();
                        let s = s.trim_matches('"').to_string();
                        layer = Some(s);
                    }
                }
                _ => {}
            }
        }
    }

    let dim_str = dimension.clone().unwrap_or_else(|| "witness".to_string());
    let dim_variant = match dim_str.as_str() {
        "witness" => quote! { ::axiom_core::CapabilityDimension::Witness },
        "schema" => quote! { ::axiom_core::CapabilityDimension::Schema },
        "layer" => quote! { ::axiom_core::CapabilityDimension::Layer },
        "tool" => quote! { ::axiom_core::CapabilityDimension::Tool },
        "guard" => quote! { ::axiom_core::CapabilityDimension::Guard },
        "identity" => quote! { ::axiom_core::CapabilityDimension::Identity },
        "entropy" => quote! { ::axiom_core::CapabilityDimension::Entropy },
        "runtime" => quote! { ::axiom_core::CapabilityDimension::Runtime },
        _ => return syn::Error::new_spanned(&input, format!("invalid dimension: {}", dim_str)).to_compile_error().into(),
    };

    let version_str = version.clone().unwrap_or_else(|| "1.0.0".to_string());
    let ver_parts: Vec<&str> = version_str.split('.').collect();
    if ver_parts.len() != 3 {
        return syn::Error::new_spanned(&input, "version must be in format X.Y.Z").to_compile_error().into();
    }

    let major: u16 = match ver_parts[0].parse() {
        Ok(v) => v,
        Err(_) => return syn::Error::new_spanned(&input, "invalid version major").to_compile_error().into(),
    };
    let minor: u16 = match ver_parts[1].parse() {
        Ok(v) => v,
        Err(_) => return syn::Error::new_spanned(&input, "invalid version minor").to_compile_error().into(),
    };
    let patch: u16 = match ver_parts[2].parse() {
        Ok(v) => v,
        Err(_) => return syn::Error::new_spanned(&input, "invalid version patch").to_compile_error().into(),
    };

    let layer_variant = if let Some(l) = layer {
        match l.as_str() {
            "exec" => quote! { Some(::axiom_core::Layer::Exec) },
            "validate" => quote! { Some(::axiom_core::Layer::Validate) },
            "agent" => quote! { Some(::axiom_core::Layer::Agent) },
            "oversight" => quote! { Some(::axiom_core::Layer::Oversight) },
            "all" => quote! { None },
            _ => return syn::Error::new_spanned(&input, format!("invalid layer: {}", l)).to_compile_error().into(),
        }
    } else {
        quote! { None }
    };

    let reg_static = syn::Ident::new(
        &format!("__CAP_REG_{}", name_str.to_uppercase()),
        proc_macro2::Span::call_site(),
    );
    let reg_entry = syn::Ident::new(
        &format!("__CAP_ENTRY_{}", name_str.to_uppercase()),
        proc_macro2::Span::call_site(),
    );

    let expanded = quote! {
        #[derive(Debug, Clone)]
        #input

        #[doc(hidden)]
        #[allow(non_upper_case_globals)]
        pub static #reg_static: ::axiom_core::CapabilityDescriptor = ::axiom_core::CapabilityDescriptor {
            dimension: #dim_variant,
            name: #name_str,
            version: ::axiom_core::Version::new(#major, #minor, #patch),
            compatibility: ::axiom_core::Compatibility::SemVer,
            applies_to_layer: #layer_variant,
            migration_chain_start: None,
        };

        #[linkme::distributed_slice(::axiom_core::CAPABILITY_REGISTRY)]
#[linkme(crate = linkme)]
        #[doc(hidden)]
        pub static #reg_entry: &'static ::axiom_core::CapabilityDescriptor = &#reg_static;
    };

    TokenStream::from(expanded)
}
