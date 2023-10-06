use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, parse_quote, FnArg, Ident, ImplItem, ItemImpl, Pat, Visibility};

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
            //println!("found fn: {:?}", item.sig.ident);
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
            let old_name = &item.sig.ident;
            let mut arg_names = Vec::new();
            for arg in item.sig.inputs.iter().skip(1) {
                if let FnArg::Typed(arg) = arg {
                    if let Pat::Ident(ident) = &*arg.pat {
                        arg_names.push(ident);
                    } else {
                        // TODO: generate new ident
                        panic!("arbitrary patterns are not supported");
                    }
                } else {
                    panic!("unexpected receiver arg");
                }
            }
            new_fn.sig.output = parse_quote! { -> Self };
            *new_fn.sig.inputs.first_mut().unwrap() = parse_quote! { mut self };
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
