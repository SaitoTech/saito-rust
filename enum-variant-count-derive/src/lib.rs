use proc_macro::{self, TokenStream};
use quote::quote;
use syn::{parse_macro_input, DeriveInput};


// This also allows some_u8.try_into()
#[proc_macro_derive(TryFromByte)]
pub fn try_from_byte(input: TokenStream) -> TokenStream {
    let DeriveInput { ident, data, .. } = parse_macro_input!(input);
    let len = match data {
        syn::Data::Enum(enum_item) => enum_item.variants.len(),
        _ => panic!("TryFromByte only works on Enums"),
    };
    
    let output = quote! {
        impl TryFrom<u8> for #ident {
            type Error = &'static str;
            fn try_from(x: u8) -> Result<Self, Self::Error> {
                let right_size = x >= 0 && x <= (#len as u8);
                match right_size {
                    true => Ok(unsafe { std::mem::transmute(x as u8) }),
                    _ => Err("invalid #ident Value"),
                }
            }
        }
    };
    output.into()
}