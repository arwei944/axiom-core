use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, ItemStruct};

use crate::utils::*;

pub fn impl_derive_signal_payload(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let mut kind = quote! { ::axiom_kernel::signal::SignalKind::Command };
    let mut layer = quote! { ::axiom_kernel::RuntimeTier::Exec };

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

    let _skip_schema = input.attrs.iter().any(|attr| {
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
        quote! { fn trace_id(&self) -> Option<&::axiom_kernel::id::TraceId> { Some(&self.trace_id) } }
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
            fn schema_version(&self) -> ::axiom_kernel::version::SchemaVersion {
                ::axiom_kernel::version::SchemaVersion::new(#ver)
            }
        }
    } else {
        quote! {}
    };

    let expanded = quote! {
        impl #impl_generics ::axiom_kernel::signal::Signal for #name #ty_generics #where_clause {
            fn signal_type(&self) -> &'static str {
                stringify!(#name)
            }
            fn msg_id(&self) -> &::axiom_kernel::id::MsgId {
                &self.msg_id
            }
            fn correlation_id(&self) -> &::axiom_kernel::id::CorrelationId {
                &self.correlation_id
            }
            #trace_id_impl
            fn vector_clock(&self) -> &::axiom_kernel::signal::VectorClock {
                &self.vector_clock
            }
            fn timestamp_ns(&self) -> u64 {
                0
            }
            fn kind(&self) -> ::axiom_kernel::signal::SignalKind {
                #kind
            }
            fn layer(&self) -> ::axiom_kernel::RuntimeTier {
                #layer
            }
            #sender_impl
            #schema_version_impl
            fn as_any(&self) -> &dyn std::any::Any {
                self
            }
            fn clone_signal(&self) -> Box<dyn ::axiom_kernel::signal::Signal> {
                Box::new(Clone::clone(self))
            }
            fn validate(&self) -> ::axiom_kernel::axiom::ValidationResult {
                ::axiom_kernel::axiom::ValidationResult::ok()
            }
            fn serialize_to_json(&self) -> ::axiom_kernel::KernelResult<serde_json::Value> {
                serde_json::to_value(self)
                    .map_err(|e| ::axiom_kernel::KernelError::SerializationError(e.to_string()))
            }
        }
    };

    TokenStream::from(expanded)
}

pub fn impl_signal(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let mut kind = quote! { ::axiom_kernel::signal::SignalKind::Command };
    let mut layer = quote! { ::axiom_kernel::RuntimeTier::Exec };
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
                    match parse_signal_kind(&syn::LitStr::new(&s, proc_macro2::Span::call_site())) {
                        Ok(ts) => kind = ts,
                        Err(e) => return e.to_compile_error().into(),
                    }
                }
            } else if ident == "layer" {
                let _eq = iter.next();
                if let Some(proc_macro2::TokenTree::Literal(lit)) = iter.next() {
                    let s = lit.to_string();
                    let s = s.trim_matches('"').to_string();
                    match parse_layer_variant(&syn::LitStr::new(&s, proc_macro2::Span::call_site()))
                    {
                        Ok(ts) => layer = ts,
                        Err(e) => return e.to_compile_error().into(),
                    }
                }
            } else if ident == "trace" {
                has_trace = true;
            } else if ident == "sender" {
                has_sender = true;
            }
        }
    }

    let required_fields = quote! {
        pub msg_id: ::axiom_kernel::id::MsgId,
        pub correlation_id: ::axiom_kernel::id::CorrelationId,
        pub vector_clock: ::axiom_kernel::signal::VectorClock,
    };

    let optional_fields = if has_trace || has_sender {
        let trace_field = if has_trace {
            quote! { pub trace_id: Option<::axiom_kernel::id::TraceId>, }
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
                    !matches!(
                        ident.to_string().as_str(),
                        "msg_id" | "correlation_id" | "vector_clock" | "trace_id" | "sender"
                    )
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
        quote! { fn trace_id(&self) -> Option<&::axiom_kernel::id::TraceId> { self.trace_id.as_ref() } }
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
            pub fn with_trace_id(mut self, trace: ::axiom_kernel::id::TraceId) -> Self {
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
                    !matches!(
                        ident.to_string().as_str(),
                        "msg_id" | "correlation_id" | "vector_clock" | "trace_id" | "sender"
                    )
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
                    !matches!(
                        ident.to_string().as_str(),
                        "msg_id" | "correlation_id" | "vector_clock" | "trace_id" | "sender"
                    )
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
            pub fn new(msg_id: ::axiom_kernel::id::MsgId, correlation_id: ::axiom_kernel::id::CorrelationId #data_field_params) -> Self {
                Self {
                    msg_id,
                    correlation_id,
                    vector_clock: ::axiom_kernel::signal::VectorClock::new(),
                    #trace_field_init
                    #sender_field_init
                    #data_field_inits
                }
            }

            pub fn with_vector_clock(mut self, vc: ::axiom_kernel::signal::VectorClock) -> Self {
                self.vector_clock = vc;
                self
            }

            #trace_setter
            #sender_setter
        }

        impl #impl_generics ::axiom_kernel::signal::Signal for #name #ty_generics #where_clause {
            fn signal_type(&self) -> &'static str {
                stringify!(#name)
            }
            fn msg_id(&self) -> &::axiom_kernel::id::MsgId {
                &self.msg_id
            }
            fn correlation_id(&self) -> &::axiom_kernel::id::CorrelationId {
                &self.correlation_id
            }
            #trace_id_impl
            fn vector_clock(&self) -> &::axiom_kernel::signal::VectorClock {
                &self.vector_clock
            }
            fn timestamp_ns(&self) -> u64 {
                0
            }
            fn kind(&self) -> ::axiom_kernel::signal::SignalKind {
                #kind
            }
            fn layer(&self) -> ::axiom_kernel::RuntimeTier {
                #layer
            }
            #sender_impl
            fn as_any(&self) -> &dyn std::any::Any {
                self
            }
            fn clone_signal(&self) -> Box<dyn ::axiom_kernel::signal::Signal> {
                Box::new(self.clone())
            }
            fn validate(&self) -> ::axiom_kernel::axiom::ValidationResult {
                ::axiom_kernel::axiom::ValidationResult::ok()
            }
            fn serialize_to_json(&self) -> ::axiom_kernel::KernelResult<serde_json::Value> {
                serde_json::to_value(self)
                    .map_err(|e| ::axiom_kernel::KernelError::SerializationError(e.to_string()))
            }
        }
    };

    TokenStream::from(expanded)
}
