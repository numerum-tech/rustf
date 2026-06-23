# Uploads Directory

RustF stores uploaded and downloadable private files under `private/uploads/` by default.

## 🤖 AI Agent Quick Reference

**Purpose**: User-uploaded files and runtime-generated private assets  
**Filesystem Root**: `private/uploads/` by default  
**Public URL**: None by default; expose files explicitly through controller routes  
**Security**: Validate file types, sizes, ownership, and access rules before serving  
**Backup**: Include in backups, exclude from version control

## Directory Layout

```text
private/
└── uploads/
    ├── .gitkeep
    ├── avatars/
    ├── documents/
    ├── images/
    ├── media/
    └── exports/
```

`rustf-cli new` creates `private/` and `private/uploads/` automatically.

## Config

```toml
[private]
directory = "private"

[uploads]
directory = "uploads"   # Resolved under [private].directory by default
max_file_size = 10485760
max_files = 5
allowed_extensions = []
create_directories = true
```

## Upload Form Example

```rust
use rustf::prelude::*;

pub fn install() -> Vec<Route> {
    routes![
        GET "/upload" => upload_form,
        POST "/upload" => handle_upload,
        GET "/download/{path}" => download_file,
    ]
}

async fn upload_form(ctx: &mut Context) -> Result<()> {
    let csrf_token = ctx.generate_csrf(None)?;

    ctx.view("uploads/form", json!({
        "title": "Upload File",
        "csrf_token": csrf_token
    }))
    .await
}

async fn handle_upload(ctx: &mut Context) -> Result<()> {
    if !ctx.verify_csrf(None)? {
        ctx.flash_error("Invalid security token")?;
        ctx.redirect("/upload")?;
        return Ok(());
    }

    let files = ctx.files()?;
    if files.is_empty() {
        ctx.flash_error("No files uploaded")?;
        ctx.redirect("/upload")?;
        return Ok(());
    }

    // Persist files under private/uploads/... using your application policy.
    ctx.flash_success("Upload complete")?;
    ctx.redirect("/upload")?;
    Ok(())
}
```

Template:

```html
<form action="/upload" method="POST" enctype="multipart/form-data">
    <input type="hidden" name="_csrf_token" value="@{csrf_token}">
    <input type="file" name="files" multiple>
    <button type="submit">Upload</button>
</form>
```

## Downloading Files

Use controller routes plus the private-rooted helpers:

```rust
async fn download_file(ctx: &mut Context) -> Result<()> {
    let relative_path = ctx.param("path").unwrap_or_default();

    // Authorize access before sending the file.
    // file_download() resolves relative paths from private/
    ctx.file_download(&relative_path, None)?;
    Ok(())
}

async fn preview_file(ctx: &mut Context) -> Result<()> {
    let relative_path = ctx.param("path").unwrap_or_default();
    ctx.file_inline(&relative_path, None)?;
    Ok(())
}
```

If you must serve a file outside `private/`, use the explicit external-access helpers:

- `file_download_from(...)`
- `file_inline_from(...)`

Do not pass unsanitized user input into those external-path variants.

## Recommendations

1. Keep uploads in `private/uploads/`, not `public/`.
2. Serve files through controllers so you can enforce authorization.
3. Use `ctx.generate_csrf(None)?` and `ctx.verify_csrf(None)?` for upload forms.
4. Validate filename, MIME type, extension, and size before persisting.
5. Store only relative paths in your database, not absolute filesystem paths.
