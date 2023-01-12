use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;

use std::collections::HashSet;

fn parse_arg(arg: &syn::Expr, parameters: &mut HashSet<String>) {
    match arg {
        syn::Expr::Lit(syn::ExprLit { .. }) => {
            // All good
        }
        syn::Expr::Path(syn::ExprPath { path, .. }) => {
            let parameter = path
                .get_ident()
                .expect("Expected simple ident as parameter");
            parameters.insert(parameter.to_string());
        }
        syn::Expr::Reference(r) => {
            parse_arg(&r.expr, parameters);
        }
        _ => (),
    }
}

fn parse_attributes(base: &syn::ExprCall) -> (String, HashSet<String>) {
    let path = match base.func.as_ref() {
        // We need it to be a path
        syn::Expr::Path(syn::ExprPath { path, .. }) => path,
        _ => panic!("Expected method call"),
    };

    if path.segments.len() != 2 {
        panic!("Expected Type::method() call with 2 segments.");
    }

    let name = path.segments[0].ident.to_string();

    let mut parameters = HashSet::new();

    for arg in &base.args {
        parse_arg(arg, &mut parameters);
    }

    (name, parameters)
}

fn is_single_generic<'a>(
    path: &'a syn::Path,
    name: &str,
) -> Option<&'a syn::Type> {
    if path.segments.len() != 1 {
        return None;
    }

    let segment = &path.segments[0];

    if segment.ident != name {
        return None;
    }

    if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
        if args.args.len() != 1 {
            return None;
        }
        let arg = &args.args[0];
        if let syn::GenericArgument::Type(ty) = arg {
            Some(ty)
        } else {
            None
        }
    } else {
        None
    }
}

fn is_vec(path: &syn::Path) -> Option<&syn::Type> {
    is_single_generic(path, "Vec")
}

fn is_option(path: &syn::Path) -> Option<&syn::Type> {
    is_single_generic(path, "Option")
}

fn parse_enum(
    item: &syn::ItemEnum,
    params: &HashSet<String>,
    base: &syn::ExprCall,
) -> proc_macro2::TokenStream {
    // Try to find disjoint set of config targets
    let mut cases = Vec::new();

    for variant in &item.variants {
        // Possible sources:
        // - Null (unit)
        // - String (single tuple)
        // - Number (single tuple with number type?)
        // - Array  (tuple with >1 items or tuple with a single array)
        // - Object (struct)
        let variant_name = variant.ident.to_string();
        match &variant.fields {
            syn::Fields::Unnamed(fields) => {
                if fields.unnamed.len() == 1 {
                    // Direct value?
                    match &fields.unnamed[0].ty {
                        syn::Type::Path(path) => {
                            if path.path.is_ident("String") {
                                // String! With name of variant as ident?
                                // variant.ident
                                // The match case
                                let consumer = parse_struct(
                                    &variant.fields,
                                    params,
                                    &variant_name,
                                    base,
                                );
                                cases.push(quote! {
                                    crate::builder::Config::String(_) => {
                                        #consumer
                                    }
                                });
                            }
                        }
                        _ => unimplemented!("Non-path type in tuple variant"),
                    }
                } else {
                    // Array?
                    unimplemented!("Non-singleton in tuple variant");
                }
            }
            syn::Fields::Named(_) => {
                // An object.
                let consumer =
                    parse_struct(&variant.fields, params, &variant_name, base);
                cases.push(quote! {
                    crate::builder::Config::Object(_) => {
                        #consumer
                    }
                });
            }
            syn::Fields::Unit => {
                // Null?
                cases.push(quote! {
                    crate::builder::Config::Null => {
                        #base
                    }
                });
            }
        }
    }

    cases.push(quote! {
        _ => return Err(crate::builder::Error::invalid_config("Unexpected config", config)),
    });

    quote! {
        match config {
            #(#cases),*
        }
    }
}

fn consumer_for_type(
    ty: &syn::Type,
    field_name: &str,
    field_ident: &syn::Ident,
    base_ident: &syn::Ident,
    context: &ConsumerContext,
) -> proc_macro2::TokenStream {
    match ty {
        syn::Type::Paren(syn::TypeParen { ref elem, .. })
        | syn::Type::Group(syn::TypeGroup { ref elem, .. }) => {
            // Just some recursive boilerplate.
            consumer_for_type(
                &elem,
                field_name,
                field_ident,
                base_ident,
                context,
            )
        }
        syn::Type::Infer(_) => {
            let function_name = format!("set_{field_name}_cb");
            let function = syn::Ident::new(&function_name, Span::call_site());
            quote! {
                #base_ident.#function(#field_ident);
            }
        }
        syn::Type::Path(syn::TypePath { ref path, .. }) => {
            // Case A: ty = Option<T>. Recurse.
            if let Some(ty) = is_option(path) {
                let consumer = consumer_for_type(
                    ty,
                    field_name,
                    field_ident,
                    base_ident,
                    context,
                );
                return quote! {
                    if let Some(#field_ident) = #field_ident {
                        #consumer
                    }
                };
            }

            if let Some(_) = &context.foreach {
                if let Some(ty) = is_vec(path) {
                    let consumer = consumer_for_type(
                        ty,
                        field_name,
                        field_ident,
                        base_ident,
                        context,
                    );
                    return quote! {
                        for #field_ident in #field_ident {
                            #consumer
                        }
                    };
                }
            }
            // Case C: plain old type. Call set_#field_name
            let function_name = if let Some(foreach) = &context.foreach {
                foreach.to_string()
            } else {
                format!("set_{field_name}")
            };
            let function = syn::Ident::new(&function_name, Span::call_site());
            quote! {
                #base_ident.#function(#field_ident);
            }
        }
        // TODO: tuple?
        // TODO: array?
        _ => panic!("unsupported type: `{field_name}`"),
    }
}

struct ConsumerContext {
    foreach: Option<syn::Ident>,
    setter: Option<syn::Ident>,
}

// Returns the quote!d code to build the object using this struct.
fn parse_struct(
    fields: &syn::Fields,
    parameter_names: &HashSet<String>,
    struct_name: &str,
    base: &syn::ExprCall,
) -> proc_macro2::TokenStream {
    // Assert: no generic?

    let fields = match fields {
        syn::Fields::Named(fields) => &fields.named,
        syn::Fields::Unnamed(fields) => &fields.unnamed,
        syn::Fields::Unit => {
            return quote! {
                #base
            };
        } // Nothing to do!
    };

    // We'll build:
    // - A list of parameter loaders
    // - A list of setter loaders
    let mut loaders = Vec::new();
    let mut setters = Vec::new();

    let base_ident = syn::Ident::new("_res", Span::call_site());

    for field in fields {
        // Potential overrides
        let mut context = ConsumerContext {
            foreach: None,
            setter: None,
        };
        for attr in &field.attrs {
            // eprintln!("Found attr: {attr:?}");
            // Found one!
            // Parse `attr.tokens` into... key=value pairs
            if let Ok(syn::Meta::List(meta)) = attr.parse_meta() {
                // eprintln!("Found meta: {meta:?}");
                if !meta.path.is_ident("recipe") {
                    continue;
                }

                for meta in &meta.nested {
                    if let syn::NestedMeta::Meta(syn::Meta::List(meta)) = meta
                    {
                        if meta.path.is_ident("foreach") {
                            if let syn::NestedMeta::Meta(syn::Meta::Path(
                                meta,
                            )) = &meta.nested[0]
                            {
                                context.foreach =
                                    Some(meta.get_ident().unwrap().clone());
                            }
                        }
                    }
                }
            }
        }

        // For each field, derive a few things:
        // - The config name (name inside the config), and the param name.
        let (config_name, param_name) = if let Some(ref ident) = field.ident {
            // We have a name!
            (Some(ident.to_string()), ident.to_string())
        } else {
            // In a tuple struct... either there is just one name (parameter)?
            if parameter_names.len() == 1 {
                (None, parameter_names.iter().next().unwrap().to_string())
            } else {
                // It's a setter. If there's no attribute, try the name of the struct?
                // Ideally, CamelCase to snake_case
                (None, struct_name.to_lowercase())
            }
        };

        // - A way to load this field
        //      Defaults to context.resolve(&config["$field"]);
        // let name_lit = syn::LitStr::new(&name, Span::call_site());
        let ident = syn::Ident::new(&param_name, Span::call_site());
        let ty = &field.ty;

        // This is for object source
        // For direct source, the loader will just be context.resolve(&config)?;

        let loader = if let Some(config_name) = config_name {
            quote! { let mut #ident: #ty = context.resolve(&config[#config_name])?; }
        } else {
            quote! { let mut #ident: #ty = context.resolve(&config)?; }
        };

        // - A way to apply this field
        if parameter_names.contains(&param_name) {
            // Note that for parameters, there is no consumer.
            loaders.push(loader);
        } else {
            // TODO: Look for any attribute on the field to override consumer.
            // Build the consumer based on the type.
            let field_ident = syn::Ident::new(&param_name, Span::call_site());
            let consumer = consumer_for_type(
                &field.ty,
                &param_name,
                &field_ident,
                &base_ident,
                &context,
            );

            setters.push(quote! {
                #loader
                #consumer
            });
        }
    }

    quote! {
        #(#loaders)*

        let mut #base_ident = #base ;

        #(#setters)*

        #base_ident
    }
}

pub fn recipe(attrs: TokenStream, item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as syn::Item);
    let base = syn::parse_macro_input!(attrs as syn::ExprCall);

    // From the attributes we can already build:
    // * A name
    // * A list of parameters
    // * A base function taking the parameters
    let (name, params) = parse_attributes(&base);

    // Then, from the body, we can get:
    // * A list of setter variables
    // * A way to check each setter variable and each parameter.

    let builder = match &input {
        syn::Item::Enum(item) => {
            if !item.generics.params.is_empty() {
                panic!("expected non-generic struct");
            }

            // Option A.1: enums are named after a data type (eg: String, Array, Object)
            // Option A.2: enums are using disjoint data types
            // If we can, build a mapping of config types to consumers
            // if let Some(map) = make_map() {
            //     //
            //     let mut cases = Vec::new();

            //     quote! {
            //         match config {
            //             #(#cases),*
            //         }
            //     }
            // } else {
            //     // Plan B: try enums one by one until one works
            //     unimplemented!();
            // }
            parse_enum(item, &params, &base)
        }
        syn::Item::Struct(item) => {
            if !item.generics.params.is_empty() {
                panic!("expected non-generic struct");
            }

            let struct_name = item.ident.to_string();
            parse_struct(&item.fields, &params, &struct_name, &base)
        }
        _ => panic!("Expected enum or struct"),
    };

    let ident = syn::Ident::new(&name, Span::call_site());
    let result = quote! {
        crate::raw_recipe!(#ident, |config, context| {
            Ok({ #builder })
        });
    };

    // eprintln!("Res: {result}");

    result.into()
}
