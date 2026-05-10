use {
    proc_macro::TokenStream,
    quote::quote,
    syn::{
        parse_macro_input, parse_quote, spanned::Spanned, FnArg, Ident, ImplItem, ItemFn, ItemImpl,
        ItemStruct, Pat, ReturnType, Visibility,
    },
};

#[proc_macro_derive(AttributeSetters, attributes(widgem_attr))]
pub fn attribute_setters(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemStruct);
    try_attribute_setters(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn try_attribute_setters(mut input: ItemStruct) -> syn::Result<proc_macro2::TokenStream> {
    let ident = &input.ident;

    let fields = match &mut input.fields {
        syn::Fields::Named(fields) => fields,
        syn::Fields::Unnamed(fields) => {
            return Err(syn::Error::new(
                fields.span(),
                "unnamed fields are unsupported",
            ))
        }
        syn::Fields::Unit => {
            return Err(syn::Error::new(input.span(), "unit struct is unsupported"));
        }
    };

    let mut constructor_args = Vec::new();
    let mut constructor_initializers = Vec::new();
    let mut setters = Vec::new();

    for field in fields.named.iter_mut() {
        let mut is_constructor_arg = false;
        let mut default_value = None;
        for attr in &field.attrs {
            if attr.path().is_ident("widgem_attr") {
                // Handles e.g. #[my_attr(rename = "foo", skip)]
                attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("constructor") {
                        is_constructor_arg = true;
                        // let value = meta.value()?;
                        // let lit: syn::LitStr = value.parse()?;
                        // let rename = lit.value();
                    } else if meta.path.is_ident("default") {
                        let value = meta.value()?;
                        default_value = Some(value.parse::<syn::Expr>()?);
                    } else {
                        return Err(meta.error("unknown attribute option"));
                    }
                    Ok(())
                })?;
            }
        }
        let ident = &field.ident;
        let ty = &field.ty;
        if is_constructor_arg {
            constructor_args.push(quote! {
                #ident: #ty,
            });
            constructor_initializers.push(quote! { #ident, });
        } else {
            let default_value = if let Some(default_value) = default_value {
                quote! { #default_value }
            } else {
                quote! { std::default::Default::default() }
            };
            constructor_initializers.push(quote! { #ident: #default_value, });
            setters.push(quote! {
                pub fn #ident(mut self, #ident: #ty) -> Self {
                    self.#ident = #ident;
                    self
                }
            });
        }
    }

    Ok(quote! {
        impl #ident {
            pub fn new(#(#constructor_args)*) -> Self {
                Self { #(#constructor_initializers)* }
            }

            #(#setters)*
        }
    })
}

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

#[proc_macro_attribute]
pub fn test(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let func = parse_macro_input!(item as ItemFn);
    let ident = &func.sig.ident;
    let ident_str = ident.to_string();
    let register_ident = Ident::new(&format!("__register_{}", ident), ident.span());

    let q = quote! {
        #[::widgem_tester::__ctor::ctor(crate_path = ::widgem_tester::__ctor)]
        fn #register_ident() {
            ::widgem_tester::add_test(
                ::widgem_tester::test_name_from_module_path(module_path!(), #ident_str),
                #ident,
            );
        }

        #func
    };
    q.into()
}
