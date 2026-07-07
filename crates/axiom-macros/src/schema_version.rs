use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, LitInt};

pub fn impl_schema_version(attr: TokenStream, item: TokenStream) -> TokenStream {
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

        impl #impl_generics ::axiom_kernel::Versioned for #name #ty_generics #where_clause {
            fn schema_version() -> ::axiom_kernel::version::SchemaVersion {
                ::axiom_kernel::version::SchemaVersion::new(#version_val)
            }
        }
    };

    TokenStream::from(expanded)
}
