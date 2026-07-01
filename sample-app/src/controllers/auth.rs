use crate::modules::auth_service::AuthService;
use rustf::prelude::*;

pub fn install() -> Vec<Route> {
    routes![
        GET "/login" => login_form,
        POST "/login" => login,
        GET "/register" => register_form,
        POST "/register" => register,
        POST "/logout" => logout,
    ]
}

async fn login_form(ctx: &mut Context) -> rustf::Result<()> {
    if ctx.require_auth().is_ok() {
        ctx.redirect("/task_lists")?;
        return Ok(());
    }

    ctx.view(
        "auth/login",
        json!({
            "title": "Sign In",
            "csrf_token": ctx.generate_csrf(None)?,
        }),
    )
}

async fn login(ctx: &mut Context) -> rustf::Result<()> {
    let form = ctx.body_form()?;
    match AuthService::authenticate(&form).await {
        Ok(user) => {
            ctx.login(user.id)?;
            ctx.flash_success(format!("Welcome back, {}.", user.display_name))?;
            ctx.redirect("/task_lists")
        }
        Err(err) => {
            ctx.flash_error(format!("Sign in failed: {}", err))?;
            ctx.redirect("/login")
        }
    }
}

async fn register_form(ctx: &mut Context) -> rustf::Result<()> {
    if ctx.require_auth().is_ok() {
        ctx.redirect("/task_lists")?;
        return Ok(());
    }

    ctx.view(
        "auth/register",
        json!({
            "title": "Create Account",
            "csrf_token": ctx.generate_csrf(None)?,
        }),
    )
}

async fn register(ctx: &mut Context) -> rustf::Result<()> {
    let form = ctx.body_form()?;
    match AuthService::register(&form).await {
        Ok(user) => {
            ctx.login(user.id)?;
            ctx.flash_success(format!("Account created. Welcome, {}.", user.display_name))?;
            ctx.redirect("/task_lists")
        }
        Err(err) => {
            ctx.flash_error(format!("Registration failed: {}", err))?;
            ctx.redirect("/register")
        }
    }
}

async fn logout(ctx: &mut Context) -> rustf::Result<()> {
    ctx.logout()?;
    ctx.flash_success("Signed out successfully.")?;
    ctx.redirect("/")
}
