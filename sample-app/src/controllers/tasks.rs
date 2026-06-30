use crate::modules::task_lists_service::TaskListsService;
use crate::modules::tasks_service::TasksService;
use rustf::prelude::*;

pub fn install() -> Vec<Route> {
    async fn before(ctx: &mut Context) -> rustf::Result<BeforeAction> {
        if current_user_id(ctx).is_err() {
            ctx.flash_error("Please sign in to manage tasks.")?;
            ctx.redirect("/login")?;
            return Ok(BeforeAction::Stop);
        }
        Ok(BeforeAction::Continue)
    }

    routes![
        before: before,
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

    if is_htmx(ctx) {
        return render_list_fragment(ctx, user_id, list_id).await;
    }

    ctx.flash_success("Task created.")?;
    ctx.redirect("/task_lists")
}

async fn toggle(ctx: &mut Context) -> rustf::Result<()> {
    let user_id = current_user_id(ctx)?;
    let task_id = ctx.param_int("id")? as i64;
    let list_id = TasksService::toggle(user_id, task_id).await?;

    if is_htmx(ctx) {
        return render_list_fragment(ctx, user_id, list_id).await;
    }

    ctx.flash_success("Task updated.")?;
    ctx.redirect("/task_lists")
}

async fn destroy(ctx: &mut Context) -> rustf::Result<()> {
    let user_id = current_user_id(ctx)?;
    let task_id = ctx.param_int("id")? as i64;
    let list_id = TasksService::delete(user_id, task_id).await?;

    if is_htmx(ctx) {
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

fn is_htmx(ctx: &Context) -> bool {
    ctx.header("HX-Request")
        .map(|value| value.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}
