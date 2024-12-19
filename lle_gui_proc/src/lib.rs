use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput};

#[proc_macro_derive(ControllerAsGrid)]
pub fn derive_controller_as_grid(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident.clone();

    match &input.data {
        Data::Enum(_) => syn::Error::new_spanned(input, "only works with structs")
            .to_compile_error()
            .into(),
        Data::Union(_) => syn::Error::new_spanned(input, "only works with structs")
            .to_compile_error()
            .into(),
        Data::Struct(data_struct) => {
            let fields = &data_struct.fields;
            let show_as_drag_value_calls = fields.iter().enumerate().map(|(index, field)| {
                if let Some(f) = field.ident.as_ref() {
                    quote! {
                        self.#f.show(ui);
                        ui.end_row();
                    }
                } else {
                    quote! {
                        self.#index.show(ui);
                        ui.end_row();
                    }
                }
            });

            let expanded = quote! {
                impl crate::config::ControllerAsGrid for #name {
                    fn show(&mut self, ui: &mut egui::Ui) {
                        #(#show_as_drag_value_calls)*
                    }
                }
            };

            TokenStream::from(expanded)
        }
    }
}
