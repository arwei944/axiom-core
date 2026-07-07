use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, ItemStruct};

pub fn impl_tool(attr: TokenStream, item: TokenStream) -> TokenStream {
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
            let _ = ::axiom_kernel::registry::WITNESS_REGISTRY.record(::axiom_kernel::witness::Witness {
                witness_id: ::axiom_kernel::id::WitnessId::new(format!("tool-wit-{}", ::axiom_kernel::clock::global_clock().now_ns())),
                schema_version: <::axiom_kernel::version::WitnessSchema as ::axiom_kernel::version::Versioned>::schema_version(),
                cell_id: "tool-executor".to_string(),
                correlation_id: ::axiom_kernel::id::CorrelationId::new("auto"),
                trace_id: None,
                triggering_msg_id: None,
                vector_clock: ::axiom_kernel::signal::VectorClock::new(),
                timestamp_ns: ::axiom_kernel::clock::global_clock().now_ns(),
                prev_hash: None,
                state_before_hash: None,
                state_after_hash: None,
                hash: ::axiom_kernel::witness::WitnessHash::zero(),
                summary: format!("tool {} executed", #name_str),
                outcome: ::axiom_kernel::witness::TransitionOutcome::Success,
                metrics: ::axiom_kernel::witness::WitnessMetrics::default(),
                version_info: ::axiom_kernel::version::VersionInfo::current(),
                signal_fingerprint: [0u8; 32],
                payload_size_bytes: 0,
                kind: ::axiom_kernel::witness::WitnessKind::ToolInvocation,
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

        impl ::axiom_kernel::tool::Tool for #name {
            fn id(&self) -> &'static str {
                #name_str
            }
            fn invoke(&self, args: Vec<u8>) -> ::axiom_kernel::KernelResult<Vec<u8>> {
                let rt = tokio::runtime::Runtime::new().map_err(|e| ::axiom_kernel::KernelError::InternalError(e.to_string()))?;
                let parameters: serde_json::Value = serde_json::from_slice(&args).map_err(|e| ::axiom_kernel::KernelError::SerializationError(e.to_string()))?;
                let result = rt.block_on(self.execute_inner(&parameters)).map_err(|e| ::axiom_kernel::KernelError::InternalError(e.to_string()))?;
                serde_json::to_vec(&result).map_err(|e| ::axiom_kernel::KernelError::SerializationError(e.to_string()))
            }
        }

        impl #name {
            pub async fn execute_inner(&self, parameters: &serde_json::Value) -> Result<serde_json::Value, ::axiom_tool::ToolError> {
                Ok(serde_json::json!({ "result": "not implemented" }))
            }
        }
    };

    TokenStream::from(expanded)
}
