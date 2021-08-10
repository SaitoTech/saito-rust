use proc_macro::{self, TokenStream};
use proc_macro2::TokenTree;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

///
/// A derive macro which implements TryFrom<u8> for an enum.
///
/// usage:
/// ```rust
/// #[derive(TryFromByte)]
/// ```
///
#[proc_macro_derive(TryFromByte)]
pub fn try_from_byte(input: TokenStream) -> TokenStream {
    // parse the code into DeriveInput
    let DeriveInput { ident, data, .. } = parse_macro_input!(input);
    // Get the length of the DataEnums in the Enum(i.e. the number of variants
    // in the enum)
    let len = match data {
        syn::Data::Enum(enum_item) => enum_item.variants.len(),
        _ => panic!("TryFromByte only works on Enums"),
    };
    // use the length to implement TryFrom for the enum
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
///
/// A derive macro which implments Persistable save/load functions.
///
/// optional attribute persist_with_name can be added to specify a
/// method which should be called to get the name of the file
/// where the data should be stored. For example, the method might
/// return a hex-based hash or a string like {timestamp}-{hash} for
/// a block. Should have the signature:
///
/// ```rust
/// pub fn method_name(&self) -> String
/// ```
///
/// usage:
/// ```rust
/// #[derive(Persistable)]
/// #[persist_with_name(get_name)]
/// ```
///
#[proc_macro_derive(Persistable, attributes(persist_with_name, persist_with_data))]
pub fn persistable(input: TokenStream) -> TokenStream {
    let DeriveInput { ident, attrs, .. } = parse_macro_input!(input);
    let mut with_name_method = None;
    // If the struct has an attribute persist_with_name, read it and get the Ident
    // of the name of the method indicated
    for attr in attrs {
        if attr.path.is_ident("persist_with_name") {
            attr.tokens
                .into_iter()
                .for_each(|tokentree| match tokentree {
                    TokenTree::Group(group) => {
                        group
                            .stream()
                            .into_iter()
                            .for_each(|subtokentree| match subtokentree {
                                TokenTree::Ident(ident) => {
                                    with_name_method = Some(ident);
                                }
                                _ => {}
                            });
                    }
                    _ => {}
                });
        }
    }
    // if a persist_with_name attribute was not passed, we will use the
    // ident(name of the struct/type) as the filename
    let struct_name = &ident.to_string().to_lowercase();
    // build a token stream for appending the filename, will be injected
    // into the output TokenStream
    let file_name_append_token_stream;
    if with_name_method.is_some() {
        file_name_append_token_stream = quote! {
            let mut filename: String = String::from("data/");
            filename.push_str(&self.#with_name_method());
        };
    } else {
        file_name_append_token_stream = quote! {
            let mut filename: String = String::from("data/");
            filename.push_str(#struct_name);
        };
    }
    // Build the output TokenStream which implements the save/load functions.
    let output = quote! {
        impl Persistable for #ident {
            fn save(&self) {
                let serialized = serde_json::to_string(&self).unwrap();
                #file_name_append_token_stream
                Storage::write(serialized.as_bytes().to_vec(), &filename);
            }
            fn load(relative_filename: &str) -> Self {
                let mut filename: String = String::from("data/");
                filename.push_str(relative_filename);
                let serialized = Storage::read(&filename).unwrap();
                let out = serde_json::from_str(std::str::from_utf8(&serialized[..]).unwrap()).unwrap();
                out
            }
        }
    };
    output.into()
}
