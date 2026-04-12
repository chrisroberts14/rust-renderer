use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, parse_macro_input};

#[proc_macro_derive(EnumIter)]
pub fn enum_iter_macro(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident;

    let variants = match input.data {
        Data::Enum(data_enum) => data_enum.variants,
        _ => panic!("EnumIter only works on enums"),
    };

    let variant_idents: Vec<_> = variants
        .iter()
        .map(|v| match &v.fields {
            syn::Fields::Unit => &v.ident,
            _ => panic!("Only unit variants supported"),
        })
        .collect();

    let expanded = quote! {
        impl #name {
            pub fn iter() -> impl ::std::iter::Iterator<Item = Self> {
                [
                    #(Self::#variant_idents),*
                ]
                .into_iter()
            }
        }
    };

    TokenStream::from(expanded)
}
