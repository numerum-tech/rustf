//! Home controller — HTTP layer only.
//!
//! Routes and handlers. All data prep + business rules live in
//! `crate::modules::home_service`. This controller never touches models or
//! external services directly — that's the layering rule.

use crate::modules::home_service::HomeService;
use rustf::prelude::*;

pub fn install() -> Vec<Route> {
    routes![
        GET "/" => index,
        GET "/about" => about,
        GET "/api/status" => api_status,
    ]
}

async fn index(ctx: &mut Context) -> rustf::Result<()> {
    ctx.view("home/index", HomeService::index_view_data())
}

async fn about(ctx: &mut Context) -> rustf::Result<()> {
    ctx.view("home/about", HomeService::about_view_data())
}

async fn api_status(ctx: &mut Context) -> rustf::Result<()> {
    ctx.json(json!({
        "status": "ok",
        "framework": "rustf",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}
