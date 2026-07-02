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

/// Derive macro for Signal types. Auto-generates the Signal trait implementation.
///
/// # Attributes
/// - `#[signal(kind = "command", layer = "exec")]`: required on the struct
/// - `#[schema(skip)]`: skip auto-generating a default `impl Schema` (use when you
///   provide your own `impl Schema for MySignal`)
///
/// # Required fields
/// - `msg_id: MsgId`
/// - `correlation_id: CorrelationId`
/// - `vector_clock: VectorClock`
///
/// # Optional fields
/// - `trace_id: Option<TraceId>` - if present, trace_id() returns Some(&self.trace_id)
/// - `timestamp_ns: u64` - if present, timestamp_ns() returns self.timestamp_ns; otherwise uses now_ns()
/// - `sender: Option<String>` - if present, sender() returns self.sender.as_deref()
///
/// # Companion attributes
/// - `#[schema_version(N)]` on the struct sets schema_version() to return SchemaVersion::new(N)
///
/// # Schema validation
/// The macro generates `fn validate(&self) -> ValidationResult` that calls
/// `<Self as Schema>::validate(self)`. By default, a no-op `impl Schema` is
/// generated. Add `#[schema(skip)]` to suppress the default and provide your own.
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

    // Parse #[schema(skip)] to suppress default impl Schema
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

/// Attribute macro for Cell implementations. Registers the cell and enforces layer marker.
///
/// # Usage
/// ```ignore
/// #[axiom_macros::cell(layer = "exec")]
/// impl Cell for MyCell { ... }
/// ```
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

    let expanded = quote! {
        #input

        #marker_impl
        #layer_of_impl
    };

    TokenStream::from(expanded)
}

/// Attribute macro for Axiom implementations. Registers the axiom in the distributed registry
/// and automatically implements DynAxiom for runtime dispatch.
///
/// Apply this to a unit struct that implements the Axiom trait.
///
/// # Usage
/// ```ignore
/// #[axiom]
/// #[derive(Debug, Default)]
/// struct NoNegativeAmount;
/// impl Axiom for NoNegativeAmount { ... }
/// ```
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

        #[::axiom_core::linkme::distributed_slice(::axiom_core::registry::AXIOM_REGISTRY)]
        #[linkme(crate = ::axiom_core::linkme)]
        #[doc(hidden)]
        pub static #reg_fn: &'static dyn ::axiom_core::axiom::DynAxiom = &#reg_static;
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
/// Place on `impl Migration for Type` blocks. The macro fills in source_version()
/// and target_version() automatically, and registers the migration in the
/// distributed registry. The user only needs to implement `migrate()`.
///
/// # Usage
/// ```ignore
/// #[axiom_macros::migration(from = 1)]
/// impl Migration for MigrateV1toV2 {
///     fn migrate(&self, input: Value) -> Result<Value> { ... }
/// }
/// ```
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

        #[::axiom_core::linkme::distributed_slice(::axiom_core::registry::MIGRATION_REGISTRY)]
        #[linkme(crate = ::axiom_core::linkme)]
        static #reg_static: fn() -> (u16, u16, &'static str, &'static str) = #reg_fn;
    };

    TokenStream::from(expanded)
}
