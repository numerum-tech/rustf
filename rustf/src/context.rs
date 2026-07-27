use crate::error::{Error, Result};
use crate::http::{
    BodyData, FileCollection, FormData, FormValue, Request, RequestData, Response, UploadedFile,
};
use crate::session::Session;
use crate::views::ViewEngine;
use hyper::StatusCode;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::any::Any;
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// HTTP request context focused on request/response operations
///
/// Context handles only HTTP-related concerns following clean architecture principles.
/// Models and modules are accessed directly via imports, not through service locators.
/// Configuration is accessed through the global CONF, not through Context.
///
/// ## Typed Parameter Access
///
/// For typed query/route/body parameter access, prefer **source-first** method names for better discoverability:
/// - Query parameters: [`query_str`](Self::query_str), [`query_int`](Self::query_int), [`query_bool`](Self::query_bool)
/// - Route parameters: [`param_str`](Self::param_str), [`param_int`](Self::param_int)
/// - Body fields: [`body_str`](Self::body_str), [`body_int`](Self::body_int), [`body_bool`](Self::body_bool)
///
/// These methods are easier to find when exploring the API (e.g., searching for "query" shows `query_int`).
/// Type-first names (`int_query`, `str_param`, `str_body`) are still available but delegate to the source-first methods.
pub struct Context {
    pub req: Request,
    pub res: Option<Response>,
    session: Option<Arc<Session>>,
    views: Arc<ViewEngine>,
    layout_name: Option<String>,
    /// Per-request repository for controller-specific data accessible in views.
    ///
    /// Held as a `Value::Object` rather than a `HashMap` so `view` can hand the
    /// engine `&self.repository` directly instead of rebuilding (and deep-cloning)
    /// a `Value` on every render. Invariant: always `Value::Object`.
    repository: Value,
    /// Storage for middleware data (not accessible in views)
    data: HashMap<String, Box<dyn Any + Send + Sync>>,
    /// Signals that the session must be removed from storage and the cookie
    /// cleared in the outbound middleware.
    session_destroyed: AtomicBool,
    /// Cached form data to avoid re-parsing
    cached_form_data: Option<Result<HashMap<String, String>>>,
    /// Cached form data with array support
    cached_form_data_arrays: Option<Result<HashMap<String, FormValue>>>,
}

// Context is automatically Send + Sync due to Arc<T> being Send + Sync

impl Context {
    /// Create a new HTTP-focused context
    ///
    /// Context handles only HTTP request/response concerns.
    /// Configuration should be accessed through the global CONF.
    pub fn new(request: Request, views: Arc<ViewEngine>) -> Self {
        // Get default layout from global config, fallback to "layouts/default"
        let default_layout = crate::configuration::CONF::get_string("views.default_layout")
            .unwrap_or_else(|| "layouts/default".to_string());

        Self {
            req: request,
            res: Some(Response::ok()), // Initialize with default 200 OK response
            session: None,
            views,
            layout_name: Some(default_layout),
            repository: Value::Object(serde_json::Map::new()),
            data: HashMap::new(),
            session_destroyed: AtomicBool::new(false),
            cached_form_data: None,
            cached_form_data_arrays: None,
        }
    }

    // Response management methods

    /// Set the response for this context
    pub fn set_response(&mut self, response: Response) {
        self.res = Some(response);
    }

    /// Get the response if set
    pub fn get_response(&self) -> Option<&Response> {
        self.res.as_ref()
    }

    /// Take the response, leaving None in its place
    pub fn take_response(&mut self) -> Option<Response> {
        self.res.take()
    }

    /// Update response body and content-type while preserving other headers
    /// This is used internally by response methods to avoid losing middleware-set headers
    fn update_response_body(
        &mut self,
        body: Vec<u8>,
        content_type: &str,
        status: Option<StatusCode>,
    ) {
        let response = self.res.as_mut().unwrap(); // Safe - always exists

        // Update status if provided
        if let Some(new_status) = status {
            response.status = new_status;
        }

        // Update body
        response.body = body;

        // Update Content-Type (remove old, add new)
        response
            .headers
            .retain(|(name, _)| name.to_lowercase() != "content-type");
        response.add_header("Content-Type", content_type);
    }

    /// Update response preserving non-content headers
    /// Used for complex responses that need multiple headers like file downloads
    fn update_response(&mut self, mut new_response: Response) {
        let response = self.res.as_mut().unwrap(); // Safe - always exists

        // Collect non-content headers to preserve
        let preserved: Vec<_> = response
            .headers
            .iter()
            .filter(|(name, _)| !name.to_lowercase().starts_with("content-"))
            .cloned()
            .collect();

        // Add preserved headers to new response if not already present
        for (name, value) in preserved {
            if !new_response.headers.iter().any(|(n, _)| n == &name) {
                new_response.headers.push((name, value));
            }
        }

        // Replace with updated response
        *response = new_response;
    }

    // Middleware data storage methods

    /// Store data for middleware communication
    pub fn set<T: Any + Send + Sync + 'static>(&mut self, key: &str, value: T) -> Result<()> {
        self.data.insert(key.to_string(), Box::new(value));
        Ok(())
    }

    /// Retrieve data stored by middleware
    pub fn get<T: Any + Send + Sync + 'static>(&self, key: &str) -> Option<&T> {
        self.data
            .get(key)
            .and_then(|boxed| boxed.downcast_ref::<T>())
    }

    /// Check if data exists
    pub fn has_data(&self, key: &str) -> bool {
        self.data.contains_key(key)
    }

    // Session management methods

    /// Set session (used by middleware)
    pub fn set_session(&mut self, session: Option<Arc<Session>>) {
        self.session = session;
        if self.session.is_some() {
            self.session_destroyed.store(false, Ordering::Relaxed);
        }
    }

    /// Get session reference
    pub fn session(&self) -> Option<&Session> {
        self.session.as_deref()
    }

    /// Get session Arc (for middleware)
    pub fn session_arc(&self) -> Option<&Arc<Session>> {
        self.session.as_ref()
    }

    /// Check if session exists
    pub fn has_session(&self) -> bool {
        self.session.is_some()
    }

    /// Require session (returns error if missing)
    pub fn require_session(&self) -> Result<&Session> {
        self.session()
            .ok_or_else(|| Error::authentication("Session required"))
    }

    /// Require authenticated session
    pub fn require_auth(&self) -> Result<&Session> {
        let session = self.require_session()?;
        if !session.is_authenticated() {
            return Err(Error::authentication("Authentication required"));
        }
        Ok(session)
    }

    /// Login user (marks session for rotation)
    pub fn login(&self, user_id: i64) -> Result<()> {
        let session = self.require_session()?;
        session.set_user_id(user_id)?;
        session.set_privilege_level(1);
        session.mark_for_rotation();
        Ok(())
    }

    /// Logout user (clears session)
    pub fn logout(&self) -> Result<()> {
        self.session_destroy();
        Ok(())
    }

    // Flash message helpers (stored in session)

    /// Set flash message
    pub fn flash(&self, key: &str, value: impl serde::Serialize) -> Result<()> {
        let session = self.require_session()?;
        session.flash_set(key, value)?;
        Ok(())
    }

    /// Get and consume flash message
    pub fn get_flash(&self, key: &str) -> Option<Value> {
        let session = self.session()?;
        session.flash_get::<Value>(key)
    }

    /// Get all flash messages and clear them
    pub fn get_all_flash(&self) -> HashMap<String, Value> {
        let Some(session) = self.session() else {
            return HashMap::new();
        };

        session.flash_get_all()
    }

    // Total.js style methods

    /// Set the layout template (empty string for no layout)
    pub fn layout(&mut self, name: &str) -> &mut Self {
        self.layout_name = if name.is_empty() {
            None
        } else {
            Some(name.to_string())
        };
        self
    }

    // Repository methods for per-request data

    /// Set a value in the request-scoped repository, readable in views as
    /// `@{R.key}` (or `@{repository.key}`).
    ///
    /// This is the channel for **everything the page needs that is not its
    /// single subject**: title, CSRF token, collections, navigation, counters,
    /// permission flags. Call it once per key, as each piece of data becomes
    /// available — including from middleware and modules, which hold
    /// `&mut Context` and can contribute before the handler runs.
    ///
    /// Do not assemble a large `json!({...})` and pass it to
    /// [`view`](Self::view) instead. The model argument carries one simple
    /// object — see [`view`](Self::view) for the split.
    ///
    /// ```rust,ignore
    /// ctx.repository_set("title", "Edit task");
    /// ctx.repository_set("items", serde_json::to_value(&items)?);
    /// ```
    ///
    /// # Cost
    ///
    /// `Context` hands the engine a borrow of this object, so no clone happens
    /// here. The engine then deep-clones it once when it builds its render
    /// context — the same cost the model passed to [`view`](Self::view) pays.
    /// Both channels are therefore equal-cost; that remaining clone is an
    /// implementation detail slated to be removed. Do not reshape a page around
    /// it — put each piece of data in the channel it belongs to.
    pub fn repository_set(&mut self, key: &str, value: impl Into<Value>) -> &mut Self {
        // Invariant upheld by `new` and `repository_clear`.
        if let Some(map) = self.repository.as_object_mut() {
            map.insert(key.to_string(), value.into());
        }
        self
    }

    /// Get a value from the request-scoped repository
    pub fn repository_get(&self, key: &str) -> Option<&Value> {
        self.repository.as_object()?.get(key)
    }

    /// Clear the request-scoped repository
    pub fn repository_clear(&mut self) -> &mut Self {
        self.repository = Value::Object(serde_json::Map::new());
        self
    }

    /// Render a view template with its model.
    ///
    /// # What belongs in `data`
    ///
    /// `data` becomes the view's model, read as `@{M.field}` (or
    /// `@{model.field}`). It carries **one simple object**: the record or form
    /// the page is about. Typically `serde_json::to_value(&record)?`.
    ///
    /// Everything else — title, CSRF token, collections, navigation, counters —
    /// goes through [`repository_set`](Self::repository_set) and is read as
    /// `@{R.key}`. A page with no single subject (a list, a dashboard) passes
    /// `json!({})` here and puts its data in the repository.
    ///
    /// ```rust,ignore
    /// // Detail page: the record is the model.
    /// ctx.repository_set("title", "Task detail");
    /// ctx.view("tasks/show", serde_json::to_value(&task)?)
    ///
    /// // List page: no single subject.
    /// ctx.repository_set("title", "Tasks");
    /// ctx.repository_set("items", serde_json::to_value(&tasks)?);
    /// ctx.view("tasks/index", json!({}))
    /// ```
    ///
    /// Do **not** collect unrelated page data into one large `json!({...})`
    /// here. `data` has a single write point, so anything cross-cutting has to
    /// be rebuilt by hand in every handler; the repository accumulates across
    /// middleware, modules, and the handler.
    ///
    /// # Concurrency
    ///
    /// Rendering is CPU-bound and runs **synchronously on the current async
    /// worker thread** — there is no `.await` inside the template engine, so the
    /// worker is occupied for the full render duration. With the template cache
    /// (very high hit rate) most renders are microsecond-scale and this is a
    /// non-issue. But a pathologically large template or data payload can pin a
    /// worker and raise tail latency for other requests sharing the runtime.
    /// If you knowingly render heavy pages, offload with
    /// [`tokio::task::spawn_blocking`] / [`tokio::task::block_in_place`].
    /// A threshold-gated offload inside the framework is tracked as future work.
    pub fn view(&mut self, template: &str, data: Value) -> Result<()> {
        let views = Arc::clone(&self.views);

        // Build per-request session Value if a session is active
        let session_value = if let Some(session) = self.session() {
            let flash = session.flash_peek_all();
            let mut session_data = session.to_value();
            if let Value::Object(ref mut map) = session_data {
                map.insert("id".to_string(), Value::String(session.id().to_string()));
                map.insert(
                    "authenticated".to_string(),
                    Value::Bool(session.is_authenticated()),
                );
                map.insert(
                    "user_id".to_string(),
                    session
                        .get_user_id()
                        .map(|id| Value::Number(id.into()))
                        .unwrap_or(Value::Null),
                );
                map.insert(
                    "flash".to_string(),
                    Value::Object(flash.into_iter().collect()),
                );
            }
            Some(session_data)
        } else {
            None
        };

        // Per-request metadata for @{url} / @{hostname} / @{mobile}.
        let request_meta = crate::views::RequestMeta {
            url: self.req.path().to_string(),
            hostname: self.req.hostname(None),
            mobile: self
                .req
                .user_agent()
                .map(crate::views::is_mobile)
                .unwrap_or(false),
        };

        // Pass repository and session as separate arguments — avoids hidden-field
        // packing and the data.clone() needed to strip those fields in the engine.
        let rendered = views.render_rich(
            template,
            &data,
            self.layout_name.as_deref(),
            Some(&self.repository),
            session_value.as_ref(),
            Some(&request_meta),
        )?;

        if let Some(session) = self.session() {
            if !session.flash_peek_all().is_empty() {
                session.flash_clear();
            }
        }
        self.update_response_body(rendered.into_bytes(), "text/html; charset=utf-8", None);
        Ok(())
    }

    /// Redirect to another URL
    pub fn redirect(&mut self, path: &str) -> Result<()> {
        let redirect = Response::redirect(path)?;
        self.update_response(redirect);
        Ok(())
    }

    /// Redirect permanently to a local application path.
    pub fn redirect_permanent(&mut self, path: &str) -> Result<()> {
        let redirect = Response::redirect_permanent(path)?;
        self.update_response(redirect);
        Ok(())
    }

    /// Redirect to an external URL explicitly.
    pub fn redirect_external(&mut self, url: &str) -> Result<()> {
        let redirect = Response::redirect_external(url);
        self.update_response(redirect);
        Ok(())
    }

    /// Redirect permanently to an external URL explicitly.
    pub fn redirect_external_permanent(&mut self, url: &str) -> Result<()> {
        let redirect = Response::redirect_external_permanent(url);
        self.update_response(redirect);
        Ok(())
    }

    /// Return a 404 response
    pub fn view404(&mut self) -> Result<()> {
        self.update_response_body(
            b"Not Found".to_vec(),
            "text/plain; charset=utf-8",
            Some(StatusCode::NOT_FOUND),
        );
        Ok(())
    }

    // Total.js-style HTTP error responses

    /// Return 400 Bad Request response
    pub fn throw400(&mut self, message: Option<&str>) -> Result<()> {
        let body = message.unwrap_or("Bad Request");
        self.update_response_body(
            body.as_bytes().to_vec(),
            "text/plain; charset=utf-8",
            Some(StatusCode::BAD_REQUEST),
        );
        Ok(())
    }

    /// Return 401 Unauthorized response
    pub fn throw401(&mut self, message: Option<&str>) -> Result<()> {
        let body = message.unwrap_or("Unauthorized");
        self.update_response_body(
            body.as_bytes().to_vec(),
            "text/plain; charset=utf-8",
            Some(StatusCode::UNAUTHORIZED),
        );
        Ok(())
    }

    /// Return 403 Forbidden response
    pub fn throw403(&mut self, message: Option<&str>) -> Result<()> {
        let body = message.unwrap_or("Forbidden");
        self.update_response_body(
            body.as_bytes().to_vec(),
            "text/plain; charset=utf-8",
            Some(StatusCode::FORBIDDEN),
        );
        Ok(())
    }

    /// Return 404 Not Found response with optional custom message
    pub fn throw404(&mut self, message: Option<&str>) -> Result<()> {
        let body = message.unwrap_or("Not Found");
        self.update_response_body(
            body.as_bytes().to_vec(),
            "text/plain; charset=utf-8",
            Some(StatusCode::NOT_FOUND),
        );
        Ok(())
    }

    /// Return 409 Conflict response
    pub fn throw409(&mut self, message: Option<&str>) -> Result<()> {
        let body = message.unwrap_or("Conflict");
        self.update_response_body(
            body.as_bytes().to_vec(),
            "text/plain; charset=utf-8",
            Some(StatusCode::CONFLICT),
        );
        Ok(())
    }

    /// Return 500 Internal Server Error response
    pub fn throw500(&mut self, message: Option<&str>) -> Result<()> {
        let body = message.unwrap_or("Internal Server Error");
        self.update_response_body(
            body.as_bytes().to_vec(),
            "text/plain; charset=utf-8",
            Some(StatusCode::INTERNAL_SERVER_ERROR),
        );
        Ok(())
    }

    /// Return 501 Not Implemented response
    pub fn throw501(&mut self, message: Option<&str>) -> Result<()> {
        let body = message.unwrap_or("Not Implemented");
        self.update_response_body(
            body.as_bytes().to_vec(),
            "text/plain; charset=utf-8",
            Some(StatusCode::NOT_IMPLEMENTED),
        );
        Ok(())
    }

    /// Return empty response (204 No Content)
    pub fn empty(&mut self) -> Result<()> {
        self.update_response_body(Vec::new(), "text/plain", Some(StatusCode::NO_CONTENT));
        Ok(())
    }

    /// Return success response with optional data
    pub fn success<T: serde::Serialize>(&mut self, data: Option<T>) -> Result<()> {
        let body = match data {
            Some(d) => serde_json::to_string(&d)?.into_bytes(),
            None => b"{\"success\":true}".to_vec(),
        };
        self.update_response_body(body, "application/json", None);
        Ok(())
    }

    /// Return plain text response (Total.js style)
    pub fn plain(&mut self, text: impl Into<String>) -> Result<()> {
        self.update_response_body(text.into().into_bytes(), "text/plain; charset=utf-8", None);
        Ok(())
    }

    /// Return HTML response (Total.js style)
    pub fn html(&mut self, content: impl Into<String>) -> Result<()> {
        self.update_response_body(
            content.into().into_bytes(),
            "text/html; charset=utf-8",
            None,
        );
        Ok(())
    }

    // Total.js-style client information properties

    /// Get client IP address (Total.js: controller.ip)
    pub fn ip(&self) -> String {
        self.req.client_ip()
    }

    /// Get user agent string (Total.js: controller.ua)
    pub fn user_agent(&self) -> Option<&str> {
        self.req.user_agent()
    }

    /// Check if request is from mobile device (Total.js: controller.mobile)
    pub fn is_mobile(&self) -> bool {
        self.req.is_mobile()
    }

    /// Check if request is from bot/crawler (Total.js: controller.robot)
    pub fn is_robot(&self) -> bool {
        self.req.is_robot()
    }

    /// Check if request is HTTPS (Total.js: controller.secured)
    pub fn is_secure(&self) -> bool {
        self.req.is_secure()
    }

    /// Check if request is AJAX/XHR (Total.js: controller.xhr)
    pub fn is_xhr(&self) -> bool {
        self.req.is_xhr()
    }

    /// Get preferred language (Total.js: controller.language)
    pub fn language(&self) -> Option<&str> {
        self.req.language()
    }

    /// Get HTTP referrer (Total.js: controller.referrer)
    pub fn referrer(&self) -> Option<&str> {
        self.req.referrer()
    }

    /// Get full request URL (Total.js: controller.url)
    pub fn url(&self) -> &str {
        &self.req.uri
    }

    // Total.js-style file handling methods

    /// Get uploaded files (Total.js: controller.files)
    pub fn files(&mut self) -> Result<&FileCollection> {
        self.req.files()
    }

    /// Get specific uploaded file by field name
    pub fn file(&mut self, field_name: &str) -> Result<Option<&UploadedFile>> {
        self.req.file(field_name)
    }

    /// Send file download response (Total.js: controller.file)
    pub async fn file_download<P: AsRef<Path>>(
        &mut self,
        path: P,
        download_name: Option<&str>,
    ) -> Result<()> {
        let new_response = Response::file_download(path, download_name).await?;
        self.update_response(new_response);
        Ok(())
    }

    /// Send file download response constrained to a base directory.
    pub async fn file_download_from<B: AsRef<Path>, P: AsRef<Path>>(
        &mut self,
        base_dir: B,
        path: P,
        download_name: Option<&str>,
    ) -> Result<()> {
        let new_response = Response::file_download_from(base_dir, path, download_name).await?;
        self.update_response(new_response);
        Ok(())
    }

    /// Send file for inline viewing
    pub async fn file_inline<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let new_response = Response::file_inline(path).await?;
        self.update_response(new_response);
        Ok(())
    }

    /// Send inline file response constrained to a base directory.
    pub async fn file_inline_from<B: AsRef<Path>, P: AsRef<Path>>(
        &mut self,
        base_dir: B,
        path: P,
    ) -> Result<()> {
        let new_response = Response::file_inline_from(base_dir, path).await?;
        self.update_response(new_response);
        Ok(())
    }

    /// Send binary data response (Total.js: controller.binary)
    pub fn binary(
        &mut self,
        data: Vec<u8>,
        content_type: &str,
        download_name: Option<&str>,
    ) -> Result<()> {
        let new_response = Response::binary(data, content_type, download_name);
        self.update_response(new_response);
        Ok(())
    }

    /// Send streaming response (Total.js: controller.stream)
    pub fn stream(
        &mut self,
        data: Vec<u8>,
        content_type: &str,
        download_name: Option<&str>,
    ) -> Result<()> {
        let new_response = Response::stream(data, content_type, download_name);
        self.update_response(new_response);
        Ok(())
    }

    /// Set flash success message
    pub fn flash_success(&self, message: impl Into<String>) -> Result<()> {
        self.flash("success", message.into())
    }

    /// Set flash error message
    pub fn flash_error(&self, message: impl Into<String>) -> Result<()> {
        self.flash("error", message.into())
    }

    /// Set flash info message
    pub fn flash_info(&self, message: impl Into<String>) -> Result<()> {
        self.flash("info", message.into())
    }

    /// Set flash warning message
    pub fn flash_warning(&self, message: impl Into<String>) -> Result<()> {
        self.flash("warning", message.into())
    }

    /// Clear all flash messages
    pub fn flash_clear(&self) -> Result<()> {
        let session = self.require_session()?;
        session.flash_clear();
        Ok(())
    }

    /// Clear a specific flash message by key
    pub fn flash_clear_key(&self, key: &str) -> Result<()> {
        let session = self.require_session()?;
        session.flash_remove(key);
        Ok(())
    }

    /// Get a URL parameter
    pub fn param(&self, key: &str) -> Option<&str> {
        self.req.params.get(key).map(|s| s.as_str())
    }

    /// Get a query parameter
    pub fn query(&self, key: &str) -> Option<&str> {
        self.req.query.get(key).map(|s| s.as_str())
    }

    // Source-first query parameter methods (canonical implementation)

    /// Get a query parameter as string (returns error if missing).
    /// Preferred discoverable name for query parameter access.
    pub fn query_str(&self, key: &str) -> Result<String> {
        self.req
            .query
            .get(key)
            .filter(|s| !s.is_empty())
            .cloned()
            .ok_or_else(|| Error::InvalidInput(format!("Query parameter '{}' is required", key)))
    }

    /// Get a query parameter as integer (returns error if missing or invalid).
    /// Preferred discoverable name for query parameter access.
    pub fn query_int(&self, key: &str) -> Result<i32> {
        self.query_str(key)?.parse().map_err(|_| {
            Error::InvalidInput(format!("Query parameter '{}' must be a valid integer", key))
        })
    }

    /// Get a query parameter as boolean (returns error if missing).
    /// Preferred discoverable name for query parameter access.
    pub fn query_bool(&self, key: &str) -> Result<bool> {
        let value = self.query_str(key)?;
        Ok(matches!(value.as_str(), "true" | "1" | "yes" | "on"))
    }

    /// Get a query parameter as string with default value.
    /// Preferred discoverable name for query parameter access.
    pub fn query_str_or(&self, key: &str, default: &str) -> String {
        self.query_str(key).unwrap_or_else(|_| default.to_string())
    }

    /// Get a query parameter as integer with default value.
    /// Preferred discoverable name for query parameter access.
    pub fn query_int_or(&self, key: &str, default: i32) -> i32 {
        self.query_int(key).unwrap_or(default)
    }

    /// Get a query parameter as boolean with default value.
    /// Preferred discoverable name for query parameter access.
    pub fn query_bool_or(&self, key: &str, default: bool) -> bool {
        self.query_bool(key).unwrap_or(default)
    }

    // Type-first query parameter methods (delegate to source-first)

    /// Get a query parameter (returns error if missing)
    /// Prefer [`query_str`](Self::query_str) for discoverability.
    pub fn str_query(&self, key: &str) -> Result<String> {
        self.query_str(key)
    }

    /// Get a query parameter as integer
    /// Prefer [`query_int`](Self::query_int) for discoverability.
    pub fn int_query(&self, key: &str) -> Result<i32> {
        self.query_int(key)
    }

    /// Get a query parameter as boolean
    /// Prefer [`query_bool`](Self::query_bool) for discoverability.
    pub fn bool_query(&self, key: &str) -> Result<bool> {
        self.query_bool(key)
    }

    /// Get a query parameter with default
    /// Prefer [`query_str_or`](Self::query_str_or) for discoverability.
    pub fn str_query_or(&self, key: &str, default: &str) -> String {
        self.query_str_or(key, default)
    }

    /// Get a query parameter as integer with default
    /// Prefer [`query_int_or`](Self::query_int_or) for discoverability.
    pub fn int_query_or(&self, key: &str, default: i32) -> i32 {
        self.query_int_or(key, default)
    }

    /// Get a query parameter as boolean with default
    /// Prefer [`query_bool_or`](Self::query_bool_or) for discoverability.
    pub fn bool_query_or(&self, key: &str, default: bool) -> bool {
        self.query_bool_or(key, default)
    }

    // Source-first route parameter methods (canonical implementation)

    /// Get a route parameter as string (returns error if missing).
    /// Preferred discoverable name for route parameter access.
    pub fn param_str(&self, key: &str) -> Result<String> {
        self.req
            .params
            .get(key)
            .filter(|s| !s.is_empty())
            .cloned()
            .ok_or_else(|| Error::InvalidInput(format!("Route parameter '{}' is required", key)))
    }

    /// Get a route parameter as integer (returns error if missing or invalid).
    /// Preferred discoverable name for route parameter access.
    pub fn param_int(&self, key: &str) -> Result<i32> {
        self.param_str(key)?.parse().map_err(|_| {
            Error::InvalidInput(format!("Route parameter '{}' must be a valid integer", key))
        })
    }

    /// Get a route parameter as string with default value.
    /// Preferred discoverable name for route parameter access.
    pub fn param_str_or(&self, key: &str, default: &str) -> String {
        self.param_str(key).unwrap_or_else(|_| default.to_string())
    }

    /// Get a route parameter as integer with default value.
    /// Preferred discoverable name for route parameter access.
    pub fn param_int_or(&self, key: &str, default: i32) -> i32 {
        self.param_int(key).unwrap_or(default)
    }

    // Type-first route parameter methods (delegate to source-first)

    /// Get a route parameter (returns error if missing)
    /// Prefer [`param_str`](Self::param_str) for discoverability.
    pub fn str_param(&self, key: &str) -> Result<String> {
        self.param_str(key)
    }

    /// Get a route parameter as integer
    /// Prefer [`param_int`](Self::param_int) for discoverability.
    pub fn int_param(&self, key: &str) -> Result<i32> {
        self.param_int(key)
    }

    /// Get a route parameter with default
    /// Prefer [`param_str_or`](Self::param_str_or) for discoverability.
    pub fn str_param_or(&self, key: &str, default: &str) -> String {
        self.param_str_or(key, default)
    }

    /// Get a route parameter as integer with default
    /// Prefer [`param_int_or`](Self::param_int_or) for discoverability.
    pub fn int_param_or(&self, key: &str, default: i32) -> i32 {
        self.param_int_or(key, default)
    }

    /// Get request body as JSON
    pub fn body_json<T: serde::de::DeserializeOwned>(&self) -> Result<T> {
        self.req.body_as_json()
    }

    /// Get request body as form data (cached to avoid re-parsing).
    /// Returns a [`FormData`] wrapper with typed getters: `form.get_int("id")?`, `form.get_str("name")?`, etc.
    pub fn body_form(&mut self) -> Result<FormData> {
        if self.cached_form_data.is_none() {
            self.cached_form_data = Some(self.req.body_as_form());
        }
        match self.cached_form_data.as_ref().unwrap() {
            Ok(data) => Ok(FormData::new(data.clone())),
            Err(e) => Err(Error::InvalidInput(e.to_string())),
        }
    }

    /// Get request body as form data with array support (field[] syntax)
    /// Returns a reference to cached data to avoid cloning overhead
    pub fn body_form_data(&mut self) -> Result<&HashMap<String, FormValue>> {
        if self.cached_form_data_arrays.is_none() {
            self.cached_form_data_arrays = Some(self.req.body_as_form_data());
        }
        match self.cached_form_data_arrays.as_ref().unwrap() {
            Ok(data) => Ok(data),
            Err(e) => Err(Error::InvalidInput(e.to_string())),
        }
    }

    /// Get request body as form data with array support (cloned)
    /// Use this only when you need ownership of the data
    pub fn body_form_data_cloned(&mut self) -> Result<HashMap<String, FormValue>> {
        self.body_form_data().map(|data| data.clone())
    }

    /// Parse form data into a typed structure
    pub fn body_form_typed<T: DeserializeOwned>(&mut self) -> Result<T> {
        let form_data = self.body_form_data()?;

        // Convert HashMap<String, FormValue> to serde_json::Value for deserialization
        let mut json_map = serde_json::Map::new();
        for (key, value) in form_data.iter() {
            let json_value = match value {
                FormValue::Single(s) => serde_json::Value::String(s.clone()),
                FormValue::Multiple(v) => serde_json::Value::Array(
                    v.iter()
                        .map(|s| serde_json::Value::String(s.clone()))
                        .collect(),
                ),
            };
            json_map.insert(key.clone(), json_value);
        }

        let json_value = serde_json::Value::Object(json_map);
        serde_json::from_value(json_value).map_err(Error::Json)
    }

    // Source-first body field methods (canonical implementation)

    /// Get a field from body as string (returns error if missing).
    /// Preferred discoverable name for body field access.
    pub fn body_str(&mut self, key: &str) -> Result<String> {
        self.body_form_data()?
            .get(key)
            .map(|v| v.as_string())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .ok_or_else(|| Error::InvalidInput(format!("Field '{}' is required", key)))
    }

    /// Get a field from body as integer (returns error if missing or invalid).
    /// Preferred discoverable name for body field access.
    pub fn body_int(&mut self, key: &str) -> Result<i32> {
        self.body_str(key)?
            .parse()
            .map_err(|_| Error::InvalidInput(format!("Field '{}' must be a valid integer", key)))
    }

    /// Get a field from body as boolean (returns error if missing).
    /// Preferred discoverable name for body field access.
    pub fn body_bool(&mut self, key: &str) -> Result<bool> {
        let value = self.body_str(key)?;
        Ok(matches!(
            value.as_str(),
            "true" | "1" | "yes" | "on" | "checked"
        ))
    }

    /// Get a field from body as string with default value.
    /// Preferred discoverable name for body field access.
    pub fn body_str_or(&mut self, key: &str, default: &str) -> String {
        self.body_str(key).unwrap_or_else(|_| default.to_string())
    }

    /// Get a field from body as integer with default value.
    /// Preferred discoverable name for body field access.
    pub fn body_int_or(&mut self, key: &str, default: i32) -> i32 {
        self.body_int(key).unwrap_or(default)
    }

    /// Get a field from body as boolean with default value.
    /// Preferred discoverable name for body field access.
    pub fn body_bool_or(&mut self, key: &str, default: bool) -> bool {
        self.body_bool(key).unwrap_or(default)
    }

    // Type-first body field methods (delegate to source-first)

    /// Get a field from body (returns error if missing)
    /// Prefer [`body_str`](Self::body_str) for discoverability.
    pub fn str_body(&mut self, key: &str) -> Result<String> {
        self.body_str(key)
    }

    /// Get a field from body as integer
    /// Prefer [`body_int`](Self::body_int) for discoverability.
    pub fn int_body(&mut self, key: &str) -> Result<i32> {
        self.body_int(key)
    }

    /// Get a field from body as boolean
    /// Prefer [`body_bool`](Self::body_bool) for discoverability.
    pub fn bool_body(&mut self, key: &str) -> Result<bool> {
        self.body_bool(key)
    }

    /// Get a field from body with default
    /// Prefer [`body_str_or`](Self::body_str_or) for discoverability.
    pub fn str_body_or(&mut self, key: &str, default: &str) -> String {
        self.body_str_or(key, default)
    }

    /// Get a field from body as integer with default
    /// Prefer [`body_int_or`](Self::body_int_or) for discoverability.
    pub fn int_body_or(&mut self, key: &str, default: i32) -> i32 {
        self.body_int_or(key, default)
    }

    /// Get a field from body as boolean with default
    /// Prefer [`body_bool_or`](Self::body_bool_or) for discoverability.
    pub fn bool_body_or(&mut self, key: &str, default: bool) -> bool {
        self.body_bool_or(key, default)
    }

    /// Get the full body data as JSON (converts form data to JSON if needed)
    /// This provides a unified interface regardless of content type
    pub fn full_body(&mut self) -> Result<serde_json::Value> {
        let content_type = self
            .req
            .headers
            .get("content-type")
            .map(|s| s.as_str())
            .unwrap_or("");

        if content_type.contains("application/json") {
            // Already JSON, return as-is
            self.body_json::<serde_json::Value>()
        } else if content_type.contains("application/x-www-form-urlencoded")
            || content_type.contains("multipart/form-data")
        {
            // Convert form data to JSON
            let form_data = self.body_form_data()?;
            Ok(Self::form_to_json(form_data))
        } else {
            let text = self.req.body_as_string();
            if text.is_empty() {
                Ok(serde_json::Value::Null)
            } else {
                Ok(serde_json::Value::String(text))
            }
        }
    }

    /// Get the raw body as string (for text, XML, or other content types)
    pub fn raw_body(&self) -> String {
        self.req.body_as_string()
    }

    /// Helper function to convert form data to JSON
    fn form_to_json(form_data: &HashMap<String, FormValue>) -> serde_json::Value {
        let mut json_map = serde_json::Map::new();

        for (key, value) in form_data {
            let json_value = match value {
                FormValue::Single(s) => {
                    // Try to parse as number or boolean, fallback to string
                    if let Ok(n) = s.parse::<i64>() {
                        serde_json::Value::Number(n.into())
                    } else if let Ok(f) = s.parse::<f64>() {
                        serde_json::Number::from_f64(f)
                            .map(serde_json::Value::Number)
                            .unwrap_or_else(|| serde_json::Value::String(s.clone()))
                    } else if s == "true" || s == "false" {
                        serde_json::Value::Bool(s == "true")
                    } else {
                        serde_json::Value::String(s.clone())
                    }
                }
                FormValue::Multiple(arr) => {
                    let values: Vec<serde_json::Value> = arr
                        .iter()
                        .map(|s| {
                            // Try to parse each value
                            if let Ok(n) = s.parse::<i64>() {
                                serde_json::Value::Number(n.into())
                            } else if let Ok(f) = s.parse::<f64>() {
                                serde_json::Number::from_f64(f)
                                    .map(serde_json::Value::Number)
                                    .unwrap_or_else(|| serde_json::Value::String(s.clone()))
                            } else if s == "true" || s == "false" {
                                serde_json::Value::Bool(s == "true")
                            } else {
                                serde_json::Value::String(s.clone())
                            }
                        })
                        .collect();
                    serde_json::Value::Array(values)
                }
            };
            json_map.insert(key.clone(), json_value);
        }

        serde_json::Value::Object(json_map)
    }

    /// Extract request data for passing to service layers
    pub fn request_data(&mut self) -> Result<RequestData> {
        // Build body data based on content type
        let content_type = self
            .req
            .headers
            .get("content-type")
            .map(|s| s.as_str())
            .unwrap_or("");

        let body = if content_type.contains("application/json") {
            let json = self.body_json::<serde_json::Value>()?;
            BodyData::Json(json)
        } else if content_type.contains("application/x-www-form-urlencoded")
            || content_type.contains("multipart/form-data")
        {
            let form = self.body_form_data()?;
            BodyData::Form(form.clone())
        } else {
            let text = self.req.body_as_string();
            if text.is_empty() {
                BodyData::Empty
            } else {
                BodyData::Text(text)
            }
        };

        Ok(RequestData::new(
            self.req.query.clone(),
            self.req.params.clone(),
            body,
            self.req.headers.clone(),
            self.req.method.clone(),
            self.req.uri.clone(),
        ))
    }

    /// Get a header value
    pub fn header(&self, name: &str) -> Option<&str> {
        let normalized = name.to_ascii_lowercase();
        self.req.headers.get(&normalized).map(|s| s.as_str())
    }

    /// Add a header to the response
    pub fn add_header(&mut self, name: impl Into<String>, value: impl Into<String>) {
        if let Some(response) = self.res.as_mut() {
            let name = name.into();
            let value = value.into();
            response.add_header(&name, &value);
        }
    }

    /// Set the response status code
    pub fn status(&mut self, status: hyper::StatusCode) {
        if let Some(response) = self.res.as_mut() {
            response.status = status;
        }
    }

    /// Return JSON response
    pub fn json<T: serde::Serialize>(&mut self, data: T) -> Result<()> {
        let json_string = serde_json::to_string(&data)?;
        self.update_response_body(json_string.into_bytes(), "application/json", None);
        Ok(())
    }

    /// Return text response
    pub fn text(&mut self, content: impl Into<String>) -> Result<()> {
        self.update_response_body(
            content.into().into_bytes(),
            "text/plain; charset=utf-8",
            None,
        );
        Ok(())
    }

    /// Total.js style "cancel" - return self for chaining
    pub fn cancel(&self) -> &Self {
        self
    }

    /// Session convenience methods
    pub fn session_set<T: serde::Serialize>(&self, key: &str, value: T) -> Result<()> {
        let session = self.require_session()?;
        session.set(key, value)
    }

    pub fn session_get<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.session()?.get(key)
    }

    /// Clear all session data and flash messages
    ///
    /// This removes all data from the session but keeps the session ID intact.
    /// Useful for user logout where you want to clear data but maintain session tracking.
    pub fn session_clear(&self) {
        if let Some(session) = self.session() {
            session.clear();
        }
    }

    /// Flush all session data (alias for session_clear)
    ///
    /// Provides Laravel-style compatibility for developers familiar with flush().
    pub fn session_flush(&self) {
        self.session_clear();
    }

    /// Mark session for destruction (clears all data)
    ///
    /// This clears all session data locally. For complete session destruction
    /// including removal from storage, the SessionStore would need to handle
    /// the actual storage deletion.
    pub fn session_destroy(&self) {
        if let Some(session) = self.session() {
            session.destroy();
        }
        self.session_destroyed.store(true, Ordering::Relaxed);
    }

    /// Whether the current session should be destroyed in the outbound phase.
    pub fn is_session_destroyed(&self) -> bool {
        self.session_destroyed.load(Ordering::Relaxed)
    }

    pub fn session_remove(&self, key: &str) -> Option<Value> {
        self.session()?.remove(key)
    }

    // Total.js compatibility convenience methods

    /// Get cookie value by name (Total.js: controller.cookie)
    pub fn cookie(&self, name: &str) -> Option<String> {
        self.req.cookie(name)
    }

    /// Get host from Host header (Total.js: controller.host)
    pub fn host(&self) -> Option<&str> {
        self.req.host()
    }

    /// Get hostname with optional path (Total.js: controller.hostname)
    pub fn hostname(&self, path: Option<&str>) -> String {
        self.req.hostname(path)
    }

    /// Get request path from URI (Total.js: controller.path)
    pub fn path(&self) -> &str {
        self.req.path()
    }

    /// Get file extension from path (Total.js: controller.extension)
    pub fn extension(&self) -> Option<&str> {
        self.req.extension()
    }

    /// Check if request is authorized (Total.js: controller.isAuthorized)
    pub fn is_authorized(&self) -> bool {
        self.req.is_authorized()
    }

    /// Get authorization header (Total.js: controller.authorization)
    pub fn authorization(&self) -> Option<&str> {
        self.req.authorization()
    }

    /// Check if request is from a proxy (Total.js: controller.isProxy)
    pub fn is_proxy(&self) -> bool {
        self.req.is_proxy()
    }

    /// Check if request is for a static file (Total.js: controller.isStaticFile)
    pub fn is_static_file(&self) -> bool {
        self.req.is_static_file()
    }

    /// Get subdomain from host (Total.js: controller.subdomain)
    pub fn subdomain(&self) -> Option<String> {
        self.req.subdomain()
    }

    /// Get path segments as array (Total.js: controller.split)
    pub fn split(&self) -> Vec<&str> {
        self.req.split()
    }

    /// Generate or retrieve CSRF token (Total.js: controller.csrf)
    pub fn csrf(&self) -> String {
        self.generate_csrf(None).unwrap_or_default()
    }

    /// Verify CSRF token for the current request (one-time use)
    pub fn verify_csrf(&mut self, token_id: Option<&str>) -> Result<bool> {
        let token_id = token_id.unwrap_or("_csrf_token");

        // Get submitted token from request first (needs mutable self)
        let submitted_token = self.get_submitted_csrf_token_with_id(token_id);

        // Require session for CSRF
        let session = match self.session() {
            Some(s) => s,
            None => return Ok(false),
        };

        // Get stored token data from session
        let token_data: Option<serde_json::Value> = session.get(token_id);

        let token_data = match token_data {
            Some(data) => data,
            None => return Ok(false),
        };

        // Extract token and expiration
        let stored_token = token_data.get("token").and_then(|v| v.as_str());
        let valid_to = token_data.get("valid_to").and_then(|v| v.as_u64());

        if stored_token.is_none() || valid_to.is_none() {
            return Ok(false);
        }

        // Check expiration
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| Error::internal(format!("System time error: {}", e)))?
            .as_secs();

        if now > valid_to.unwrap() {
            // Token expired, remove it
            let _ = session.remove(token_id);
            return Ok(false);
        }

        let valid = match submitted_token {
            Some(submitted) => {
                // Constant-time comparison to prevent timing attacks
                constant_time_eq(stored_token.unwrap().as_bytes(), submitted.as_bytes())
            }
            None => false,
        };

        // IMPORTANT: Destroy token after verification (one-time use)
        if valid {
            let _ = session.remove(token_id);
        }

        Ok(valid)
    }

    /// Generate CSRF token with optional ID and expiration
    pub fn generate_csrf(&self, token_id: Option<&str>) -> Result<String> {
        let token_id = token_id.unwrap_or("_csrf_token");
        let token = Request::generate_csrf_token();

        // Require session for CSRF
        let session = self.require_session()?;

        // Store token with expiration timestamp (1 hour validity)
        use std::time::{SystemTime, UNIX_EPOCH};
        let valid_to = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| Error::internal(format!("System time error: {}", e)))?
            .as_secs()
            + 3600; // 1 hour validity

        // Store as JSON object with token and expiration
        let token_data = serde_json::json!({
            "token": token,
            "valid_to": valid_to
        });

        session.set(token_id, token_data)?;
        Ok(token)
    }

    /// Get submitted CSRF token from request with custom token ID support
    fn get_submitted_csrf_token_with_id(&mut self, token_id: &str) -> Option<String> {
        // Try X-CSRF-Token header first
        if let Some(token) = self.req.headers.get("x-csrf-token") {
            return Some(token.clone());
        }

        // Try form data if POST request (uses cached form data)
        if matches!(
            self.req.method.as_str(),
            "POST" | "PUT" | "PATCH" | "DELETE"
        ) {
            if let Ok(form_data) = self.body_form() {
                // Try token_id field
                if let Some(token) = form_data.get(token_id) {
                    return Some(token.clone());
                }
                // Try _token field for default
                if token_id == "_csrf_token" {
                    if let Some(token) = form_data.get("_token") {
                        return Some(token.clone());
                    }
                }
            }
        }

        None
    }
}

/// Constant-time comparison to prevent timing attacks
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut result = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        result |= x ^ y;
    }
    result == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::Session;

    fn create_test_context() -> Context {
        let mut request = Request::new("GET", "/test", "1.1");
        request.query.insert("page".to_string(), "2".to_string());
        request
            .query
            .insert("active".to_string(), "true".to_string());
        request
            .query
            .insert("search".to_string(), "rust".to_string());
        request.params.insert("id".to_string(), "123".to_string());
        request
            .params
            .insert("slug".to_string(), "test-post".to_string());

        let views = Arc::new(ViewEngine::new());
        Context::new(request, views)
    }

    #[test]
    fn test_require_session_and_auth_return_401_errors() {
        let ctx = create_test_context();

        let err = ctx.require_session().err().unwrap();
        assert_eq!(err.status_code(), 401);

        let err = ctx.require_auth().err().unwrap();
        assert_eq!(err.status_code(), 401);

        let mut ctx = create_test_context();
        ctx.set_session(Some(Arc::new(Session::new("sess-1"))));
        let err = ctx.require_auth().err().unwrap();
        assert_eq!(err.status_code(), 401);
    }

    #[test]
    fn test_logout_marks_session_for_destruction() {
        let mut ctx = create_test_context();
        ctx.set_session(Some(Arc::new(Session::new("sess-1"))));

        ctx.logout().unwrap();

        assert!(ctx.is_session_destroyed());
        assert!(ctx.session().unwrap().is_destroyed());
    }

    #[test]
    fn test_csrf_verification_rejects_query_tokens() {
        let mut ctx = create_test_context();
        ctx.set_session(Some(Arc::new(Session::new("sess-1"))));

        let token = ctx.generate_csrf(None).unwrap();
        ctx.req.method = "POST".to_string();
        ctx.req
            .query
            .insert("_csrf_token".to_string(), token.clone());

        assert!(!ctx.verify_csrf(None).unwrap());
    }

    #[test]
    fn test_header_lookup_is_case_insensitive() {
        let mut request = Request::new("GET", "/test", "1.1");
        request
            .headers
            .insert("content-type".to_string(), "application/json".to_string());

        let views = Arc::new(ViewEngine::new());
        let ctx = Context::new(request, views);

        assert_eq!(ctx.header("Content-Type"), Some("application/json"));
        assert_eq!(ctx.header("content-type"), Some("application/json"));
    }

    #[test]
    fn test_request_data_returns_error_for_invalid_json() {
        let mut request = Request::new("POST", "/test", "1.1");
        request
            .headers
            .insert("content-type".to_string(), "application/json".to_string());
        request.set_body(br#"{"broken": }"#.to_vec());

        let views = Arc::new(ViewEngine::new());
        let mut ctx = Context::new(request, views);

        assert!(ctx.request_data().is_err());
    }

    #[test]
    fn test_redirect_replaces_stale_content_headers() {
        let mut ctx = create_test_context();
        ctx.text("hello").unwrap();
        ctx.add_header("X-Test", "keep");
        ctx.redirect("/next").unwrap();

        let response = ctx.get_response().unwrap();
        assert_eq!(response.status, StatusCode::FOUND);
        assert!(response
            .headers
            .iter()
            .any(|(k, v)| k == "Location" && v == "/next"));
        assert!(response
            .headers
            .iter()
            .any(|(k, v)| k == "X-Test" && v == "keep"));
        assert!(!response
            .headers
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("Content-Type")));
    }

    #[test]
    fn test_query_methods() {
        let ctx = create_test_context();

        // Test mandatory query methods
        assert_eq!(ctx.str_query("page").unwrap(), "2");
        assert_eq!(ctx.int_query("page").unwrap(), 2);
        assert_eq!(ctx.bool_query("active").unwrap(), true);

        // Test missing required query param
        assert!(ctx.str_query("missing").is_err());
        assert!(ctx.int_query("missing").is_err());

        // Test optional query methods
        assert_eq!(ctx.str_query_or("page", "1"), "2");
        assert_eq!(ctx.int_query_or("page", 1), 2);
        assert_eq!(ctx.bool_query_or("active", false), true);

        // Test defaults for missing params
        assert_eq!(ctx.str_query_or("missing", "default"), "default");
        assert_eq!(ctx.int_query_or("missing", 99), 99);
        assert_eq!(ctx.bool_query_or("missing", true), true);
    }

    #[test]
    fn test_param_methods() {
        let ctx = create_test_context();

        // Test mandatory param methods
        assert_eq!(ctx.str_param("id").unwrap(), "123");
        assert_eq!(ctx.int_param("id").unwrap(), 123);
        assert_eq!(ctx.str_param("slug").unwrap(), "test-post");

        // Test missing required param
        assert!(ctx.str_param("missing").is_err());
        assert!(ctx.int_param("missing").is_err());

        // Test optional param methods
        assert_eq!(ctx.str_param_or("id", "0"), "123");
        assert_eq!(ctx.int_param_or("id", 0), 123);
        assert_eq!(ctx.str_param_or("missing", "default"), "default");
        assert_eq!(ctx.int_param_or("missing", 42), 42);
    }

    #[test]
    fn test_bool_parsing() {
        let mut request = Request::new("GET", "/test", "1.1");
        request.query.insert("yes".to_string(), "yes".to_string());
        request.query.insert("one".to_string(), "1".to_string());
        request.query.insert("on".to_string(), "on".to_string());
        request.query.insert("true".to_string(), "true".to_string());
        request
            .query
            .insert("false".to_string(), "false".to_string());
        request.query.insert("zero".to_string(), "0".to_string());

        let views = Arc::new(ViewEngine::new());
        let ctx = Context::new(request, views);

        assert_eq!(ctx.bool_query("yes").unwrap(), true);
        assert_eq!(ctx.bool_query("one").unwrap(), true);
        assert_eq!(ctx.bool_query("on").unwrap(), true);
        assert_eq!(ctx.bool_query("true").unwrap(), true);
        assert_eq!(ctx.bool_query("false").unwrap(), false);
        assert_eq!(ctx.bool_query("zero").unwrap(), false);
    }

    #[test]
    fn test_error_messages() {
        let ctx = create_test_context();

        // Check error messages
        let err = ctx.str_query("missing").unwrap_err();
        assert!(err
            .to_string()
            .contains("Query parameter 'missing' is required"));

        let err = ctx.int_param("slug").unwrap_err();
        assert!(err
            .to_string()
            .contains("Route parameter 'slug' must be a valid integer"));
    }

    #[test]
    fn test_source_first_query_methods() {
        let ctx = create_test_context();

        // Test source-first query methods (canonical API)
        assert_eq!(ctx.query_str("page").unwrap(), "2");
        assert_eq!(ctx.query_int("page").unwrap(), 2);
        assert_eq!(ctx.query_bool("active").unwrap(), true);

        // Test missing required query param
        assert!(ctx.query_str("missing").is_err());
        assert!(ctx.query_int("missing").is_err());

        // Test optional query methods
        assert_eq!(ctx.query_str_or("page", "1"), "2");
        assert_eq!(ctx.query_int_or("page", 1), 2);
        assert_eq!(ctx.query_bool_or("active", false), true);

        // Test defaults for missing params
        assert_eq!(ctx.query_str_or("missing", "default"), "default");
        assert_eq!(ctx.query_int_or("missing", 99), 99);
        assert_eq!(ctx.query_bool_or("missing", true), true);
    }

    #[test]
    fn test_source_first_param_methods() {
        let ctx = create_test_context();

        // Test source-first param methods (canonical API)
        assert_eq!(ctx.param_str("id").unwrap(), "123");
        assert_eq!(ctx.param_int("id").unwrap(), 123);
        assert_eq!(ctx.param_str("slug").unwrap(), "test-post");

        // Test missing required param
        assert!(ctx.param_str("missing").is_err());
        assert!(ctx.param_int("missing").is_err());

        // Test optional param methods
        assert_eq!(ctx.param_str_or("id", "0"), "123");
        assert_eq!(ctx.param_int_or("id", 0), 123);
        assert_eq!(ctx.param_str_or("missing", "default"), "default");
        assert_eq!(ctx.param_int_or("missing", 42), 42);
    }

    #[test]
    fn test_source_first_body_methods() {
        let mut request = Request::new("POST", "/test", "1.1");
        request.headers.insert(
            "content-type".to_string(),
            "application/x-www-form-urlencoded".to_string(),
        );

        let form_data = "name=Alice&age=30&active=true";
        request.set_body(form_data.as_bytes().to_vec());

        let views = Arc::new(ViewEngine::new());
        let mut ctx = Context::new(request, views);

        // Test source-first body methods (canonical API)
        assert_eq!(ctx.body_str("name").unwrap(), "Alice");
        assert_eq!(ctx.body_int("age").unwrap(), 30);
        assert_eq!(ctx.body_bool("active").unwrap(), true);

        // Test missing required body field
        assert!(ctx.body_str("missing").is_err());
        assert!(ctx.body_int("missing").is_err());

        // Test optional body methods
        assert_eq!(ctx.body_str_or("name", "Bob"), "Alice");
        assert_eq!(ctx.body_int_or("age", 0), 30);
        assert_eq!(ctx.body_bool_or("active", false), true);
        assert_eq!(ctx.body_str_or("missing", "default"), "default");
        assert_eq!(ctx.body_int_or("missing", 99), 99);
        assert_eq!(ctx.body_bool_or("missing", true), true);
    }

    #[test]
    fn test_full_body_with_form_data() {
        let mut request = Request::new("POST", "/test", "1.1");
        request.headers.insert(
            "content-type".to_string(),
            "application/x-www-form-urlencoded".to_string(),
        );

        // Simulate form data
        let form_data = "name=John+Doe&age=30&active=true&tags[]=rust&tags[]=web";
        request.set_body(form_data.as_bytes().to_vec());

        let views = Arc::new(ViewEngine::new());
        let mut ctx = Context::new(request, views);

        // Get full body as JSON
        let json = ctx.full_body().unwrap();

        // Check conversion to JSON
        assert_eq!(json["name"], "John Doe");
        assert_eq!(json["age"], 30);
        assert_eq!(json["active"], true);

        // Check if tags exists and is an array (brackets are removed during parsing)
        assert!(json["tags"].is_array());
        let tags = json["tags"].as_array().unwrap();
        assert_eq!(tags.len(), 2);
        assert_eq!(tags[0], "rust");
        assert_eq!(tags[1], "web");
    }

    #[test]
    fn test_full_body_with_json() {
        let mut request = Request::new("POST", "/test", "1.1");
        request
            .headers
            .insert("content-type".to_string(), "application/json".to_string());

        let json_data = r#"{"name":"Jane Doe","age":25,"active":false}"#;
        request.set_body(json_data.as_bytes().to_vec());

        let views = Arc::new(ViewEngine::new());
        let mut ctx = Context::new(request, views);

        // Get full body as JSON
        let json = ctx.full_body().unwrap();

        assert_eq!(json["name"], "Jane Doe");
        assert_eq!(json["age"], 25);
        assert_eq!(json["active"], false);
    }

    #[test]
    fn test_full_body_with_text() {
        let mut request = Request::new("POST", "/test", "1.1");
        request
            .headers
            .insert("content-type".to_string(), "text/plain".to_string());

        let text_data = "Plain text content";
        request.set_body(text_data.as_bytes().to_vec());

        let views = Arc::new(ViewEngine::new());
        let mut ctx = Context::new(request, views);

        // Get full body - should wrap text in JSON string
        let json = ctx.full_body().unwrap();
        assert_eq!(
            json,
            serde_json::Value::String("Plain text content".to_string())
        );
    }

    #[test]
    fn test_full_body_with_json_text() {
        let mut request = Request::new("POST", "/test", "1.1");
        request
            .headers
            .insert("content-type".to_string(), "text/plain".to_string());

        // Text that happens to be valid JSON
        let json_text = r#"{"key": "value"}"#;
        request.set_body(json_text.as_bytes().to_vec());

        let views = Arc::new(ViewEngine::new());
        let mut ctx = Context::new(request, views);

        // Non-JSON content types stay as strings even if the payload looks like JSON.
        let json = ctx.full_body().unwrap();
        assert_eq!(json, serde_json::Value::String(json_text.to_string()));
    }

    #[test]
    fn test_raw_body() {
        let mut request = Request::new("POST", "/test", "1.1");
        request
            .headers
            .insert("content-type".to_string(), "text/xml".to_string());

        let xml_data = "<root><item>Test</item></root>";
        request.set_body(xml_data.as_bytes().to_vec());

        let views = Arc::new(ViewEngine::new());
        let ctx = Context::new(request, views);

        // Get raw body
        let raw = ctx.raw_body();
        assert_eq!(raw, xml_data);
    }

    #[test]
    fn test_form_to_json_conversion() {
        let mut form_data = HashMap::new();
        form_data.insert("string".to_string(), FormValue::Single("text".to_string()));
        form_data.insert("number".to_string(), FormValue::Single("42".to_string()));
        form_data.insert("float".to_string(), FormValue::Single("3.14".to_string()));
        form_data.insert(
            "bool_true".to_string(),
            FormValue::Single("true".to_string()),
        );
        form_data.insert(
            "bool_false".to_string(),
            FormValue::Single("false".to_string()),
        );
        form_data.insert(
            "array".to_string(),
            FormValue::Multiple(vec!["1".to_string(), "2".to_string(), "3".to_string()]),
        );

        let json = Context::form_to_json(&form_data);

        assert_eq!(json["string"], "text");
        assert_eq!(json["number"], 42);
        assert_eq!(json["float"], 3.14);
        assert_eq!(json["bool_true"], true);
        assert_eq!(json["bool_false"], false);

        let array = json["array"].as_array().unwrap();
        assert_eq!(array.len(), 3);
        assert_eq!(array[0], 1);
        assert_eq!(array[1], 2);
        assert_eq!(array[2], 3);
    }
}
