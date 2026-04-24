//! Home page business logic.
//!
//! Per the RustF layering rule, all non-HTTP logic lives in modules. Controllers
//! stay thin and call into this. For a tiny home page the "business logic" is
//! trivial — it's included anyway to model the pattern for real features.
//!
//! This is a plain (stateless) utility module: no `SharedModule` trait, no
//! `MODULE::register` call — just import and use.

use rustf::prelude::*;

pub struct HomeService;

impl HomeService {
    /// Feature list shown on the landing page.
    pub fn features() -> Vec<&'static str> {
        vec![
            "Auto-discovery of controllers, models, middleware, modules, and workers",
            "Dual-phase middleware with typed inbound/outbound separation",
            "Total.js-style template engine with layouts and partials",
            "Lock-free session storage via DashMap (and optional Redis)",
            "Trie-based routing with zero-copy parameter extraction",
            "Schema-driven model generation from YAML",
            "AI-friendly conventions for both humans and coding assistants",
        ]
    }

    /// Short project description shown on the about page.
    pub fn about_summary() -> &'static str {
        "RustF is a convention-based MVC web framework for Rust, inspired by Total.js v4. \
         It is designed to be equally productive for human developers and AI coding assistants."
    }

    /// Builds the repository payload handed to views. Controllers receive this
    /// and stash it into `ctx.repository_set`; keeps the controller itself free
    /// of structural decisions about what the view needs.
    pub fn index_view_data() -> serde_json::Value {
        json!({
            "title": "Welcome to RustF",
            "tagline": "AI-friendly MVC for Rust",
            "features": Self::features(),
        })
    }

    pub fn about_view_data() -> serde_json::Value {
        json!({
            "title": "About",
            "summary": Self::about_summary(),
        })
    }
}
