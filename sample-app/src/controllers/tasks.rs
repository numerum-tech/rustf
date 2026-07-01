use crate::modules::task_lists_service::TaskListsService;
use crate::modules::tasks_service::TasksService;
use rustf::prelude::*;

pub fn install() -> Vec<Route> {
    // Authentication for the /task_lists and /tasks prefixes is enforced by
    // AuthMiddleware (src/middleware/auth.rs), so no per-controller guard is
    // needed here.
    routes![
        POST "/task_lists/{id}/tasks" => create,
        POST "/tasks/{id}/toggle" => toggle,
        POST "/tasks/{id}/delete" => destroy,
    ]
}

async fn create(ctx: &mut Context) -> rustf::Result<()> {
    let user_id = current_user_id(ctx)?;
    let list_id = ctx.param_int("id")? as i64;
    let form = ctx.body_form()?;
    TasksService::create(user_id, list_id, &form).await?;

    if ctx.is_xhr() {
        return render_list_fragment(ctx, user_id, list_id).await;
    }

    ctx.flash_success("Task created.")?;
    ctx.redirect("/task_lists")
}

async fn toggle(ctx: &mut Context) -> rustf::Result<()> {
    let user_id = current_user_id(ctx)?;
    let task_id = ctx.param_int("id")? as i64;
    let list_id = TasksService::toggle(user_id, task_id).await?;

    if ctx.is_xhr() {
        return render_list_fragment(ctx, user_id, list_id).await;
    }

    ctx.flash_success("Task updated.")?;
    ctx.redirect("/task_lists")
}

async fn destroy(ctx: &mut Context) -> rustf::Result<()> {
    let user_id = current_user_id(ctx)?;
    let task_id = ctx.param_int("id")? as i64;
    let list_id = TasksService::delete(user_id, task_id).await?;

    if ctx.is_xhr() {
        return render_list_fragment(ctx, user_id, list_id).await;
    }

    ctx.flash_success("Task deleted.")?;
    ctx.redirect("/task_lists")
}

async fn render_list_fragment(ctx: &mut Context, user_id: i64, list_id: i64) -> rustf::Result<()> {
    let list = TaskListsService::list_fragment(user_id, list_id).await?;
    ctx.layout("");
    ctx.view("tasks/list", serde_json::to_value(&list)?)
}

fn current_user_id(ctx: &Context) -> rustf::Result<i64> {
    ctx.require_auth()?
        .get_user_id()
        .ok_or_else(|| rustf::Error::validation("Authenticated session is missing a user id"))
}
