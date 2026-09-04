//! Task Lists CRUD controller.
//!
//! HTTP LAYER ONLY. Per RustF's layering rule
//! (Base Model -> Model -> Module -> Controller), this file contains ZERO
//! calls to `TaskLists::*` or any other model. All business
//! logic, validation, and model access live in
//! `crate::modules::task_lists_service`. If you find yourself reaching for
//! `use crate::models::*` here, push that code into the service instead.

use crate::modules::task_lists_service::TaskListsService;
use rustf::prelude::*;

pub fn install() -> Vec<Route> {
    // Authentication for the /task_lists prefix is enforced by AuthMiddleware
    // (src/middleware/auth.rs), so no per-controller guard is needed here.
    routes![
        GET  "/task_lists"          => index,
        POST "/task_lists"          => create,
        GET  "/task_lists/{id}/edit" => edit_form,
        POST "/task_lists/{id}"      => update,
        POST "/task_lists/{id}/delete" => destroy,
    ]
}

async fn index(ctx: &mut Context) -> rustf::Result<()> {
    let user_id = current_user_id(ctx)?;
    let lists = TaskListsService::dashboard(user_id).await?;
    ctx.repository_set("lists", serde_json::to_value(&lists)?);
    ctx.view(
        "task_lists/index",
        json!({
            "title": "Your Task Lists",
            "csrf_token": ctx.generate_csrf(None)?,
            "has_lists": !lists.is_empty(),
        }),
    )
}

async fn create(ctx: &mut Context) -> rustf::Result<()> {
    let user_id = current_user_id(ctx)?;
    let form = ctx.body_form()?;
    match TaskListsService::create(user_id, &form).await {
        Ok(_) => {
            ctx.flash_success("Task list created.")?;
            ctx.redirect("/task_lists")
        }
        Err(e) => {
            ctx.flash_error(&format!("Create failed: {}", e))?;
            ctx.redirect("/task_lists")
        }
    }
}

async fn edit_form(ctx: &mut Context) -> rustf::Result<()> {
    let user_id = current_user_id(ctx)?;
    let id = ctx.param_as::<i64>("id")?;
    let item = TaskListsService::find_owned(user_id, id).await?;
    ctx.repository_set("item", serde_json::to_value(&item)?);
    ctx.view(
        "task_lists/edit",
        json!({
            "title": format!("Edit {}", item.title()),
            "csrf_token": ctx.generate_csrf(None)?,
        }),
    )
}

async fn update(ctx: &mut Context) -> rustf::Result<()> {
    let user_id = current_user_id(ctx)?;
    let id = ctx.param_as::<i64>("id")?;
    let form = ctx.body_form()?;
    match TaskListsService::update(user_id, id, &form).await {
        Ok(()) => {
            ctx.flash_success("Task list updated.")?;
            ctx.redirect("/task_lists")
        }
        Err(e) => {
            ctx.flash_error(&format!("Update failed: {}", e))?;
            ctx.redirect(&format!("/task_lists/{}/edit", id))
        }
    }
}

async fn destroy(ctx: &mut Context) -> rustf::Result<()> {
    let user_id = current_user_id(ctx)?;
    let id = ctx.param_as::<i64>("id")?;
    TaskListsService::delete(user_id, id).await?;
    ctx.flash_success("Task list deleted.")?;
    ctx.redirect("/task_lists")
}

fn current_user_id(ctx: &Context) -> rustf::Result<i64> {
    ctx.require_auth()?
        .get_user_id()
        .ok_or_else(|| rustf::Error::validation("Authenticated session is missing a user id"))
}
