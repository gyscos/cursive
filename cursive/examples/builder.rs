use cursive::views::{BoxedView, Button, EditView, Panel, TextView};

// This is how we can define some global blueprints.
// Here, we define a blueprint from a template.
cursive::manual_blueprint!(LabeledField from {
    // We just need to return a cursive::builder::Config
    // (in practice, a serde_json::Value).
    //
    // Here we parse yaml but any other serde-supported language would work.
    serde_yaml::from_str(include_str!("label-view.yaml")).unwrap()
});

cursive::manual_blueprint!(VSpace from {
    // Another similar blueprint.
    //
    // Here we embed the template in the binary with `include_str!`,
    // but it'd be possible as well to dynamically read a file,
    // load from network, ...
    //
    // Note that this code only runs when this blueprint is actually called
    // (when a `VSpace` view is requested).
    serde_yaml::from_str(include_str!("vspace.yaml")).unwrap()
});

// We can also define blueprint that build arbitrary views.
cursive::manual_blueprint!(Titled, |config, context| {
    // Manual blueprints just need to return something that implements `View`.

    // Fetch a string from the config
    let title: String = context.resolve(&config["title"])?;

    // Build a view from the other field
    let child = context.build(&config["child"])?;

    // And return some view
    Ok(Panel::new(child).title(title))
});

// Or we can use a declarative blueprint definition
#[cursive::blueprint(Panel::new(child), name = "WithTitle")]
struct Blueprint {
    // Some fields are used in the initialization expression above.
    child: BoxedView,

    // Additional fields use `set_*` setters.
    title: String,
}

fn main() {
    cursive::logger::init();

    // We will build a view from a template (possibly written by another team)
    let mut context = cursive::builder::Context::new();

    // The only thing we need to know are the variables it expects.
    //
    // In our case, it's a title string, and two callbacks.
    context.store("title", String::from("Config-driven layout example"));

    // Callbacks are tricky to store and need the exact closure type.
    // Here we use a helper function, `on_edit_cb`, to wrap a closure in the proper type.
    context.store("on_edit", EditView::on_edit_cb(on_edit_callback));

    // Each callback-taking function has a matching helper.
    context.store("randomize", Button::new_cb(randomize));

    // Load the template - here it's a yaml file.
    const CONFIG: &str = include_str!("builder.yaml");
    let config = serde_yaml::from_str(CONFIG).unwrap();

    // And build the view
    let view = context.build(&config).unwrap_or_else(|e| {
        panic!("{e:#?}");
    });

    let mut siv = cursive::default();
    siv.add_global_callback('~', cursive::Cursive::toggle_debug_console);
    siv.screen_mut().add_transparent_layer(view);
    siv.run();
}

fn randomize(s: &mut cursive::Cursive) {
    let cb = s
        .call_on_name("edit", |e: &mut EditView| e.set_content("Not so random!"))
        .unwrap();
    cb(s);
}

// Just a regular callback for EditView::on_edit
fn on_edit_callback(siv: &mut cursive::Cursive, text: &str, cursor: usize) {
    siv.call_on_name("status", |v: &mut TextView| {
        let spaces: String = " ".repeat(cursor + "You wrote `".len());
        v.set_content(format!("You wrote `{text}`\n{spaces}^"));
    })
    .unwrap();
}
