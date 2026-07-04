use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemStruct};

pub fn impl_axiom(_attr: TokenStream, item: TokenStream) -> TokenStream {
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
