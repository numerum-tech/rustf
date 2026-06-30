use rustf::prelude::*;

pub fn install() -> Vec<Route> {
    routes![
        GET "/" => index,
        GET "/about" => about,
    ]
}

async fn index(ctx: &mut Context) -> Result<()> {
    if let Some(session) = ctx.session() {
        if session.is_authenticated() {
            ctx.redirect("/task_lists")?;
            return Ok(());
        }
    }

    let data = json!({
        "title": "RustF Tasks",
        "message": "A small task manager that demonstrates RustF sessions, schema-generated models, Bootstrap SSR views, and htmx interactions.",
        "features": [
            "Register and sign in with secure password hashing",
            "Create named task lists for each account",
            "Add tasks and complete them inline with htmx",
            "Bootstrap-powered SSR UI using RustF templates",
            "SQLite-backed sample data with schema bootstrap events",
            "Thin controllers with module-owned business logic"
        ]
    });

    ctx.view("home/index", data)
}

async fn about(ctx: &mut Context) -> Result<()> {
    let data = json!({
        "title": "About RustF Tasks",
        "description": "This sample app shows how RustF can power a convention-based MVC application with auth, SQLite persistence, SSR views, and htmx-enhanced interactions."
    });

    ctx.view("home/about", data)
}
