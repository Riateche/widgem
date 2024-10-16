use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input, parse_quote, spanned::Spanned, FnArg, Ident, ImplItem, ItemImpl, Pat,
    ReturnType, Visibility,
};

#[proc_macro_attribute]
pub fn impl_with(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut impl1 = parse_macro_input!(item as ItemImpl);
    let mut new_fns = Vec::new();
    for item in &impl1.items {
        if let ImplItem::Fn(item) = item {
            if !matches!(item.vis, Visibility::Public(_)) {
                continue;
            }
            let Some(first_arg) = item.sig.inputs.iter().next() else {
                continue;
            };
            if !is_ref_mut_self(first_arg) {
                continue;
            }
            if !matches!(item.sig.output, ReturnType::Default) {
                continue;
            }
            let mut new_fn = item.clone();
            let mut stripped_name = item.sig.ident.to_string();
            if let Some(x) = stripped_name.strip_prefix("set_") {
                stripped_name = x.into();
            }
            if let Some(x) = stripped_name.strip_prefix("add_") {
                stripped_name = x.into();
            }
            new_fn.sig.ident =
                Ident::new(&format!("with_{}", stripped_name), item.sig.ident.span());
            let mut new_inputs = Vec::new();
            let old_name = &item.sig.ident;
            let mut arg_names = Vec::new();
            for (index, arg) in item.sig.inputs.iter().enumerate() {
                match arg {
                    FnArg::Typed(arg) => {
                        let ty = &arg.ty;
                        let ident = if let Pat::Ident(ident) = &*arg.pat {
                            ident.ident.clone()
                        } else {
                            Ident::new(&format!("arg{index}"), arg.span())
                        };
                        new_inputs.push(quote! { #ident: #ty });
                        arg_names.push(ident);
                    }
                    FnArg::Receiver(_) => {
                        new_inputs.push(quote! { mut self });
                    }
                }
            }
            new_fn.sig.output = parse_quote! { -> Self };
            new_fn.sig.inputs = parse_quote! { #(#new_inputs),* };
            // *new_fn.sig.inputs.first_mut().unwrap() = parse_quote! { mut self };
            new_fn.block = parse_quote! { {
                self.#old_name(#(#arg_names,)*);
                self
            } };
            new_fns.push(ImplItem::Fn(new_fn));
        }
    }
    impl1.items.extend(new_fns);
    quote! { #impl1 }.into()
}

fn is_ref_mut_self(arg: &FnArg) -> bool {
    let FnArg::Receiver(arg) = arg else {
        return false;
    };
    arg.reference.is_some() && arg.mutability.is_some()
}
