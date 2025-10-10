use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;

use std::collections::HashSet;

fn is_single_generic<'a>(path: &'a syn::Path, name: &str) -> Option<&'a syn::Type> {
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

// fn is_vec(path: &syn::Path) -> Option<&syn::Type> {
//     // TODO: handle std::vec::Vec?
//     is_single_generic(path, "Vec")
// }

fn is_option(path: &syn::Path) -> Option<&syn::Type> {
    // TODO: handle std::option::Option?
    is_single_generic(path, "Option")
}

fn is_option_type(ty: &syn::Type) -> Option<&syn::Type> {
    match ty {
        syn::Type::Path(syn::TypePath { ref path, .. }) => is_option(path),
        _ => None,
    }
}

fn looks_inferred(ty: &syn::Type) -> bool {
    match ty {
        syn::Type::Infer(_) => true,
        syn::Type::Path(syn::TypePath { path, .. }) => {
            if let Some(ty) = is_option(path) {
                looks_inferred(ty)
            } else {
                false
            }
        }
        _ => false,
    }
}

fn parse_enum(
    item: &syn::ItemEnum,
    params: &HashSet<String>,
    base: &syn::Expr,
    root: &proc_macro2::TokenStream,
) -> syn::parse::Result<proc_macro2::TokenStream> {
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
                    // String! With name of variant as ident?
                    // variant.ident
                    // The match case
                    let consumer =
                        parse_struct(&variant.fields, params, &variant_name, base, root)?;

                    cases.push(quote! {
                        match || -> Result<_, crate::builder::Error> {
                            Ok({#consumer})
                        }() {
                            Ok(res) => return Ok(res),
                            Err(err) => errors.push(err),
                        }
                    });
                } else {
                    // Array?
                    unimplemented!("Non-singleton in tuple variant");
                }
            }
            syn::Fields::Named(_) => {
                // An object.
                let consumer = parse_struct(&variant.fields, params, &variant_name, base, root)?;
                cases.push(quote! {
                    match || -> Result<_, crate::builder::Error> {
                        Ok({#consumer})
                    }() {
                        Ok(res) => return Ok(res),
                        Err(err) => errors.push(err),
                    }
                });
            }
            syn::Fields::Unit => {
                // Null?
                cases.push(quote! {
                    if config.is_null() {
                        return Ok({#base});
                    } else {

                    }
                });
            }
        }
    }

    Ok(quote! {
        || -> Result<_, crate::builder::Error> {
            let mut errors = Vec::new();
            #(#cases)*
            Err(#root::builder::Error::AllVariantsFailed {
                config: config.clone(),
                errors
            })
        }()?
    })
}

// #[blueprint(
//      config = false,
//      setter = set_enabled,
//      config_name = "foo",
//      foreach = add_stuff,
//      callback = true,
//      default = Foo::default(),
// )]
struct VariableSpecs {
    no_config: bool,
    setter: Option<syn::Ident>,
    foreach: Option<syn::Ident>,
    callback: Option<bool>,
    config_name: Option<String>,
    default: Option<syn::Expr>,
}

struct Variable {
    // How to load the variable from a config.
    loader: Loader,

    // How to call the variable between loading and consuming.
    // Mostly irrelevant, except for constructor.
    ident: syn::Ident,

    // How to consume the data (set some fields?)
    consumer: Consumer,
}

struct Loader {
    // When `true`, load it through a `cursive::builder::NoConfig`.
    no_config: bool,

    // How the value is called in the config.
    config_name: Option<String>,

    // Type we want to load into the ident.
    ty: syn::Type,

    // Default constructor for the value.
    default: Option<syn::Expr>,
}

impl Loader {
    fn load(
        &self,
        ident: &syn::Ident,
        root: &proc_macro2::TokenStream,
    ) -> proc_macro2::TokenStream {
        let ty = &self.ty;
        let mut resolve_type = quote! { #ty };
        let mut suffix = quote! {};

        let config = if let Some(ref config_name) = self.config_name {
            quote! { &config[#config_name] }
        } else {
            quote! { &config }
        };

        if let Some(ref default) = self.default {
            resolve_type = quote! { Option<#resolve_type> };
            suffix = quote! { .unwrap_or_else(|| #default) #suffix };
        }

        if self.no_config {
            resolve_type = quote! { #root::NoConfig<#ty> };
            suffix = quote! { .into_inner() #suffix };
        }

        // Some constructor require &mut access to the fields
        quote! {
            let mut #ident: #ty =
                context.resolve::<#resolve_type>(
                    #config
                )? #suffix;
        }
    }
}

impl VariableSpecs {
    fn parse(field: &syn::Field) -> syn::parse::Result<Self> {
        let mut result = VariableSpecs {
            no_config: false,
            setter: None,
            foreach: None,
            callback: None,
            config_name: None,
            default: None,
        };
        // Look for an explicit #[blueprint]
        for attr in &field.attrs {
            if !attr.path().is_ident("blueprint") {
                continue;
            }

            // eprintln!("Parsing {attr:?}");

            attr.parse_nested_meta(|meta| {
                // eprintln!("Parsed nested meta: {:?} / {:?}", meta.path, meta.input);
                if meta.path.is_ident("config") {
                    let value = meta.value()?;
                    let config: syn::LitBool = value.parse()?;
                    result.no_config = !config.value();
                } else if meta.path.is_ident("foreach") {
                    let value = meta.value()?;
                    let foreach = value.parse()?;
                    result.foreach = Some(foreach);
                } else if meta.path.is_ident("setter") {
                    let value = meta.value()?;
                    let setter = value.parse()?;
                    result.setter = Some(setter);
                } else if meta.path.is_ident("callback") {
                    let value = meta.value()?;
                    let callback: syn::LitBool = value.parse()?;
                    result.callback = Some(callback.value());
                } else if meta.path.is_ident("default") {
                    let value = meta.value()?;
                    let default: syn::Expr = value.parse()?;
                    result.default = Some(default);
                } else if meta.path.is_ident("config_name") {
                    let value = meta.value()?;
                    let name: syn::LitStr = value.parse()?;
                    result.config_name = Some(name.value());
                } else {
                    panic!("Unrecognized ident: {:?}", meta.path);
                }
                Ok(())
            })?;
        }

        Ok(result)
    }
}

impl Variable {
    fn load(&self, root: &proc_macro2::TokenStream) -> proc_macro2::TokenStream {
        self.loader.load(&self.ident, root)
    }

    fn consume(&self, base_ident: &syn::Ident) -> proc_macro2::TokenStream {
        self.consumer.consume(base_ident, &self.ident)
    }

    fn parse(
        field: &syn::Field,
        struct_name: &str,
        constructor_fields: &HashSet<String>,
    ) -> syn::parse::Result<Self> {
        // First: is it one of the constructor fields? If so, skip the setter.
        let specs = VariableSpecs::parse(field)?;

        // An example from TextView:
        //
        // ```
        // #[crate::blueprint(TextView::empty())]
        // enum Blueprint {
        //     Empty,
        //
        //     Content(String),
        //
        //     Object { content: Option<String> },
        // }
        // ```
        //
        // Here we should be able to parse both:
        // * `Object { content: Object<String> }` as usual.
        // * `Content(String)` as `set_content(...)`
        //      For this, we need to know the name of the "struct" (here `Content`),
        //      since the field itself (`String`) is unnamed.

        let parameter_name = match field.ident {
            Some(ref ident) => ident.to_string(),
            None => struct_name.to_lowercase(),
        };
        let ident = syn::Ident::new(&parameter_name, Span::call_site());

        // Only one of setter/foreach/callback.
        let mut consumer = if constructor_fields.contains(&parameter_name) {
            Consumer::Noop
        } else {
            let inferred_type = looks_inferred(&field.ty);
            match (specs.setter, specs.foreach, specs.callback, inferred_type) {
                (None, None, None | Some(false), false) | (None, None, Some(false), true) => {
                    // Default case: use a setter based on the ident.
                    let setter = format!("set_{parameter_name}");
                    Consumer::Setter(Setter {
                        method: syn::Ident::new(&setter, Span::call_site()),
                    })
                }
                (Some(setter), None, None | Some(false), _) => {
                    // Explicit setter function
                    Consumer::Setter(Setter { method: setter })
                }
                (None, Some(foreach), None | Some(false), _) => {
                    // Foreach function (like `add_item`)
                    Consumer::ForEach(Box::new(Consumer::Setter(Setter { method: foreach })))
                }
                (None, None, Some(true), _) | (None, None, _, true) => {
                    // TODO: Check that the type is iterable? A Vec?

                    // Callback flag means we use the callback_helper-generated `_cb` setter.
                    let setter = format!("set_{parameter_name}_cb");
                    Consumer::Setter(Setter {
                        method: syn::Ident::new(&setter, Span::call_site()),
                    })
                }
                _ => panic!("unsupported configuration"),
            }
        };

        // Some types have special handling
        if is_option_type(&field.ty).is_some() {
            consumer = Consumer::Opt(Box::new(consumer));
        }

        // Now, how to fetch the config?
        // Either specs.config_name, or the parameter name
        let loader = Loader {
            no_config: specs.no_config,
            config_name: specs
                .config_name
                .or_else(|| field.ident.as_ref().map(|i| i.to_string())),
            ty: field.ty.clone(),
            default: specs.default,
        };

        Ok(Variable {
            consumer,
            loader,
            ident,
        })
    }
}

struct Setter {
    method: syn::Ident,
}

impl Setter {
    fn consume(
        &self,
        base_ident: &syn::Ident,
        field_ident: &syn::Ident,
    ) -> proc_macro2::TokenStream {
        let function = &self.method;
        quote! {
            #base_ident.#function(#field_ident);
        }
    }
}

/// Defines how to consume/use a variable.
///
/// For example: just include it in the constructor, or run a method on the item, ...
enum Consumer {
    // Not individually consumed.
    //
    // Most likely, the value is used in the constructor, no need to set it here.
    Noop,

    // The value is a Vec<T> and we need to call something on each item.
    ForEach(Box<Consumer>),

    Opt(Box<Consumer>),

    // We need to call this method to "set" the value.
    //
    // TODO: support more than just ident (expr? closure?)
    Setter(Setter),
}

impl Consumer {
    fn consume(
        &self,
        base_ident: &syn::Ident,
        field_ident: &syn::Ident,
    ) -> proc_macro2::TokenStream {
        match self {
            Consumer::Noop => quote! {},
            Consumer::Setter(setter) => setter.consume(base_ident, field_ident),
            Consumer::Opt(consumer) => {
                let consumer = consumer.consume(base_ident, field_ident);
                quote! {
                    if let Some(#field_ident) = #field_ident {
                        #consumer
                    }
                }
            }
            Consumer::ForEach(consumer) => {
                let consumer = consumer.consume(base_ident, field_ident);
                quote! {
                    for #field_ident in #field_ident {
                        #consumer
                    }
                }
            }
        }
    }
}

// Returns the quote!d code to build the object using this struct.
fn parse_struct(
    fields: &syn::Fields,
    parameter_names: &HashSet<String>,
    struct_name: &str,
    base: &syn::Expr,
    root: &proc_macro2::TokenStream,
) -> syn::parse::Result<proc_macro2::TokenStream> {
    // Assert: no generic?

    let fields = match fields {
        syn::Fields::Named(fields) => &fields.named,
        syn::Fields::Unnamed(fields) => &fields.unnamed,
        syn::Fields::Unit => {
            return Ok(quote! {
                #base
            });
        } // Nothing to do!
    };

    // We'll build:
    // - A list of parameter loaders
    // - A list of setter loaders
    let base_ident = syn::Ident::new("_res", Span::call_site());

    let vars: Vec<Variable> = fields
        .iter()
        .map(|field| Variable::parse(field, struct_name, parameter_names))
        .collect::<Result<_, _>>()?;

    let loaders: Vec<_> = vars.iter().map(|var| var.load(root)).collect();
    let consumers: Vec<_> = vars.iter().map(|var| var.consume(&base_ident)).collect();

    Ok(quote! {
        #(#loaders)*

        let mut #base_ident = #base ;

        #(#consumers)*

        #base_ident
    })
}

// Direct parsing (with minimal processing) of the attributes from a blueprint.
struct BlueprintAttributes {
    // Base expression to build. Ex: `TextView::new()`.
    // Might rely on variables in base_parameters.
    base: syn::Expr,

    // Set of parameter names we need for the constructor.
    // These might not need to be set separately.
    base_parameters: HashSet<String>,

    // Name for the blueprint.
    name: String,
}

fn find_parameters(expr: &syn::Expr, parameters: &mut HashSet<String>) {
    match expr {
        // Handle the main `View::new(...)` expression.
        syn::Expr::Call(syn::ExprCall { args, .. }) => {
            for arg in args {
                find_parameters(arg, parameters);
            }
        }
        // Handle individual variables given as parameters.
        syn::Expr::Path(syn::ExprPath { path, .. }) => {
            if path.segments.len() == 1 {
                parameters.insert(path.segments[0].ident.to_string());
            }
        }
        // Handle method calls like `var1.to_lowercase()`
        syn::Expr::MethodCall(syn::ExprMethodCall { receiver, args, .. }) => {
            find_parameters(receiver, parameters);
            for arg in args {
                find_parameters(arg, parameters);
            }
        }
        // Handle `[var1, var2]`
        syn::Expr::Array(syn::ExprArray { elems, .. }) => {
            for elem in elems {
                find_parameters(elem, parameters);
            }
        }
        syn::Expr::Reference(syn::ExprReference { expr, .. }) => find_parameters(expr, parameters),
        _ => (),
    }
}

fn base_default_name(expr: &syn::Expr) -> Option<String> {
    // From `TextView::new(content)`, return `TextView`.
    // If the expression is not such a method call, bail.
    let func = match expr {
        syn::Expr::Call(syn::ExprCall { func, .. }) => func,
        _ => return None,
    };

    let path = match &**func {
        syn::Expr::Path(syn::ExprPath { path, .. }) => path,
        _ => return None,
    };

    let struct_name_id = path.segments.len().checked_sub(2)?;
    let ident = &path.segments[struct_name_id].ident;
    Some(ident.to_string())
}

impl syn::parse::Parse for BlueprintAttributes {
    // Parse attributes for a blueprint. Ex:
    // #[blueprint(TextView::new(content), name="Text")]
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::parse::Result<Self> {
        let base: syn::Expr = input.parse()?;

        let mut base_parameters = HashSet::new();
        find_parameters(&base, &mut base_parameters);

        // Compute name and parameters from the expression.
        let mut name = base_default_name(&base).unwrap_or_default();

        // We can't parse this as a regular nested meta.
        // Parse it as a list of `key = value` items.
        // So far only `name = "Name"` is supported.
        while input.peek(syn::Token![,]) {
            let _comma: syn::Token![,] = input.parse()?;
            let path: syn::Path = input.parse()?;
            let _equal: syn::Token![=] = input.parse()?;
            if path.is_ident("name") {
                let value: syn::LitStr = input.parse()?;
                name = value.value();
            }
        }

        Ok(BlueprintAttributes {
            base,
            base_parameters,
            name,
        })
    }
}

pub fn blueprint(attrs: TokenStream, item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as syn::Item);

    // Parse the initial things given to the #[blueprint(...)] macro.
    // We expect:
    // Positional, first argument: an expression.
    // Optional, named arguments:
    // name = "BlueprintName"
    let attributes = syn::parse_macro_input!(attrs as BlueprintAttributes);

    // Either cursive or cursive_core are good roots.
    // If we can't find it, assume it's building cursive_core itself.
    let root = match find_crate::find_crate(|s| {
        s == "cursive" || s == "cursive-core" || s == "cursive_core"
    }) {
        Ok(cursive) => {
            let root = syn::Ident::new(&cursive.name, Span::call_site());
            quote! { ::#root }
        }
        Err(_) => {
            quote! { crate }
        }
    };

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
            parse_enum(item, &attributes.base_parameters, &attributes.base, &root).unwrap()
        }
        syn::Item::Struct(item) => {
            if !item.generics.params.is_empty() {
                panic!("expected non-generic struct");
            }

            let struct_name = item.ident.to_string();
            parse_struct(
                &item.fields,
                &attributes.base_parameters,
                &struct_name,
                &attributes.base,
                &root,
            )
            .unwrap()
        }
        _ => panic!("Expected enum or struct"),
    };

    let ident = syn::Ident::new(&attributes.name, Span::call_site());
    let result = quote! {
        #root::manual_blueprint!(#ident, |config, context| {
            Ok({ #builder })
        });
    };

    // eprintln!("Res: {result}");

    result.into()
}
