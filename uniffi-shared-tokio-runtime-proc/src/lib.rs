use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, 
    ItemImpl, 
    ImplItem, 
    ImplItemFn, 
    Signature, 
    FnArg, 
    ReturnType,
};

/// Proc macro to generate blocking and async impl blocks with UniFFI export
#[proc_macro_attribute]
pub fn uniffi_async_export(_attr: TokenStream, input: TokenStream) -> TokenStream {
    println!("attr: \"{_attr}\"");
    println!("item: \"{input}\"");

    let i_clone = input.clone();
    let input_impl = parse_macro_input!(input as ItemImpl);
    
    // Extract the type name (struct) the impl is for
    let struct_name = &input_impl.self_ty;
    
    // Collect async methods with blocking counterparts
    let async_methods: Vec<&ImplItemFn> = input_impl.items
        .iter()
        .filter_map(|item| {
            if let syn::ImplItem::Fn(method) = item {
                if method.sig.asyncness.is_some() {
                    Some(method)
                } else {
                    panic!("Only async methods are supported")
                }
            } else {
                panic!("Only methods are supported")
            }
        })
        .collect();
    
    // Generate blocking method implementations
    let blocking_methods = async_methods.iter().map(|async_method| {
        let async_sig = &async_method.sig;
        let method_name = &async_sig.ident;
        let blocking_method_name = format_ident!("{}_blocking", method_name);
        let blocking_method_name_str = format!("{}_blocking", method_name);

        // Extract input types and names
        let input_args: Vec<_> = async_sig.inputs.iter()
            .filter_map(|arg| {
                match arg {
                    FnArg::Typed(pat_type) => {
                        let pat = &pat_type.pat;
                        let ty = &pat_type.ty;
                        Some((pat, ty))
                    },
                    _ => None,
                    // FnArg::Receiver(receiver) => None,
                }
            })
            .collect();
        
        // Prepare input names and types for blocking method signature
        let input_pats: Vec<_> = input_args.iter().map(|(pat, _)| pat).collect();
        let input_tys: Vec<_> = input_args.iter().map(|(_, ty)| ty).collect();
        
        // Extract return type
        let return_type = match &async_sig.output {
            ReturnType::Type(_, ty) => ty,
            _ => panic!("Async method must have a return type")
        };
        
        quote! {
            #[uniffi::method(name = #blocking_method_name_str)]
            pub fn #blocking_method_name(
                &self, 
                #(#input_pats: #input_tys),*
            ) -> #return_type {
                let rt = self.tokio_runtime.lock().unwrap();
                let rt = rt.as_ref().unwrap();
                rt.block_on(self.#method_name(#(#input_pats),*))
            }
        }
    });
    
    // Generate the two impl blocks
    let expanded = quote! {
        // Original async impl block
        impl #struct_name {
            #(#async_methods)*
        }

        
        // // Blocking methods impl block with UniFFI export
        #[uniffi::export]
        impl #struct_name {
            #(#blocking_methods)*
        }
    };
    
    TokenStream::from(expanded)
}