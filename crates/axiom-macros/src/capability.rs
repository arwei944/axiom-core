use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, ItemStruct};

pub fn impl_capability(attr: TokenStream, item: TokenStream) -> TokenStream {
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
        _ => {
            return syn::Error::new_spanned(&input, format!("invalid dimension: {}", dim_str))
                .to_compile_error()
                .into()
        }
    };

    let version_str = version.clone().unwrap_or_else(|| "1.0.0".to_string());
    let ver_parts: Vec<&str> = version_str.split('.').collect();
    if ver_parts.len() != 3 {
        return syn::Error::new_spanned(&input, "version must be in format X.Y.Z")
            .to_compile_error()
            .into();
    }

    let major: u16 = match ver_parts[0].parse() {
        Ok(v) => v,
        Err(_) => {
            return syn::Error::new_spanned(&input, "invalid version major")
                .to_compile_error()
                .into()
        }
    };
    let minor: u16 = match ver_parts[1].parse() {
        Ok(v) => v,
        Err(_) => {
            return syn::Error::new_spanned(&input, "invalid version minor")
                .to_compile_error()
                .into()
        }
    };
    let patch: u16 = match ver_parts[2].parse() {
        Ok(v) => v,
        Err(_) => {
            return syn::Error::new_spanned(&input, "invalid version patch")
                .to_compile_error()
                .into()
        }
    };

    let layer_variant = if let Some(l) = layer {
        match l.as_str() {
            "exec" => quote! { Some(::axiom_core::Layer::Exec) },
            "validate" => quote! { Some(::axiom_core::Layer::Validate) },
            "agent" => quote! { Some(::axiom_core::Layer::Agent) },
            "oversight" => quote! { Some(::axiom_core::Layer::Oversight) },
            "all" => quote! { None },
            _ => {
                return syn::Error::new_spanned(&input, format!("invalid layer: {}", l))
                    .to_compile_error()
                    .into()
            }
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
