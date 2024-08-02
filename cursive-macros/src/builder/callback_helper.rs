use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;

fn find_arg_name<'a>(signature: &'a syn::Signature, type_name: &'a syn::Ident) -> &'a syn::Ident {
    for arg in &signature.inputs {
        let arg = match arg {
            syn::FnArg::Typed(arg) => arg,
            _ => continue,
        };

        let path = match arg.ty.as_ref() {
            syn::Type::Path(path) => &path.path,
            _ => continue,
        };

        if path.is_ident(type_name) {
            match arg.pat.as_ref() {
                syn::Pat::Ident(ident) => return &ident.ident,
                _ => panic!("Argument is an unsupported pattern."),
            }
        }
    }

    panic!("Could not find argument with type {type_name}.");
}

// Return a stream of all generics involved in this type.
//
// We'll want to wrap the function, and we need to make it just as generic.
fn find_dependent_generics(
    signature: &syn::Signature,
    bound: &syn::TraitBound,
) -> proc_macro2::TokenStream {
    use std::collections::HashMap;

    // Visit all idents in this path.
    fn visit_path_idents(p: &syn::Path, f: &mut impl FnMut(&syn::Ident)) {
        for segment in &p.segments {
            f(&segment.ident);
            match &segment.arguments {
                syn::PathArguments::AngleBracketed(arguments) => {
                    for argument in &arguments.args {
                        match argument {
                            syn::GenericArgument::Type(t) => visit_type_idents(t, f),
                            syn::GenericArgument::AssocType(t) => visit_type_idents(&t.ty, f),
                            syn::GenericArgument::AssocConst(_c) => {
                                // Visit c.expr ?
                            }
                            _ => (),
                        }
                    }
                }
                syn::PathArguments::Parenthesized(arguments) => {
                    for t in &arguments.inputs {
                        visit_type_idents(t, f);
                    }
                    if let syn::ReturnType::Type(_, t) = &arguments.output {
                        visit_type_idents(t, f);
                    }
                }
                _ => (),
            }
        }
    }

    // Visit all idents in this type.
    fn visit_type_idents(t: &syn::Type, f: &mut impl FnMut(&syn::Ident)) {
        match t {
            syn::Type::Paren(t) => visit_type_idents(&t.elem, f),
            syn::Type::Path(p) => {
                if let Some(qs) = &p.qself {
                    visit_type_idents(&qs.ty, f);
                }

                visit_path_idents(&p.path, f)
            }
            syn::Type::Array(a) => visit_type_idents(&a.elem, f),
            syn::Type::Group(g) => visit_type_idents(&g.elem, f),
            syn::Type::Reference(r) => visit_type_idents(&r.elem, f),
            syn::Type::Slice(s) => visit_type_idents(&s.elem, f),
            syn::Type::Tuple(t) => {
                for t in &t.elems {
                    visit_type_idents(t, f);
                }
            }
            _ => (),
        }
    }

    fn check_new_dependent(
        // signature: &syn::Signature,
        relevant: &mut HashMap<syn::Ident, bool>,
        bound: &syn::TraitBound,
    ) {
        let mut new_idents = Vec::new();
        visit_path_idents(&bound.path, &mut |ident| {
            if let Some(r) = relevant.get_mut(ident) {
                if !(*r) {
                    *r = true;
                    new_idents.push(ident.clone());
                    // Find the new bound
                    check_new_dependent(/* signature, */ relevant, bound);
                }
            }
        });
    }

    let mut relevant: HashMap<syn::Ident, bool> = signature
        .generics
        .type_params()
        .map(|t| (t.ident.clone(), false))
        .collect();

    // Maps type ident to Vec<Bounds> that mention this type.
    // So we know if we need the type, we probably need to include the bound?
    let mut bounds = HashMap::new();

    // Look for links in the generics from the function definition.
    for bound in signature
        .generics
        .type_params()
        .flat_map(|t| &t.bounds)
        .filter_map(|bounds| match bounds {
            syn::TypeParamBound::Trait(bound) => Some(bound),
            _ => None,
        })
    {
        // Attach this bound to all relevant types
        visit_path_idents(&bound.path, &mut |ident| {
            // Register this bound as relevant to this event.
            bounds
                .entry(ident.clone())
                .or_insert_with(Vec::new)
                .push(bound);
        });
    }

    // Look for links in the where clause.
    // For example there could be `where T: Into<U>`, in which case if we need T, we need `U`.
    //
    // if let Some(ref where_clause) = signature.generics.where_clause {
    //     for pred in &where_clause.predicates {
    //         match pred {
    //             syn::WherePredicate::Type(t) => for bound in &t.bounds {},
    //             syn::WherePredicate::Lifetime(l) => (),
    //             syn::WherePredicate::Eq(e) => (),
    //         }
    //     }
    // }

    check_new_dependent(/* signature, */ &mut relevant, bound);

    let generics: Vec<_> = signature
        .generics
        .type_params()
        .filter(|t| relevant[&t.ident])
        .collect();

    quote! {
        #(#generics),*
    }
}

/// Returns (Bounds, argument name, Optional<type name>)
///
/// The type name is not available if the arg is impl Fn()
fn find_fn_generic(
    signature: &syn::Signature,
) -> Option<(&syn::TraitBound, &syn::Ident, Option<&syn::Ident>)> {
    // Option A: fn foo<F: Fn()>(f: F)
    for param in signature.generics.type_params() {
        for bound in &param.bounds {
            let bound = match bound {
                syn::TypeParamBound::Trait(bound) => bound,
                _ => continue,
            };

            let segment = match bound.path.segments.iter().last() {
                Some(segment) => segment,
                None => continue,
            };

            if segment.ident == "Fn" || segment.ident == "FnMut" || segment.ident == "FnOnce" {
                let arg_name = find_arg_name(signature, &param.ident);
                // This is it!
                return Some((bound, arg_name, Some(&param.ident)));
            }
        }
    }

    // Option B: fn foo<F>(f: F) where F: Fn()
    for predicate in &signature.generics.where_clause.as_ref()?.predicates {
        let predicate = match predicate {
            syn::WherePredicate::Type(predicate) => predicate,
            _ => continue,
        };

        for bound in &predicate.bounds {
            let bound = match bound {
                syn::TypeParamBound::Trait(bound) => bound,
                _ => continue,
            };

            let segment = match bound.path.segments.iter().last() {
                Some(segment) => segment,
                None => continue,
            };

            if segment.ident == "Fn" || segment.ident == "FnMut" || segment.ident == "FnOnce" {
                // This is it!
                let ident = match &predicate.bounded_ty {
                    syn::Type::Path(path) => path
                        .path
                        .get_ident()
                        .expect("expected single-ident for this type"),
                    _ => panic!("expected generic type for this bound"),
                };

                let arg_name = find_arg_name(signature, ident);

                return Some((bound, arg_name, Some(ident)));
            }
        }
    }

    // Option C: fn foo(f: impl Fn())
    for arg in &signature.inputs {
        let arg = match arg {
            syn::FnArg::Typed(arg) => arg,
            _ => continue,
        };

        let impl_trait = match arg.ty.as_ref() {
            syn::Type::ImplTrait(impl_trait) => impl_trait,
            _ => continue,
        };

        for bound in &impl_trait.bounds {
            let bound = match bound {
                syn::TypeParamBound::Trait(bound) => bound,
                _ => continue,
            };

            let segment = match bound.path.segments.iter().last() {
                Some(segment) => segment,
                None => continue,
            };

            if segment.ident == "Fn" || segment.ident == "FnMut" || segment.ident == "FnOnce" {
                // Found it!
                let arg_name = match arg.pat.as_ref() {
                    syn::Pat::Ident(ident) => &ident.ident,
                    _ => panic!("Argument is an unsupported pattern."),
                };
                return Some((bound, arg_name, None));
            }
        }
    }

    None
}

fn bound_to_dyn(bound: &syn::TraitBound) -> proc_macro2::TokenStream {
    quote!(::std::sync::Arc<dyn #bound + Send + Sync>)
}

fn get_arity(bound: &syn::TraitBound) -> usize {
    let segment = bound.path.segments.iter().last().unwrap();
    let args = match &segment.arguments {
        syn::PathArguments::Parenthesized(args) => args,
        _ => panic!("Expected Fn trait arguments"),
    };
    args.inputs.len()
}

/// Generate two helper functions to help working with callbacks in cursive blueprints.
///
/// # Problem to solve
///
/// When writing cursive blueprints, it is often necessary to load parameters or variables.
///
/// Some of these have simple types like `u64` or `String`, but in some cases we need to load a
/// callback. Most of the time, the existing setter function will take a generic `<F: Fn(...)>`.
///
/// In this case, the blueprint loading the variable and the user storing the variable need
/// to use the exact same type (otherwise, downcasting will not work). This is made complicated by Rust's
/// closures, where each closure is a unique anonymous type: if the user directly stores a closure,
/// there will be no way to identify its exact type to downcast it in the blueprint.
///
/// Instead, both sides (blueprint and user) need to agree to use a fixed type, for example a trait
/// object like `Arc<dyn Fn(&mut Cursive)>` (we use `Arc` rather than `Box` because we want variables
/// to be cloneable).
///
/// It's a bit cumbersome having to write the exact type including the `Arc` whenever we want to
/// store a callback for a blueprint. Similarly, it's a bit annoying when writing the blueprint to make
/// sure the correct `Arc<...>` type is fetched and converted to a type directly usable as callback.
/// Most importantly, it increases the chances of the two sides not using _exactly_ the same type,
/// leading to failures when attempting to load the variable for the blueprint.
///
/// # Solution
///
/// This is where this macro comes into play: from an original function that requires a closure, it
/// generates two helper functions:
/// * A _maker_ function, to be used when storing variables. This function takes a generic type
///   implementing the same `Fn` trait as the desired callback, and returns it wrapped in the
///   correct trait object. It will be named `{name}_cb`, where `{name}` is the name of the
///   original function this macro is attached to.
/// * A _setter_ function, to be used when writing blueprints. This function wraps the original
///   function, but takes a trait-object instead of a generic `Fn` type, and unwraps it
///   internally. It will be named `{name}_with_cb`, where `{name}` is the name of the original
///   function this macro is attached to.
///
/// # Notes
///
/// * The wrapped function doesn't even have to take `self`, it can be a "static" constructor
///   method.
/// * The `maker` function always takes a `Fn`, not a `FnMut` or `FnOnce`. Use the
///   `cursive::immut1!` (and others) macros to wrap a `FnMut` into a `Fn` if you need it.
///
/// # Examples
///
/// ```rust,ignore
/// struct Foo {
///     callback: Box<dyn Fn(&mut Cursive) + Send + Sync>,
/// }
///
/// impl Foo {
///     // This will generate 2 extra functions:
///     // * `new_cb` to wrap a closure into the proper "shareable" type.
///     // * `new_with_cb` that takes the "shareable" type instead of `F`, and internally calls
///     //   `new` itself.
///     #[cursive::callback_helpers]
///     pub fn new<F>(callback: F) -> Self
///     where
///         F: Fn(&mut Cursive) + 'static,
///     {
///         let callback = Box::new(callback);
///         Foo { callback }
///     }
/// }
///
/// cursive::blueprint!(Foo, |config, context| {
///     // In a blueprint, we use `new_with_cb` to resolve the proper callback type.
///     let foo =
///         Foo::new_with_cb(context.resolve(config["callback"])?);
///
///     Ok(foo)
/// });
///
/// // Elsewhere
/// fn foo() {
///     let mut context = cursive::builder::Context::new();
///
///     // When storing the callback, we use `new_cb` to wrap it into a shareable type.
///     context.store("callback", Foo::new_cb(|s| s.quit()));
/// }
/// ```
pub fn callback_helpers(item: TokenStream) -> TokenStream {
    // Read the tokens. Should be a function.
    let input = syn::parse_macro_input!(item as syn::ImplItemFn);

    // TODO: use attrs to customize the setter/maker names
    // * Set the wrapper name
    // * Set the setter name
    // * Specify the generic parameter to wrap.

    // eprintln!("{:#?}", input.sig);

    // The wrapped function should have (at least) one generic type parameter.
    // This type parameter should include a function bound.
    // It could be specified in many ways: impl Fn, <F: Fn>, where F: Fn...
    let (fn_bound, cb_arg_name, type_ident) =
        find_fn_generic(&input.sig).expect("Could not find function-like generic parameter.");

    // Fn-ify the function bound
    let mut fn_bound = fn_bound.clone();
    fn_bound.path.segments.last_mut().unwrap().ident = syn::Ident::new("Fn", Span::call_site());

    // We will deduce a dyn-able type from this bound (a Arc<dyn Fn(Input) -> Output)
    let dyn_type = bound_to_dyn(&fn_bound);

    // set_on_foo | new
    let fn_ident = &input.sig.ident;
    let fn_name = format!("{}", fn_ident);

    // on_foo | new
    let (maker_name, setter_name) = match fn_name.strip_prefix("set_") {
        Some(base) => (format!("{base}_cb"), format!("{fn_name}_cb")),
        None => (format!("{fn_name}_cb"), format!("{fn_name}_with_cb")),
    };

    // We will then append to the function 2 helper functions:
    // * A callback-maker function: takes the same function type, return the dyn-able type.
    // on_foo_cb | new_cb
    let maker_ident = syn::Ident::new(&maker_name, Span::call_site());

    // TODO: There may be extra generics for F, such as a generic argument.
    // So find all the generics that are referenced by F (but not F itself, since we are getting
    // rid of it)
    let maker_generics = find_dependent_generics(&input.sig, &fn_bound);

    // And all bounds that apply to these generics
    // TODO: implement it
    let maker_bounds = quote!(); // find_dependent_bounds(&maker_generics);

    // And keep them in the maker.
    let maker_doc = format!(
        r#"Helper method to store a callback of the correct type for [`Self::{fn_ident}`].

This is mostly useful when using this view in a template."#
    );
    let maker_fn = quote! {
        #[doc = #maker_doc]
        pub fn #maker_ident
            <F: #fn_bound + 'static + Send + Sync, #maker_generics>
            ( #cb_arg_name: F ) -> #dyn_type
            where #maker_bounds
        {
            ::std::sync::Arc::new(#cb_arg_name)
        }
    };

    // eprintln!("{}", maker_fn);

    // * A callback-setter function: takes the dyn-able type (and maybe other vars), and call the

    // set_on_foo_cb | new_with_cb
    let setter_ident = syn::Ident::new(&setter_name, Span::call_site());

    let return_type = &input.sig.output;

    let args_signature: Vec<_> = input
        .sig
        .inputs
        .iter()
        .map(|arg| {
            if let syn::FnArg::Typed(arg) = arg {
                if let syn::Pat::Ident(ident) = arg.pat.as_ref() {
                    if &ident.ident == cb_arg_name {
                        return quote! { #cb_arg_name: #dyn_type };
                    }
                }
            }

            quote! { #arg }
        })
        .collect();

    let args_signature = quote! {
        #(#args_signature),*
    };

    // a,b,c... for as many arguments as the function takes.
    let n_args = get_arity(&fn_bound);

    let cb_args: Vec<_> = (0..n_args).map(|i| quote::format_ident!("a{i}")).collect();
    let cb_args = quote! {
        #(#cb_args),*
    };

    let args_call: Vec<_> = input
        .sig
        .inputs
        .iter()
        .map(|arg| match arg {
            syn::FnArg::Receiver(_) => {
                quote! { self }
            }
            syn::FnArg::Typed(arg) => {
                if let syn::Pat::Ident(ident) = arg.pat.as_ref() {
                    if &ident.ident == cb_arg_name {
                        return quote! {
                            move |#cb_args| { (*#cb_arg_name)(#cb_args) }
                        };
                    }
                }

                let pat = &arg.pat;
                quote! { #pat }
            }
        })
        .collect();
    let args_call = quote! {
        #(#args_call),*
    };

    let generics: Vec<_> = input
        .sig
        .generics
        .params
        .iter()
        .filter(|param| {
            if let syn::GenericParam::Type(type_param) = param {
                Some(&type_param.ident) != type_ident
            } else {
                true
            }
        })
        .collect();

    let generics = quote! {
        < #(#generics),* >
    };

    let where_clause: Vec<_> = input
        .sig
        .generics
        .where_clause
        .as_ref()
        .map(|where_clause| {
            where_clause
                .predicates
                .iter()
                .filter(|predicate| {
                    if let syn::WherePredicate::Type(syn::PredicateType {
                        bounded_ty: syn::Type::Path(path),
                        ..
                    }) = predicate
                    {
                        type_ident.map_or(false, |ident| !path.path.is_ident(ident))
                    } else {
                        false
                    }
                })
                .collect()
        })
        .unwrap_or_else(Vec::new);

    let where_clause = quote! {
        where #(#where_clause),*
    };

    // TODO: omit the Function's trait bound when we forward it
    // TODO: decide:
    // - should we just take a Arc<dyn Fn> instead of impl Fn?
    // - or should we take (config, context) and parse there instead? And maybe do nothing on null?
    let setter_doc = format!(
        r#"Helper method to call [`Self::{fn_ident}`] with a variable from a config.

This is mostly useful when writing a cursive blueprint for this view."#
    );
    let setter_fn = quote! {
        #[doc = #setter_doc]
        pub fn #setter_ident #generics (#args_signature) #return_type #where_clause {

            Self::#fn_ident(#args_call)

        }
    };

    // eprintln!("{}", setter_fn);

    // main wrapped function.
    TokenStream::from(quote! {
        #input

        #maker_fn

        #setter_fn
    })
}
