use proc_macro::TokenStream;
use quote::{quote, quote_spanned};
use syn::{parse_macro_input, spanned::Spanned, Data, DeriveInput};

#[proc_macro_derive(ControllerStartWindow)]
pub fn derive_controller_as_grid(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let path: syn::Path = syn::parse_quote! {::ui_traits};
    let trait_item: syn::ItemTrait = syn::parse_quote! {
        pub trait ControllerStartWindow {
            fn show_start_window(&mut self, ui: &mut egui::Ui);
        }
    };
    simple_derive(&input, &trait_item, Some(&path)).into()
}

#[proc_macro_derive(ControllerUI)]
pub fn derive_show_controller_ui(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let path: syn::Path = syn::parse_quote! {::ui_traits};
    let trait_item: syn::ItemTrait = syn::parse_quote! {
        pub trait ControllerUI {
            fn show_controller(&mut self, ui: &mut egui::Ui);
        }
    };

    simple_derive(&input, &trait_item, Some(&path)).into()
}

fn simple_derive(
    derive: &DeriveInput,
    trait_item: &syn::ItemTrait,
    trait_path: Option<&syn::Path>,
) -> proc_macro2::TokenStream {
    let trait_name: &syn::Ident = &trait_item.ident;
    let target_name = &derive.ident;

    let function = trait_item
        .items
        .iter()
        .map(|x| match x {
            syn::TraitItem::Fn(m) => Some(m),
            _ => None,
        })
        .flatten();
    let impls = match &derive.data {
        Data::Union(_) => syn::Error::new_spanned(&derive, "not implemented for unions")
            .to_compile_error()
            .into(),
        Data::Enum(data_enum) => simple_derive_enum(data_enum, trait_item, function, trait_path),
        Data::Struct(data_struct) => simple_derive_struct(data_struct, trait_item, function, trait_path),
    };
    let trait_path = trait_path.map(|x| quote!(#x :: ));
    quote! {
        impl #trait_path #trait_name for #target_name {
            #impls
        }
    }
}

fn simple_derive_struct<'a>(
    data_struct: &syn::DataStruct,
    trait_item: &syn::ItemTrait,
    function: impl Iterator<Item = &'a syn::TraitItemFn>,
    trait_path: Option<&syn::Path>,
) -> proc_macro2::TokenStream {

    
    let trait_path = trait_path.map(|x| quote!(#x :: ));

    function
        .map(|function| {
            let func_name = &function.sig.ident;
            let trait_name = &trait_item.ident;

            let mut self_prefix = proc_macro2::TokenStream::new();

            let function_sig = &function.sig;

            if let Some(syn::FnArg::Receiver(r)) = function_sig.inputs.first() {
                if r.reference.is_some() {
                    self_prefix = quote! {& };
                }
                if r.mutability.is_some() {
                    self_prefix.extend(quote! {mut });
                }
            };

            let paras = function_sig
                .inputs
                .iter()
                .map(|x| match x {
                    syn::FnArg::Receiver(_) => None,
                    syn::FnArg::Typed(t) => Some(t),
                })
                .flatten();

            let impl_paras = paras
                .map(|x| {
                    let ident = &x.pat;
                    quote_spanned!(x.span()=> #ident)
                })
                .collect::<Vec<_>>();
            let fields = &data_struct.fields;
            let field_call = fields.iter().enumerate().map(|(index, field)| {
                let span = field.ty.span();
                if let Some(f) = field.ident.as_ref() {
                    quote_spanned! {span=>
                        #trait_path #trait_name::#func_name(#self_prefix self.#f, #(#impl_paras),*);
                    }
                } else {
                    quote_spanned! {span=>
                        #trait_path #trait_name::#func_name(#self_prefix self.#index, #(#impl_paras),*);
                    }
                }
            });

            quote! {
                #function_sig{
                    #(#field_call)*
                }
            }
        })
        .collect()
}

fn simple_derive_enum<'a>(
    data_enum: &syn::DataEnum,
    trait_item: &syn::ItemTrait,
    function: impl Iterator<Item = &'a syn::TraitItemFn>,
    trait_path: Option<&syn::Path>,
) -> proc_macro2::TokenStream {
    function.map(|function| {
    let func_name = &function.sig.ident;
    let trait_name = &trait_item.ident;

    let function_sig = &function.sig;

     
    let trait_path = trait_path.map(|x| quote!(#x :: ));

    let paras = function_sig
        .inputs
        .iter()
        .map(|x| match x {
            syn::FnArg::Receiver(_) => None,
            syn::FnArg::Typed(t) => Some(t),
        })
        .flatten();

    let impl_paras = paras
        .map(|x| {
            let ident = &x.pat;
            quote_spanned!(x.span()=> #ident)
        })
        .collect::<Vec<_>>();

    let variants = &data_enum.variants;
    let variant_call = variants.iter().enumerate().map(|(index,variant)| {
                let variant_name = &variant.ident;
                let fields = &variant.fields;
                match fields{
                    syn::Fields::Unnamed(variant) => {
                        if let &[field] = variant.unnamed.iter().collect::<Vec<_>>().as_slice(){
                            let span = field.ty.span();
                            let bind_ident= syn::Ident::new(&format!("__a{}",index),variant.span());
                            quote_spanned! {span=>
                                Self::#variant_name(#bind_ident) => #trait_path #trait_name::#func_name(#bind_ident, #(#impl_paras),*),
                            }
                        }else{
                            return syn::Error::new_spanned(fields, "derive only implemented for variants with one field")
                                .to_compile_error()
                                .into();
                        }
                    },
                    _ => return syn::Error::new_spanned(fields, "derive only implemented for variants with unnamed fields")
                        .to_compile_error()
                        .into(),
                }
        });

    quote! {
        #function_sig{
            match self {
                #(#variant_call)*
            }
        }
    }}).collect()
}

#[cfg(test)]
mod test;
