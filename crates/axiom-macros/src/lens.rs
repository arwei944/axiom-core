use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, ItemStruct};

pub fn impl_lens(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let name = &input.ident;
    let name_str = name.to_string();

    let mut lens_id = None;
    let mut _aggregate = None;
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
                        _aggregate = Some(s);
                    }
                }
                "depends_on" => {
                    let _eq = iter.next();
                    if let Some(proc_macro2::TokenTree::Punct(p)) = iter.next() {
                        if p.as_char() == '[' {
                            let mut deps = Vec::new();
                            for tt2 in iter.by_ref() {
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

    let _id_str = lens_id.unwrap_or_else(|| {
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
                quote! { ::axiom_kernel::id::LensId::new(#d) }
            })
            .collect();
        let len = depends_on.len();
        quote! {
            static DEPS: [::axiom_kernel::id::LensId; #len] = [
                #(#dep_lens_ids),*
            ];
        }
    } else {
        quote! {}
    };

    let _depends_on_impl = if !depends_on.is_empty() {
        quote! {
            fn depends_on(&self) -> &[::axiom_kernel::id::LensId] {
                #deps_static
                &DEPS
            }
        }
    } else {
        quote! {}
    };

    let _cache_key_impl = if cache {
        quote! {
            fn cache_key(&self, input: &Self::Input) -> Option<String> {
                serde_json::to_string(input).ok()
            }
        }
    } else {
        quote! {}
    };

    let _reg_static = syn::Ident::new(
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
            pub fn #reg_entry() -> &'static dyn ::axiom_kernel::axiom::DynLens {
                use std::sync::OnceLock;
                static INSTANCE: OnceLock<#name> = OnceLock::new();
                let instance = INSTANCE.get_or_init(|| #name::default());
                static REF: OnceLock<&'static dyn ::axiom_kernel::axiom::DynLens> = OnceLock::new();
                *REF.get_or_init(|| {
                    let boxed = Box::new(instance.clone());
                    Box::leak(boxed) as &dyn ::axiom_kernel::axiom::DynLens
                })
            }

            #[linkme::distributed_slice(::axiom_kernel::registry::LENS_REGISTRY)]
    #[linkme(crate = linkme)]
            #[doc(hidden)]
            pub static #reg_entry_fn: fn() -> &'static dyn ::axiom_kernel::axiom::DynLens = #reg_entry;

            #[doc(hidden)]
            #[allow(non_upper_case_globals)]
            pub static #cap_reg_static: ::axiom_kernel::registry::CapabilityDescriptor = ::axiom_kernel::registry::CapabilityDescriptor {
                dimension: ::axiom_kernel::registry::CapabilityDimension::Schema,
                name: #name_str,
                version: ::axiom_kernel::version::Version::new(#major, #minor, #patch),
                compatibility: ::axiom_kernel::version::Compatibility::Exact,
                applies_to_layer: None,
                migration_chain_start: None,
            };

            #[linkme::distributed_slice(::axiom_kernel::registry::CAPABILITY_REGISTRY)]
    #[linkme(crate = linkme)]
            #[doc(hidden)]
            pub static #cap_reg_entry: &'static ::axiom_kernel::registry::CapabilityDescriptor = &#cap_reg_static;
        };

    TokenStream::from(expanded)
}