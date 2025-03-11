use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    FnArg, ImplItem, ImplItemFn, ItemEnum, ItemImpl, ItemStruct, ReturnType, Signature,
    parse_macro_input,
};

/// Generate the necessary code to deserialize elements in a streamed JSON
/// that are arrays of objects into their respective types with serde support.
#[proc_macro_attribute]
pub fn streamed_json(attr: TokenStream, input: TokenStream) -> TokenStream {
    let input_enum = parse_macro_input!(input as ItemEnum);

    // Is lowerCamelCase attr set?  
    let lower_camel_case = if attr.to_string() == "lowerCamelCase" {
        true
    } else {
        false
    };
    let enum_name = &input_enum.ident;

    let match_key_to_field = input_enum.variants.iter().map(|field| {
        let field_name = &field.ident;
        let field_name_str = field_name.to_string();

        // Convert field name to lowerCamelCase if needed
        let field_name_str = if lower_camel_case {
            let mut chars = field_name_str.chars();
            let first_char = chars.next().unwrap().to_lowercase().next().unwrap();
            let rest: String = chars.collect();
            format!("{}{}", first_char, rest)
        } else {
            field_name_str
        };
        
        // Ensure there is only one field type
        if field.fields.len() != 1 {
            panic!("Only one field type is allowed");
        }

        let field_type = &field.fields.iter().next().unwrap().ty;

        quote! {
            Some(#field_name_str) => {
                Ok(#enum_name::#field_name(
                    #field_type::deserialize_single(reader)?
                ))
            }
        }
    });

    let expanded = quote! {
        #input_enum

        impl streamed_json::StreamedJsonDeserializable for #enum_name {
            fn deserialize_for_field<R: std::io::Read>(
                reader: R,
                field_name: Option<&str>,
            ) -> std::io::Result<Self>
            where
                Self: Sized,
            {
                use streamed_json::ReaderDeserializableExt;

                match field_name {
                    #(#match_key_to_field)*
                    _ => Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "Invalid field name",
                    )),
                }
            }
        }
    };

    TokenStream::from(expanded)
}