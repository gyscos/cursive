use cursive::views::{Button, EditView, TextView};

// This is how we can define some global recipes.
// Here, we define a recipe from a template.
cursive::raw_recipe!(LabeledField from {
    serde_yaml::from_str(include_str!("label-view.yaml")).unwrap()
});

cursive::raw_recipe!(VSpace from {
    serde_yaml::from_str(include_str!("vspace.yaml")).unwrap()
});

// We can also define recipe that build arbitrary views.
cursive::raw_recipe!(Titled, |config, context| {
    use cursive::views::Panel;

    // Fetch a string from the config
    let title: String = context.resolve(&config["title"])?;

    // Build a view from the other field
    let child = context.build(&config["child"])?;

    // And return some view
    Ok(Panel::new(child).title(title))
});

fn main() {
    cursive::logger::init();

    // We will build a view from a template (possibly written by another team)
    let mut context = cursive::builder::Context::new();

    // The only thing we need to know are the variables it expects.
    //
    // In our case, it's a title, and an on_edit callback.
    context.store("title", String::from("Config-driven layout example"));
    context.store("on_edit", EditView::on_edit_cb(on_edit_callback));
    context.store(
        "randomize",
        Button::new_cb(|s| {
            let cb = s
                .call_on_name("edit", |e: &mut EditView| {
                    e.set_content("Not so random!")
                })
                .unwrap();
            cb(s);
        }),
    );

    // Load the template - here it's a yaml file.
    const CONFIG: &str = include_str!("builder.yaml");
    let config = serde_yaml::from_str(CONFIG).unwrap();

    // And build the view
    let view = context.build(&config).unwrap();

    let mut siv = cursive::default();
    siv.add_global_callback('~', cursive::Cursive::toggle_debug_console);
    siv.screen_mut().add_transparent_layer(view);
    siv.run();
}

// Just a regular callback for EditView::on_edit
fn on_edit_callback(siv: &mut cursive::Cursive, text: &str, cursor: usize) {
    siv.call_on_name("status", |v: &mut TextView| {
        let spaces: String = std::iter::repeat(" ")
            .take(cursor + "You wrote `".len())
            .collect();
        v.set_content(format!("You wrote `{text}`\n{spaces}^"));
    })
    .unwrap();
}

// Still TODO:
// * Resolve a config, then let us work with it?
//  * From a recipe, we can:
//      * Get a sub-field (resolve!)
//      * Check the type of the config (array, string (then resolve!), number...)
//          * The problem is when we look for, say, an array, but actually got a variable that
//          _resolves_ to an array. :^/
//          * In that case... We can:
//              * _Also_ provide a way to do the same without a direct array value (but as a field)
//              * When checking a string, see if it's a variable (in the recipe).
//              * First try to resolve as an array config, then try string.
//              * Go full-on callback visitors instead, let the context resolve the thing, and call
//                  the appropriate visitor. Ugh.
//      * Almost every time we want to check a sub-object, we actually want to resolve it.
// * Write more recipes (almost done?)
// * Automate `if let Some(v) = context.resolve(&config[field_name])? { foo.set_v(v); }`
//      * Need an automatic "try_set_foo(&mut self, &Config, &Context)" function
//      * Need to register all such fields for a struct and call them all in turn?
//          * Might be from more than just one impl block... :(
//          * A derive on the type itself + proc macro per function?
//          * An attribute on the impl block? Each block?
//      * Could be done with inventory? (Post-macro?)
// * Simplify a bit Rc everywhere
//      * Especially for places where we already need a Rc<callback> anyway, maybe don't
//      double-wrap it?
// * Merge recipes & variables? ~~
// * Documentation
// * Standardize casing of values
//      * CamelCase? (Currently used for Views)
//      * snake_case? (Currently used for wrappers, keys and some values)
//      * space case? (Currently used for some values)
