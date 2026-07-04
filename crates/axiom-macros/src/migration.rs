use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, ItemImpl};

pub fn impl_migration(attr: TokenStream, item: TokenStream) -> TokenStream {
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

    input.items.insert(
        0,
        syn::parse2(source_version).expect("valid migration source version"),
    ); // foxguard: ignore[rs/no-unwrap-in-lib]
    input.items.insert(
        1,
        syn::parse2(target_version).expect("valid migration target version"),
    ); // foxguard: ignore[rs/no-unwrap-in-lib]

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
