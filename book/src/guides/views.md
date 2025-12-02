# RustF View & Template Engine Layer

## Overview

RustF provides a powerful view system with Total.js as the built-in template engine. The framework is optimized for Total.js templates, providing a familiar syntax for Total.js developers and direct integration with framework features.

## Template Engine

### Total.js (Built-in)
- **No feature flag required** - Always available
- **Syntax**: Uses `@{...}` for expressions
- **Compatible with Total.js v4** - Familiar syntax for Total.js developers
- **Optimized for RustF** - Direct integration with framework features
- **Powerful features** - Conditionals, loops, layouts, sections, helpers
- **Auto-escaping** - HTML is escaped by default for security

## Configuration

Configure your template engine in `config.toml`:

```toml
[views]
directory = "views"                # Template directory
default_layout = "layouts/default"  # Default layout template
cache_enabled = false              # Template caching (dev: false, prod: true)
extension = "html"                 # File extension for templates
# engine is always "totaljs" - no other options available
storage = "filesystem"             # Choose: "filesystem" or "embedded"
default_root = ""                  # Base URL path for deployment (e.g., "/app")
```

## Basic Usage

### In Controllers

```rust
use rustf::prelude::*;

#[rustf::install]
impl HomeController {
    #[route(GET, "/")]
    pub async fn index(ctx: Context) -> Result<Response> {
        // Render a template with data
        ctx.view("home/index", json!({
            "title": "Welcome",
            "message": "Hello from RustF!",
            "features": vec!["Fast", "Safe", "Productive"]
        }))
    }
    
    #[route(GET, "/about")]
    pub async fn about(mut ctx: Context) -> Result<Response> {
        // Render with custom layout
        ctx.layout("layouts/special")
            .view("about", json!({
                "title": "About Us"
            }))
    }
    
    #[route(GET, "/standalone")]
    pub async fn standalone(mut ctx: Context) -> Result<Response> {
        // Render without layout
        ctx.layout("")  // Empty string = no layout
            .view("standalone", json!({
                "content": "This page has no layout"
            }))
    }
}
```

### Context Methods

```rust
// Basic template rendering
ctx.view("template_name", data)

// Set custom layout (requires mut ctx)
ctx.layout("layouts/special")
    .view("template", data)

// Render without layout
ctx.layout("")  // Empty string = no layout
    .view("template", data)

// Use default layout from config (no need to call layout())
ctx.view("template", data)  // Uses config.views.default_layout

// Render HTML string directly
ctx.html("<h1>Direct HTML</h1>")

// JSON response
ctx.json(data)

// Text response
ctx.text("Plain text response")
```

## Total.js Template Syntax

### Variables
```html
<!-- Model/View data access -->
<h1>@{model.title}</h1>
<h1>@{M.title}</h1>  <!-- Alias for model -->

<!-- Nested properties -->
<p>@{model.user.name}</p>
<p>@{M.user.name}</p>  <!-- Alias -->

<!-- With default value -->
<!-- Default values using || operator -->
<p>@{model.description || 'No description'}</p>
<p>@{user.avatar || '/images/default-avatar.png'}</p>
<span>@{settings.theme || 'light'}</span>
```

**Using `||` for Fallback Values:**

The `||` (OR) operator provides default values when the left operand is falsy (null, undefined, empty string, 0, false, empty array/object):

```html
<!-- String defaults -->
<p>Bio: @{user.bio || "No bio available"}</p>

<!-- Numeric defaults -->
<p>Items per page: @{settings.pageSize || 20}</p>

<!-- Chaining multiple fallbacks -->
<h1>@{custom_title || page.title || "Untitled Page"}</h1>
```

### Conditionals

#### Basic If/Else
```html
@{if model.logged_in}
    <p>Welcome, @{model.username}!</p>
@{else}
    <p>Please log in</p>
@{fi}

<!-- Inline conditional -->
<div class="@{if M.active}active@{else}inactive@{fi}">
```

#### Comparison Operators
```html
<!-- Equality -->
@{if user.role == "admin"}
    <p>Admin Dashboard</p>
@{fi}

@{if status != "deleted"}
    <p>Active Item</p>
@{fi}

<!-- Strict equality (type-sensitive) -->
@{if count === 0}
    <p>Exactly zero</p>
@{fi}

<!-- Numeric comparison -->
@{if age >= 18}
    <p>Adult content</p>
@{fi}

@{if stock > 0}
    <button>Add to Cart</button>
@{else}
    <span>Out of Stock</span>
@{fi}

@{if price <= budget}
    <span class="affordable">Within Budget</span>
@{fi}
```

**Supported Comparison Operators:**
- `==` - Loose equality (type coercion)
- `!=` - Loose inequality
- `===` - Strict equality (no type coercion)
- `!==` - Strict inequality
- `<` - Less than
- `>` - Greater than
- `<=` - Less than or equal
- `>=` - Greater than or equal

#### Logical Operators

**AND (`&&`)** - Both conditions must be true:
```html
@{if user.is_admin && user.is_active}
    <a href="/admin">Admin Panel</a>
@{fi}

@{if age >= 18 && has_id}
    <p>Access granted</p>
@{fi}
```

**OR (`||`)** - At least one condition must be true:
```html
@{if user.is_admin || user.is_moderator}
    <div class="staff-tools">Staff Area</div>
@{fi}

@{if is_weekend || is_holiday}
    <span>🎉 Special Hours</span>
@{fi}
```

**Complex Conditions** - Combine with parentheses:
```html
@{if (age >= 18 && verified) || is_staff}
    <p>Access granted</p>
@{fi}

@{if (is_member && points > 100) || is_vip}
    <span class="badge">Premium</span>
@{fi}
```

#### Negation with `!`
```html
<!-- NOT operator -->
@{if !user.is_banned}
    <p>Welcome back!</p>
@{fi}

@{if !is_empty}
    <ul>@{foreach item in items}...@{end}</ul>
@{fi}

<!-- Negating expressions -->
@{if !(status == "deleted" || status == "archived")}
    <button>Edit</button>
@{fi}
```

#### Else-If Chains with `@{elif}`
```html
@{if score > 90}
    <span class="grade-a">A - Excellent!</span>
@{elif score > 80}
    <span class="grade-b">B - Good Job</span>
@{elif score > 70}
    <span class="grade-c">C - Fair</span>
@{elif score > 60}
    <span class="grade-d">D - Needs Improvement</span>
@{else}
    <span class="grade-f">F - Failed</span>
@{fi}
```

**Multiple elif branches:**
```html
@{if role == "owner"}
    <p>Full Access</p>
@{elif role == "admin"}
    <p>Administrative Access</p>
@{elif role == "moderator"}
    <p>Moderation Access</p>
@{elif role == "member"}
    <p>Member Access</p>
@{else}
    <p>Guest Access</p>
@{fi}
```

#### Truthy/Falsy Values
Values are evaluated as truthy or falsy in conditionals:

**Falsy values:**
- `false`
- `null`
- `0`
- `""` (empty string)
- `[]` (empty array)
- `{}` (empty object)

**Truthy values:**
- `true`
- Any non-zero number
- Any non-empty string
- Any non-empty array
- Any non-empty object

```html
<!-- Check if value exists and is truthy -->
@{if user.email}
    <p>Email: @{user.email}</p>
@{fi}

<!-- Check if array has items -->
@{if products}
    <p>@{products.length} products available</p>
@{fi}
```

### Loops
```html
<!-- Array iteration -->
@{foreach item in model.items}
    <li>@{item.name} - @{item.price}</li>
@{end}

<!-- With index (using M alias) -->
@{foreach item, index in M.items}
    <li>@{index}: @{item}</li>
@{end}
```

### Escaping
```html
<!-- Auto-escaped (safe by default) -->
<p>@{model.user_input}</p>

<!-- Raw HTML (unescaped) -->
<div>@{!M.html_content}</div>
```

### Expressions & Operators

The Total.js template engine supports rich expressions with multiple operator types, allowing for complex logic directly in templates.

#### Operator Reference

| Category | Operators | Description | Example |
|----------|-----------|-------------|---------|
| **Comparison** | `==`, `!=` | Loose equality/inequality (with type coercion) | `@{if age == "18"}` |
| | `===`, `!==` | Strict equality/inequality (no type coercion) | `@{if count === 0}` |
| | `<`, `>` | Less than, greater than | `@{if price > 100}` |
| | `<=`, `>=` | Less/greater than or equal | `@{if stock <= 5}` |
| **Logical** | `&&` | AND - both must be true | `@{if a && b}` |
| | `\|\|` | OR - at least one must be true | `@{if a \|\| b}` |
| **Unary** | `!` | NOT - negates boolean | `@{if !is_deleted}` |
| | `-` | Negation - negates number | `@{-value}` |
| **Arithmetic** | `+` | Addition / String concatenation | `@{price + tax}` |
| | `-` | Subtraction | `@{total - discount}` |
| | `*` | Multiplication | `@{price * quantity}` |
| | `/` | Division | `@{total / count}` |
| | `%` | Modulo (remainder) | `@{index % 2}` |

#### Operator Precedence

Operators are evaluated in the following order (highest to lowest precedence):

1. **Parentheses** `( )` - Grouping (evaluated first)
2. **Unary** `!`, `-` - NOT, negation
3. **Multiplication/Division** `*`, `/`, `%` - Arithmetic
4. **Addition/Subtraction** `+`, `-` - Arithmetic
5. **Comparison** `<`, `>`, `<=`, `>=` - Relational
6. **Equality** `==`, `!=`, `===`, `!==` - Equality checks
7. **Logical AND** `&&` - Boolean AND
8. **Logical OR** `||` - Boolean OR (evaluated last)

**Examples:**
```html
<!-- Precedence without parentheses -->
@{if x + 5 > 10 && y < 20}
    <!-- Evaluates as: ((x + 5) > 10) && (y < 20) -->
@{fi}

<!-- Using parentheses to control precedence -->
@{if (a || b) && c}
    <!-- OR is evaluated before AND due to parentheses -->
@{fi}
```

#### Arithmetic Operations

```html
<!-- Basic arithmetic -->
<p>Subtotal: $@{price * quantity}</p>
<p>Tax: $@{subtotal * 0.08}</p>
<p>Total: $@{subtotal + (subtotal * 0.08)}</p>

<!-- Complex calculations -->
<p>Price per unit: $@{total_price / item_count}</p>
<p>Discount: @{(original_price - sale_price) / original_price * 100}%</p>

<!-- Modulo for alternating styles -->
@{foreach item, index in items}
    <tr class="@{if index % 2 == 0}even@{else}odd@{fi}">
        <td>@{item.name}</td>
    </tr>
@{end}
```

#### String Concatenation

```html
<!-- Concatenate strings with + -->
<p>@{first_name + " " + last_name}</p>
<p>@{"Hello, " + user.name + "!"}</p>

<!-- Build URLs -->
<a href="@{"/products/" + product.id}">View Product</a>

<!-- Combine with variables -->
<p>@{greeting + " " + M.username}</p>
```

#### Default Values with `||`

The `||` operator provides fallback values when the left side is falsy:

```html
<!-- Simple default -->
<p>@{user.bio || "No bio provided"}</p>

<!-- Nested properties with default -->
<p>@{user.profile.avatar || "/images/default-avatar.png"}</p>

<!-- Numeric defaults -->
<p>Items per page: @{settings.page_size || 20}</p>

<!-- Chain multiple defaults -->
<p>@{custom_title || default_title || "Untitled"}</p>
```

#### Complex Expressions

```html
<!-- Nested conditionals in expressions -->
<p class="@{if score >= 90}excellent@{elif score >= 70}good@{else}poor@{fi}">
    Score: @{score}
</p>

<!-- Mathematical expressions in conditionals -->
@{if (price * quantity) > budget}
    <span class="warning">Over budget!</span>
@{fi}

<!-- Combining multiple operators -->
@{if ((total - discount) * tax_rate) < 100 && is_member}
    <span>Free shipping!</span>
@{fi}

<!-- Expression in attribute values -->
<div data-score="@{points * 10}" 
     class="level-@{if points > 100}high@{else}low@{fi}">
</div>
```

#### Grouping with Parentheses

Use parentheses to control evaluation order and clarify complex expressions:

```html
<!-- Without parentheses (may be unclear) -->
@{if a && b || c && d}  <!-- Evaluates as: (a && b) || (c && d) -->

<!-- With parentheses (explicit) -->
@{if (a && b) || (c && d)}  <!-- Same result, but clearer intent -->

<!-- Change evaluation order -->
@{if a && (b || c) && d}  <!-- OR evaluated before outer ANDs -->

<!-- Arithmetic grouping -->
<p>@{(price + shipping) * (1 + tax_rate)}</p>
```

#### Practical Examples

**Shopping Cart Total:**
```html
@{var subtotal = 0}
@{foreach item in cart.items}
    @{var item_total = item.price * item.quantity}
    @{subtotal = subtotal + item_total}
    <tr>
        <td>@{item.name}</td>
        <td>@{item.quantity} × $@{item.price}</td>
        <td>$@{item_total}</td>
    </tr>
@{end}
<tr class="total">
    <td colspan="2">Subtotal:</td>
    <td>$@{subtotal}</td>
</tr>
<tr>
    <td colspan="2">Tax (@{tax_rate * 100}%):</td>
    <td>$@{subtotal * tax_rate}</td>
</tr>
<tr class="grand-total">
    <td colspan="2">Total:</td>
    <td>$@{subtotal + (subtotal * tax_rate)}</td>
</tr>
```

**Pagination Logic:**
```html
@{var total_pages = Math.ceil(total_items / items_per_page)}
@{var has_prev = current_page > 1}
@{var has_next = current_page < total_pages}

<div class="pagination">
    @{if has_prev}
        <a href="?page=@{current_page - 1}">Previous</a>
    @{fi}
    
    <span>Page @{current_page} of @{total_pages}</span>
    
    @{if has_next}
        <a href="?page=@{current_page + 1}">Next</a>
    @{fi}
</div>
```

**Access Control:**
```html
@{var can_edit = (user.id === post.author_id) || user.is_admin}
@{var can_delete = user.is_admin || (user.is_moderator && post.flagged)}
@{var can_publish = (user.role === "editor" || user.role === "admin") && !post.published}

@{if can_edit}
    <button onclick="editPost()">Edit</button>
@{fi}

@{if can_delete}
    <button onclick="deletePost()" class="danger">Delete</button>
@{fi}

@{if can_publish}
    <button onclick="publishPost()" class="primary">Publish</button>
@{fi}
```

### Data Access in Templates

#### Global Constants
These are directly accessible without any prefix:
```html
@{url}          <!-- Current request URL -->
@{hostname}     <!-- Server hostname -->
@{root}         <!-- Application root path (from config.default_root) -->
@{index}        <!-- Current loop index (inside foreach) -->
@{csrf_token}   <!-- CSRF token for forms -->
```

#### Data Source Objects
These objects can be accessed entirely or via dot notation for properties:
```html
<!-- Configuration (entire object or properties) -->
@{CONF}               <!-- Entire config object -->
@{CONF.app_name}      <!-- Specific config value -->
@{CONF.default_root}  <!-- Deployment root path -->

<!-- Session (entire object or properties) -->
@{session}            <!-- Entire session object -->
@{session.user_id}    <!-- Specific session value -->
@{session.cart}       <!-- Session cart data -->

<!-- Flash Messages (stored in session, aliased for convenience) -->
@{flash}              <!-- All flash messages -->
@{flash.error}        <!-- Error flash message -->
@{flash.success}      <!-- Success flash message -->
@{flash.info}         <!-- Info flash message -->
@{flash.custom_key}   <!-- Custom flash message -->

<!-- Query parameters (entire object or properties) -->
@{query}              <!-- All query parameters -->
@{query.page}         <!-- Specific query parameter -->
@{query.search}       <!-- Search query parameter -->

<!-- User (entire object or properties) -->
@{user}               <!-- Entire user object -->
@{user.name}          <!-- User's name -->
@{user.email}         <!-- User's email -->

<!-- Context Repository (per-handler) -->
@{repository}         <!-- Entire repository object -->
@{repository.key}     <!-- Specific repository value -->
@{R.key}              <!-- Shorthand for repository.key -->

<!-- Global Repository (application-wide) -->
@{APP.key}            <!-- Global repository value -->
@{MAIN.key}           <!-- Alias for APP.key -->

<!-- Model/View Data (from ctx.view()) -->
@{model}              <!-- Entire model object -->
@{model.key}          <!-- Specific model value -->
@{M}                  <!-- Alias for entire model -->
@{M.key}              <!-- Alias for model.key -->
```

#### Loop Variables
Within `@{foreach}` loops, the iterator variable is directly accessible:
```html
@{foreach product in model.products}
    <!-- 'product' is directly accessible here -->
    <h3>@{product.name}</h3>
    <p>Price: @{product.price}</p>
    <p>Index: @{index}</p>  <!-- Loop index is available -->
@{end}

<!-- Looping over objects -->
@{foreach key, value in CONF}
    <p>Config @{key}: @{value}</p>
@{end}
```

### Built-in Functions
```html
<!-- URL helpers -->
@{url('/path')}         <!-- Generate URL with root prefix -->
@{css('/static/app.css')}  <!-- CSS link tag -->
@{js('/static/app.js')}    <!-- Script tag -->

<!-- Data formatting -->
@{json(data)}           <!-- JSON stringify -->
@{encode(text)}         <!-- URL encode -->
@{escape(html)}         <!-- HTML escape -->

<!-- String manipulation -->
@{upper(text)}          <!-- Uppercase -->
@{lower(text)}          <!-- Lowercase -->
@{trim(text)}           <!-- Trim whitespace -->
@{capitalize(text)}     <!-- Capitalize first letter -->

<!-- Iteration helpers -->
@{range(10)}            <!-- Generate array [0,1,2,3,4,5,6,7,8,9] -->
@{range(1, 11)}         <!-- Generate array [1,2,3,4,5,6,7,8,9,10] -->
@{range(0, 10, 2)}      <!-- Generate array [0,2,4,6,8] with step -->
```

#### Range Function Examples

The `range()` function generates numeric sequences for loops:

```html
<!-- Simple range: 0 to N-1 -->
@{foreach num in range(5)}
    <p>Item @{num}</p>  <!-- Outputs: 0, 1, 2, 3, 4 -->
@{end}

<!-- Range with start and stop -->
@{foreach page in range(1, 6)}
    <a href="/page/@{page}">Page @{page}</a>  <!-- Pages 1-5 -->
@{end}

<!-- Range with custom step -->
@{foreach year in range(2020, 2030, 2)}
    <option value="@{year}">@{year}</option>  <!-- 2020, 2022, 2024, 2026, 2028 -->
@{end}

<!-- Practical example: pagination -->
@{foreach page in range(1, model.totalPages + 1)}
    @{if page == model.currentPage}
        <span class="active">@{page}</span>
    @{else}
        <a href="?page=@{page}">@{page}</a>
    @{fi}
@{end}
```

### Template Partials and Includes

Total.js templates support including other templates (partials) using the `@{view()}` function:

```html
<!-- Include a partial template -->
@{view('partials/header')}

<!-- Include with specific model data -->
@{view('partials/user-card', model.currentUser)}

<!-- Include in loops -->
@{foreach user in model.users}
    @{view('partials/user-card', user)}
@{end}

<!-- Nested partials (partials can include other partials) -->
<!-- In partials/sidebar.html: -->
@{view('partials/user-info', model.user)}
@{view('partials/menu', model.menuItems)}
```

#### Partial Template Example

Create reusable components in `views/partials/`:

```html
<!-- views/partials/user-card.html -->
<div class="user-card">
    <img src="@{model.avatar}" alt="@{model.name}">
    <h3>@{model.name}</h3>
    <p>@{model.email}</p>
    @{if model.isAdmin}
        <span class="badge">Admin</span>
    @{fi}
</div>
```

Use it in your main templates:

```html
<!-- views/users/list.html -->
<div class="users-grid">
    @{foreach user in model.users}
        @{view('partials/user-card', user)}
    @{end}
</div>
```

#### Important Notes

- Partials have access to all global data (repository, session, etc.)
- When passing model data, the partial receives only that data as its model
- Partials are loaded relative to the views directory
- Nested partials are supported (partials can include other partials)
- Currently, array indexing in expressions (e.g., `model.users[0]`) is not supported in the view() function

## Layouts

### Layout System Overview

RustF follows the Total.js pattern for layouts:
- **Default layout**: Configured in `config.toml` as `views.default_layout` (default: `"layouts/default"`)
- **Custom layout**: Use `ctx.layout("path/to/layout")` in controller (requires `mut ctx`)
- **No layout**: Use `ctx.layout("")` with empty string
- **Layout inheritance**: Templates can specify their own layout with `@{layout('path')}`

### Controller-Side Layout Control

```rust
use rustf::prelude::*;

#[rustf::install]
impl PageController {
    // Uses default layout from config
    #[route(GET, "/")]
    pub async fn index(ctx: Context) -> Result<Response> {
        ctx.view("home/index", json!({"title": "Home"}))
    }
    
    // Custom layout
    #[route(GET, "/admin")]
    pub async fn admin(mut ctx: Context) -> Result<Response> {
        ctx.layout("layouts/admin")  // Set custom layout
            .view("admin/dashboard", json!({"user": "Admin"}))
    }
    
    // No layout (standalone page)
    #[route(GET, "/api-doc")]
    pub async fn api_doc(mut ctx: Context) -> Result<Response> {
        ctx.layout("")  // Empty string = no layout
            .view("docs/api", json!({"version": "1.0"}))
    }
}
```

### Total.js Layouts

Define a layout (`views/layouts/default.html`):
```html
<!DOCTYPE html>
<html>
<head>
    <title>@{if model.title}@{model.title} - @{fi}My App</title>
    @{css('/static/css/main.css')}
</head>
<body>
    <nav>
        <a href="@{root}/">Home</a>
        <a href="@{root}/about">About</a>
    </nav>
    
    <main>
        @{body}  <!-- Child template content renders here -->
    </main>
    
    @{js('/static/js/app.js')}
</body>
</html>
```

Use in child template (`views/home/index.html`):
```html
@{layout('layouts/default')}  <!-- Optional: Override controller layout -->

<h1>@{model.title}</h1>
<p>@{M.message}</p>

@{foreach feature in model.features}
    <div>@{feature}</div>
@{end}
```

## Sections

### Section System Overview

Sections allow child templates to define content blocks that parent layouts can render in specific locations. This is the standard pattern for customizing different areas of a layout (head content, sidebar, footer, etc.) from child views.

**Key Concepts**:
- **Child defines sections**: Use `@{section name}...@{end}` in child templates
- **Parent renders sections**: Use `@{section('name')}` in layout templates
- **Optional sections**: Missing sections render as empty (no errors)
- **Multiple sections**: Child can define multiple sections for different layout areas

### Basic Section Usage

**Layout with Section Placeholders** (`views/layouts/main.html`):
```html
<!DOCTYPE html>
<html>
<head>
    <title>@{M.title}</title>
    
    <!-- Child can inject custom styles here -->
    @{section('styles')}
    
    <!-- Child can add meta tags, scripts, etc. -->
    @{section('head')}
</head>
<body>
    <header>
        <!-- Child can customize header -->
        @{section('header')}
    </header>
    
    <main>
        @{body}  <!-- Main content -->
    </main>
    
    <aside>
        <!-- Child can provide sidebar content -->
        @{section('sidebar')}
    </aside>
    
    <footer>
        <!-- Child can customize footer -->
        @{section('footer')}
    </footer>
    
    <!-- Child can inject page-specific scripts -->
    @{section('scripts')}
</body>
</html>
```

**Child Template Defining Sections** (`views/products/detail.html`):
```html
@{section styles}
<link rel="stylesheet" href="/css/products.css">
<style>
    .product-gallery { display: grid; }
</style>
@{end}

@{section head}
<meta property="og:title" content="@{M.product.name}">
<meta property="og:image" content="@{M.product.image}">
@{end}

@{section header}
<h1>@{M.product.name}</h1>
<nav class="breadcrumb">
    <a href="@{root}/">Home</a> &gt;
    <a href="@{root}/products">Products</a> &gt;
    @{M.product.category}
</nav>
@{end}

<!-- Main content (rendered in @{body}) -->
<div class="product-detail">
    <img src="@{M.product.image}" alt="@{M.product.name}">
    <div class="product-info">
        <p class="price">$@{M.product.price}</p>
        <p>@{M.product.description}</p>
    </div>
</div>

@{section sidebar}
<h3>Related Products</h3>
@{foreach item in M.related}
    <div class="related-item">
        <a href="/products/@{item.id}">@{item.name}</a>
    </div>
@{end}
@{end}

@{section scripts}
<script src="/js/product-gallery.js"></script>
<script>
    initProductGallery('@{M.product.id}');
</script>
@{end}
```

### Section Features

**1. Optional Sections** - No errors if section not defined:
```html
<!-- Layout -->
<div class="notifications">
    @{section('alerts')}  <!-- Empty if child doesn't define it -->
</div>

<!-- Child - can choose to define or skip -->
@{section alerts}
<div class="alert">Special offer!</div>
@{end}
```

**2. Conditional Sections**:
```html
<!-- Child template -->
@{if M.show_newsletter}
    @{section footer}
    <div class="newsletter-signup">
        <form>...</form>
    </div>
    @{end}
@{fi}
```

**3. Sections with Dynamic Content**:
```html
<!-- Child template -->
@{section sidebar}
    <h3>Categories</h3>
    @{foreach cat in M.categories}
        <a href="/category/@{cat.slug}">@{cat.name}</a>
    @{end}
    
    <h3>Tags</h3>
    @{foreach tag in M.tags}
        <span class="tag">@{tag}</span>
    @{end}
@{end}
```

**4. Multiple Sections in One Template**:
```html
<!-- A single child view can define many sections -->
@{section meta}...@{end}
@{section styles}...@{end}
@{section header}...@{end}

<!-- Main content here -->

@{section sidebar}...@{end}
@{section footer}...@{end}
@{section scripts}...@{end}
```

### Common Section Patterns

**Meta Tags & SEO**:
```html
<!-- Layout head -->
@{section('meta')}

<!-- Child defines SEO metadata -->
@{section meta}
<meta name="description" content="@{M.description}">
<meta name="keywords" content="@{M.keywords}">
<link rel="canonical" href="@{M.canonical_url}">
@{end}
```

**Page-Specific Styles**:
```html
<!-- Layout head -->
@{section('page_styles')}

<!-- Child adds custom CSS -->
@{section page_styles}
<link rel="stylesheet" href="/css/dashboard.css">
<style>
    .dashboard-widget { margin: 1rem; }
</style>
@{end}
```

**JavaScript Initialization**:
```html
<!-- Layout before </body> -->
@{section('page_scripts')}

<!-- Child adds page logic -->
@{section page_scripts}
<script src="/js/chart.min.js"></script>
<script>
    initCharts(@{json(M.chart_data)});
</script>
@{end}
```

**Breadcrumbs**:
```html
<!-- Layout -->
<nav class="breadcrumbs">
    <a href="/">Home</a>
    @{section('breadcrumbs')}
</nav>

<!-- Child extends breadcrumb trail -->
@{section breadcrumbs}
&gt; <a href="/products">Products</a>
&gt; @{M.category}
@{end}
```

### Section vs @{head} Special Placeholder

RustF provides a special `@{head}` placeholder that automatically renders content from a section named "head":

```html
<!-- These are equivalent: -->
@{section('head')}
@{head}
```

The `@{head}` placeholder is a convenience feature inherited from Total.js for adding content to the HTML `<head>` section. For all other sections, use the explicit `@{section('name')}` syntax.

### Best Practices

1. **Name sections clearly**: Use descriptive names like `page_styles`, `sidebar_menu`, `footer_links`

2. **Document expected sections**: Comment layout templates to show which sections are available:
   ```html
   <!-- Available sections:
        - styles: Custom CSS for this page
        - header: Page-specific header content
        - sidebar: Sidebar widgets
        - scripts: Page-specific JavaScript
   -->
   ```

3. **Keep sections focused**: Each section should have a single, clear purpose

4. **Make sections optional**: Don't require every child to define every section

5. **Use consistent naming**: Establish naming conventions across your app:
   - `page_styles` / `page_scripts` for page-specific assets
   - `meta` for SEO metadata
   - `header` / `footer` / `sidebar` for layout areas

### Example: Complete Blog Post Template

**Layout** (`views/layouts/blog.html`):
```html
<!DOCTYPE html>
<html>
<head>
    <title>@{M.title} - My Blog</title>
    <meta charset="UTF-8">
    @{section('meta')}
    @{css('/css/blog.css')}
    @{section('styles')}
</head>
<body>
    <header class="site-header">
        <h1>My Blog</h1>
        @{section('breadcrumbs')}
    </header>
    
    <div class="content-wrapper">
        <article>
            @{section('article_header')}
            @{body}
        </article>
        
        <aside class="sidebar">
            @{section('sidebar')}
        </aside>
    </div>
    
    <footer>
        @{section('comments')}
        <p>&copy; 2025 My Blog</p>
    </footer>
    
    @{js('/js/blog.js')}
    @{section('scripts')}
</body>
</html>
```

**Blog Post** (`views/blog/post.html`):
```html
@{section meta}
<meta name="description" content="@{M.excerpt}">
<meta name="author" content="@{M.author}">
<meta property="og:image" content="@{M.featured_image}">
@{end}

@{section breadcrumbs}
<a href="/blog">Blog</a> &gt;
<a href="/blog/@{M.category}">@{M.category_name}</a>
@{end}

@{section article_header}
<h1>@{M.title}</h1>
<div class="meta">
    By @{M.author} on @{M.published_date}
</div>
<img src="@{M.featured_image}" alt="@{M.title}">
@{end}

<!-- Main article content -->
<div class="article-body">
    @{M.content}
</div>

@{section sidebar}
<h3>About the Author</h3>
<p>@{M.author_bio}</p>

<h3>Related Posts</h3>
@{foreach post in M.related_posts}
    <a href="/blog/@{post.slug}">@{post.title}</a>
@{end}
@{end}

@{section comments}
<div class="comments">
    <h3>Comments (@{M.comment_count})</h3>
    @{foreach comment in M.comments}
        <div class="comment">
            <strong>@{comment.author}</strong>
            <p>@{comment.text}</p>
        </div>
    @{end}
</div>
@{end}

@{section scripts}
<script src="/js/syntax-highlighter.js"></script>
<script>highlightCode();</script>
@{end}
```

## Storage Options

### Filesystem (Development)
- Templates loaded from disk at runtime
- Supports hot-reloading
- Easy to modify during development
- Configure with: `storage = "filesystem"`

### Embedded (Production)
- Templates compiled into binary
- Faster startup, no file I/O
- Requires `embedded-views` feature
- Configure with: `storage = "embedded"`

## Advanced Features

### Custom Template Functions

Register custom functions for Total.js templates:

```rust
use rustf::views::TotalJsEngine;

// In your app setup
let mut engine = TotalJsEngine::new("views");
engine.register_function("format_price", |args| {
    if let Some(Value::Number(n)) = args.first() {
        Value::String(format!("${:.2}", n.as_f64().unwrap_or(0.0)))
    } else {
        Value::String("$0.00".to_string())
    }
});

// Use in template
// @{format_price(19.99)} => "$19.99"
```

### CSRF Protection

Automatic CSRF token injection:

```rust
// In controller - generate token and render view
ctx.generate_csrf()?;
ctx.view("form", data)
```

```html
<!-- In template - Total.js -->
<form method="POST">
    @{csrf_field}  <!-- Renders hidden input with token -->
    <!-- or manually -->
    <input type="hidden" name="_token" value="@{csrf_token}">
</form>
```

### Repository System

RustF provides two repository systems for sharing data between controllers and views:

#### Context Repository (Per-Request)

The context repository is request-scoped data that controllers can set to share with templates. It's useful for page-specific settings, user permissions, or any data that's specific to the current request.

**Setting in Controllers:**
```rust
// Requires mut ctx for repository operations
async fn dashboard(mut ctx: Context) -> Result<Response> {
    // Set various types of repository data
    ctx.repository_set("current_section", "analytics")
       .repository_set("sidebar_expanded", true)
       .repository_set("user_role", "admin")
       .repository_set("available_actions", json!(["view", "edit", "delete"]))
       .repository_set("theme", json!({
           "primary_color": "#007bff",
           "mode": "dark"
       }));
    
    // Repository data is automatically available in templates
    ctx.view("dashboard/index", json!({
        "title": "Dashboard",
        "stats": get_stats().await?
    }))
}
```

**Accessing in Templates:**
```html
<!-- Using R. prefix (recommended, shorter) -->
@{if R.user_role == 'admin'}
    <div class="admin-panel">
        @{foreach action in R.available_actions}
            <button>@{action}</button>
        @{end}
    </div>
@{fi}

<!-- Using repository. prefix (alternative) -->
<div class="sidebar @{if repository.sidebar_expanded}expanded@{fi}">
    Current Section: @{repository.current_section}
</div>

<!-- Accessing nested data -->
<style>
    :root {
        --primary-color: @{R.theme.primary_color};
    }
    body.@{R.theme.mode} {
        /* Dark mode styles */
    }
</style>
```

#### Global Repository (APP/MAIN)

The global repository is application-wide data shared across all requests. It's typically set during application initialization and contains site-wide settings, application metadata, or shared constants.

**Setting During Application Setup:**
```rust
// In your main.rs or app initialization
use rustf::views::TotalJsEngine;

let engine = TotalJsEngine::new("views");
engine.set_global_repository(json!({
    "site_name": "My Application",
    "version": "2.1.0",
    "copyright": "© 2024 MyCompany",
    "contact_email": "support@example.com",
    "features": {
        "comments": true,
        "notifications": true,
        "api_v2": false
    },
    "social_links": {
        "twitter": "https://twitter.com/myapp",
        "github": "https://github.com/myapp"
    }
}));
```

**Accessing in Templates:**
```html
<!-- Using APP. prefix -->
<footer>
    <p>@{APP.copyright}</p>
    <p>Contact: <a href="mailto:@{APP.contact_email}">@{APP.contact_email}</a></p>
    <p>Version @{APP.version}</p>
</footer>

<!-- Using MAIN. prefix (alias for APP) -->
<title>@{title} - @{MAIN.site_name}</title>

<!-- Checking features -->
@{if APP.features.comments}
    <div class="comments-section">
        <!-- Comments enabled -->
    </div>
@{fi}

<!-- Social links -->
<div class="social">
    @{if APP.social_links.twitter}
        <a href="@{APP.social_links.twitter}">Twitter</a>
    @{fi}
    @{if APP.social_links.github}
        <a href="@{APP.social_links.github}">GitHub</a>
    @{fi}
</div>
```

#### Repository vs Data: What's the Difference?

**Repository Data (`R.` or `repository.`)**
- Available to all views called within the same handler function
- Set using `ctx.repository_set()` before calling views
- Ideal for shared UI elements, user info, theme settings
- Lives for the duration of the handler function execution

**View Data (Model)**
- Specific to each individual view call
- Passed via `ctx.view(template, data)`
- Accessed via `@{model.key}` or `@{M.key}` prefix
- For page-specific content
- Only available in that specific view call
- Preferabily used for data flowing from db to form or vis versa

**Example showing both:**
```rust
async fn product_page(mut ctx: Context) -> Result<Response> {
    // Repository: Available to all views called in this handler
    ctx.repository_set("nav_items", get_navigation())
       .repository_set("user", get_current_user())
       .repository_set("cart_count", get_cart_count());
    
    // Could render multiple views, all would have repository access
    if should_show_banner() {
        // This view has access to repository data
        let banner_html = ctx.render_partial("/partials/banner", json!({}))?;
    }
    
    // View data: Specific to this view call
    let data = json!({
        "product": get_product(id),
        "reviews": get_reviews(id),
        "related": get_related_products(id)
    });
    
    // Main view also has access to repository data
    ctx.view("/products/show", data)
}
```

```html
<!-- Repository data accessible in the template -->
<nav>
    @{foreach item in R.nav_items}
        <a href="@{item.url}">@{item.title}</a>
    @{end}
    <span class="cart">Cart (@{R.cart_count})</span>
</nav>

<!-- View data (Model) specific to this template -->
<h1>@{model.product.name}</h1>
<p>@{M.product.description}</p>
@{foreach review in model.reviews}
    <div class="review">@{review.text}</div>
@{end}
```

#### Repository Best Practices

1. **Use Context Repository for:**
   - Data needed by multiple views within the same handler
   - User information for the current request
   - UI state for the current handler (theme, layout settings)
   - Temporary computed values used across view calls
   - Avoiding data duplication when rendering multiple views

2. **Use Global Repository (APP/MAIN) for:**
   - Application metadata (name, version, copyright)
   - Site-wide configuration (features, limits)
   - Constants needed across the entire application
   - External service URLs and API keys (non-sensitive)
   - Data accessible throughout the application, not just in views

3. **Naming Conventions:**
   ```rust
   // ✅ Good: Clear, semantic names
   ctx.repository_set("user_permissions", permissions);
   ctx.repository_set("page_metadata", metadata);
   
   // ❌ Bad: Generic or unclear names
   ctx.repository_set("data", something);
   ctx.repository_set("x", value);
   ```

4. **Performance Considerations:**
   - Context repository exists only during handler execution (keep it reasonable)
   - Global repository is shared application-wide (can hold more persistent data)
   - Context repository is cleared when handler returns
   - Global repository persists for the application's lifetime

5. **Common Patterns:**
   ```rust
   // Pattern 1: Set common data at the start of a handler
   async fn my_handler(mut ctx: Context) -> Result<Response> {
       // Set repository data first
       ctx.repository_set("user", get_current_user(&ctx)?);
       ctx.repository_set("notifications", get_notifications(&ctx)?);
       
       // Then render view(s) that need this data
       ctx.view("/my_template", specific_data)
   }
   
   // Pattern 2: Use repository for feature flags
   ctx.repository_set("features", json!({
       "new_editor": is_feature_enabled("new_editor"),
       "beta_api": is_feature_enabled("beta_api")
   }));
   
   // Pattern 3: Breadcrumb builder
   ctx.repository_set("breadcrumbs", json!([
       {"label": "Home", "url": "/"},
       {"label": "Products", "url": "/products"},
       {"label": product.name, "url": null}  // Current page
   ]));
   ```

### Flash Messages

Flash messages are one-time messages that survive redirects but are automatically cleared when a view is rendered. They provide a clean way to show user feedback across page redirections.

#### Setting Flash Messages in Controllers

```rust
// Standard convenience methods
ctx.flash_success("User created successfully!");
ctx.flash_error("Invalid credentials");
ctx.flash_info("Please check your email");

// Generic flash setter for any key and serializable value
ctx.flash("warning_msg", "This is a warning")?;
ctx.flash("user_level", 42)?;
ctx.flash("notification", json!({
    "text": "You have new messages",
    "count": 5
}))?;
ctx.flash("recent_actions", vec!["Logged in", "Updated profile"])?;

// Manual flash management
ctx.flash_clear();                    // Clear all flash messages
ctx.flash_clear_key("error_msg");     // Clear specific flash message

// Typical usage pattern
if validation_failed {
    ctx.flash_error("Please correct the errors below");
    return ctx.redirect("/form");
}

ctx.flash_success("Form submitted successfully!");
ctx.redirect("/dashboard")
```

#### Accessing Flash Messages in Templates

Flash messages are stored in the session but are aliased for convenience, so you can access them directly using `@{flash.xxx}` instead of `@{session.flash.xxx}`:

```html
<!-- Standard flash messages - Total.js -->
@{if flash.success_msg}
    <div class="alert alert-success">@{flash.success_msg}</div>
@{fi}

@{if flash.error_msg}
    <div class="alert alert-error">@{flash.error_msg}</div>
@{fi}

@{if flash.info_msg}
    <div class="alert alert-info">@{flash.info_msg}</div>
@{fi}

<!-- Custom flash messages -->
@{if flash.warning_msg}
    <div class="alert alert-warning">@{flash.warning_msg}</div>
@{fi}

<!-- Complex flash data -->
@{if flash.notification}
    <div class="notification">
        @{flash.notification.text}
        @{if flash.notification.count}
            <span class="badge">@{flash.notification.count}</span>
        @{fi}
    </div>
@{fi}

<!-- Array flash data -->
@{if flash.recent_actions}
    <div class="recent-actions">
        <h4>Recent Actions:</h4>
        <ul>
        @{foreach action in flash.recent_actions}
            <li>@{action}</li>
        @{end}
        </ul>
    </div>
@{fi}

<!-- Number flash data -->
@{if flash.user_level}
    <div class="user-level">Level: @{flash.user_level}</div>
@{fi}
```

#### Flash Message Lifecycle

- **Set in controller**: Flash messages are stored in the session
- **Survive redirects**: Messages persist through HTTP redirects
- **Auto-cleared on view render**: Messages are automatically removed when any view is rendered
- **Manual clearing**: Use `flash_clear()` or `flash_clear_key()` for explicit control

#### Best Practices

```rust
// ✅ Good: Use specific flash keys for different message types
ctx.flash("validation_error", "Email is required")?;
ctx.flash("success", "Account created")?;
ctx.flash("warning", "Trial period expires soon")?;

// ✅ Good: Clear flash before setting new messages in error scenarios
if let Err(e) = process_data() {
    ctx.flash_clear(); // Clear any existing flash
    ctx.flash_error(&format!("Processing failed: {}", e));
    return ctx.redirect("/retry");
}

// ✅ Good: Use structured data for complex notifications
ctx.flash("notification", json!({
    "type": "info",
    "title": "System Update",
    "message": "Maintenance scheduled for tonight",
    "actions": [{"text": "Learn More", "url": "/maintenance"}]
}))?;

// ✅ Good: Selective clearing for fine-grained control
ctx.flash_clear_key("error_msg"); // Keep other flash messages
ctx.flash_success("Operation completed!");
```

### Static Asset Helpers

```html
<!-- Total.js built-in helpers -->
@{css('/static/css/app.css')}
<!-- Renders: <link rel="stylesheet" href="/static/css/app.css"> -->

@{js('/static/js/app.js')}
<!-- Renders: <script src="/static/js/app.js"></script> -->

<!-- With root path -->
@{css(root + '/static/css/app.css')}
<!-- Renders: <link rel="stylesheet" href="/myapp/static/css/app.css"> -->
```

### Translation & Internationalization (i18n)

RustF provides a Total.js-style translation system using resource files with automatic key generation and view-scoped translations.

#### Template Syntax

```html
<!-- Direct text translation (key generated from text) -->
<h1>@(Welcome to our application)</h1>
<p>@(Please enter your credentials)</p>

<!-- Translation by custom key -->
<footer>@(#app.copyright)</footer>
<nav>@(#nav.home)</nav>
```

#### Resource File Format

Translation files use the `.res` extension and are stored in the `resources/` directory:

```
# resources/default.res
[global]
# Common translations used across multiple views
save : "Save"
cancel : "Cancel"
loading : "Loading..."

[views/home/index]
# View-specific translations
welcome_to_our_application : "Welcome to our application"
please_enter_your_credentials : "Please enter your credentials"

[views/layouts/main]
# Layout translations
app.copyright : "© 2024 My Company"
nav.home : "Home"
```

#### Setting Up Translations

**Loading translations in your application:**
```rust
use rustf::prelude::*;
use rustf::views::TotalJsEngine;

// During app initialization
let engine = TotalJsEngine::new("views");

// Load translations from resources directory
engine.load_translations(Path::new("resources"))?;

// Or manually with resource translation system
use rustf::views::totaljs::resource_translation::ResourceTranslationSystem;

let mut translator = ResourceTranslationSystem::new();
translator.load_resources_dir(Path::new("resources"))?;
translator.set_language("fr");  // Switch to French
engine.set_resource_translator(translator);
```

#### Managing Translations with CLI

```bash
# Scan views and generate default.res with all discovered translations
rustf-cli translations scan

# Output:
# 🔍 Scanning views for translations...
# ✅ Generated resources/default.res with 45 translations

# Update existing language files with new keys
rustf-cli translations update --lang fr

# Output:
# 🔄 Regenerating default.res...
# 📝 Updating fr.res...
# 📊 Update Report for fr.res:
#   ✓ Existing translations preserved: 42
#   ⚠️  New keys needing translation: 3
#      - welcome_message
#      - user_greeting
#      - logout_confirm

# Check for missing translations
rustf-cli translations check --lang es

# Show translation statistics
rustf-cli translations stats
```

#### Translation Workflow

1. **Write views with translatable text:**
```html
<!-- views/home/index.html -->
<h1>@(Welcome to RustF)</h1>
<p>@(Build fast web applications)</p>
<div>@(#custom.message)</div>
```

2. **Generate translation keys:**
```bash
rustf-cli translations scan
```

3. **Translate to other languages:**
```bash
# Copy default to create language file
cp resources/default.res resources/fr.res

# Edit fr.res and translate the values
vim resources/fr.res
```

4. **View generated fr.res:**
```
# resources/fr.res
[global]
save : "Enregistrer"
cancel : "Annuler"
loading : "Chargement..."

[views/home/index]
welcome_to_rustf : "Bienvenue à RustF"
build_fast_web_applications : "Construire des applications web rapides"
custom.message : ""  # TODO: needs translation
```

#### Key Generation

- Text in `@(text)` is converted to a readable slug key (max 30 chars)
- Spaces become underscores, special characters removed
- Common translations (appearing in 3+ views) are automatically moved to `[global]`
- Custom keys using `@(#key)` are preserved as-is

#### Best Practices

1. **Use descriptive custom keys for important text:**
   ```html
   @(#app.title)       <!-- Good: Clear purpose -->
   @(#msg1)           <!-- Bad: Unclear -->
   ```

2. **Keep translations organized by view:**
   - Each view gets its own section in the .res file
   - Layouts have their own sections
   - Common text automatically extracted to global

3. **Language switching in controllers:**
   ```rust
   // Set language based on user preference
   if let Some(lang) = ctx.session_get("language") {
       engine.resource_translator.set_language(&lang);
   }
   ```

4. **Fallback language support:**
   ```rust
   translator.set_fallback("en");  // English as fallback
   ```

## Global VIEW API

RustF provides a global `VIEW` API for rendering templates from anywhere in your application without needing a `Context` instance. This is particularly useful for:
- Email generation in background workers
- PDF/report generation
- Dynamic content rendering in services
- Template rendering in middleware or utilities

### Basic Usage

```rust
use rustf::prelude::*;
use serde_json::json;

// Render an inline template string
let html = VIEW::render_string(
    "Hello @{M.name}!",
    json!({"name": "Alice"}),
    None
)?;
// Output: "Hello Alice!"

// Render a template file
let html = VIEW::render(
    "emails/welcome",
    json!({"user": "Alice"}),
    None,
    Some("layouts/email")
)?;
```

### Method Signatures

#### `VIEW::render_string()`
Render an inline template string with model and optional repository data.

```rust
pub fn render_string(
    template_string: &str,
    model: Value,
    repository: Option<Value>
) -> Result<String>
```

**Parameters:**
- `template_string`: Template content as string
- `model`: Main template data (accessible as `@{M.key}`)
- `repository`: Optional context data (accessible as `@{R.key}`)

**Example:**
```rust
let template = "Welcome @{M.user}, your role is @{R.role}";
let model = json!({"user": "Alice"});
let repository = json!({"role": "Admin"});

let html = VIEW::render_string(template, model, Some(repository))?;
// Output: "Welcome Alice, your role is Admin"
```

#### `VIEW::render()`
Render a template file with model, repository, and optional layout.

```rust
pub fn render(
    template_path: &str,
    model: Value,
    repository: Option<Value>,
    layout: Option<&str>
) -> Result<String>
```

**Parameters:**
- `template_path`: Path to template (relative to views directory, without extension)
- `model`: Main template data (accessible as `@{M.key}`)
- `repository`: Optional context data (accessible as `@{R.key}`)
- `layout`: Optional layout template name

**Example:**
```rust
let html = VIEW::render(
    "reports/sales",
    json!({"month": "January", "total": 50000}),
    Some(json!({"company": "ACME Corp", "year": 2025})),
    Some("layouts/pdf")
)?;
```

### Model vs Repository Data

The VIEW API maintains the same model/repository separation as controller rendering:

- **Model (`@{M.key}`)**: Primary template data - the main content
- **Repository (`@{R.key}`)**: Context/metadata - site config, user info, etc.

```rust
let model = json!({
    "product": "Widget",
    "price": 29.99,
    "quantity": 5
});

let repository = json!({
    "site_name": "My Store",
    "currency": "USD",
    "tax_rate": 0.08
});

let template = r#"
    <h1>@{R.site_name}</h1>
    <p>Product: @{M.product}</p>
    <p>Price: @{R.currency} @{M.price}</p>
"#;

let html = VIEW::render_string(template, model, Some(repository))?;
```

### Use Cases

#### Email Generation in Workers

```rust
use rustf::prelude::*;

pub async fn send_welcome_email(ctx: WorkerContext) -> rustf::Result<()> {
    let email_template = r#"
        <html>
        <body>
            <h1>Welcome to @{R.app_name}!</h1>
            <p>Hello @{M.name},</p>
            <p>Thanks for signing up. Your username is: @{M.username}</p>
            <footer>&copy; @{R.year} @{R.company}</footer>
        </body>
        </html>
    "#;
    
    let model = json!({
        "name": ctx.data["name"],
        "username": ctx.data["username"]
    });
    
    let repository = json!({
        "app_name": "My App",
        "year": 2025,
        "company": "My Company Inc."
    });
    
    let html = VIEW::render_string(email_template, model, Some(repository))?;
    
    // Send email with html content
    send_email(&ctx.data["email"].as_str().unwrap(), &html).await?;
    Ok(())
}
```

#### Report Generation

```rust
use rustf::prelude::*;

pub fn generate_sales_report(sales_data: Vec<SaleRecord>) -> rustf::Result<String> {
    let template = r#"
        <h1>Sales Report - @{M.period}</h1>
        <table>
            <tr><th>Product</th><th>Units</th><th>Revenue</th></tr>
            @{foreach item in M.items}
            <tr>
                <td>@{item}</td>
                <td>@{item}</td>
                <td>$@{item}</td>
            </tr>
            @{end}
        </table>
        <p>Generated: @{R.generated_date}</p>
    "#;
    
    let model = json!({
        "period": "Q1 2025",
        "items": sales_data
    });
    
    let repository = json!({
        "generated_date": chrono::Utc::now().to_rfc3339()
    });
    
    VIEW::render_string(template, model, Some(repository))
}
```

#### Dynamic Content in Middleware

```rust
use rustf::prelude::*;

pub async fn render_error_page(status: u16, message: &str) -> rustf::Result<String> {
    let template = r#"
        <!DOCTYPE html>
        <html>
        <head><title>Error @{M.status}</title></head>
        <body>
            <h1>@{M.status} - @{M.title}</h1>
            <p>@{M.message}</p>
            <footer>@{R.app_name}</footer>
        </body>
        </html>
    "#;
    
    let model = json!({
        "status": status,
        "title": if status == 404 { "Not Found" } else { "Error" },
        "message": message
    });
    
    let repository = json!({
        "app_name": "My Application"
    });
    
    VIEW::render_string(template, model, Some(repository))
}
```

#### Template File Rendering

```rust
// views/emails/order_confirmation.html
// <h1>Order Confirmation</h1>
// <p>Thank you @{M.customer_name}!</p>
// <p>Order #@{M.order_id} has been confirmed.</p>

use rustf::prelude::*;

pub async fn send_order_confirmation(order: Order) -> rustf::Result<()> {
    let html = VIEW::render(
        "emails/order_confirmation",
        json!({
            "customer_name": order.customer_name,
            "order_id": order.id,
            "total": order.total
        }),
        Some(json!({
            "site_name": "My Shop",
            "support_email": "support@myshop.com"
        })),
        Some("layouts/email")  // Use email layout
    )?;
    
    send_email(&order.customer_email, &html).await?;
    Ok(())
}
```

### Limitations

1. **No Session Data**: Global VIEW API doesn't have access to session data (use `ctx.view()` in controllers for session access)
2. **Initialization Required**: VIEW must be initialized during app startup (automatic with `auto_load()`)
3. **Template Features**: All Total.js features work (conditionals, loops, variables, etc.)

### Best Practices

1. **Use Model/Repository Separation**: Keep template data (`M`) separate from context (`R`)
2. **Validate Before Rendering**: Ensure data is complete before calling VIEW
3. **Handle Errors**: Always handle `Result` from VIEW methods
4. **Cache Templates**: Use file-based templates for better caching in production
5. **Test Rendering**: Unit test your templates with VIEW::render_string()

## Performance Tips

### Development
```toml
[views]
cache_enabled = false  # Hot-reload templates
```

### Production
```toml
[views]
cache_enabled = true   # Cache compiled templates
storage = "embedded"   # Embed templates in binary
```

### Template Caching

Templates are automatically cached when `cache_enabled = true`:
- First request compiles the template
- Subsequent requests use cached version
- No manual cache management needed

## Directory Structure

```
views/
├── layouts/
│   └── application.html    # Main layout
├── home/
│   ├── index.html          # Homepage
│   └── about.html          # About page
├── auth/
│   ├── login.html          # Login form
│   └── register.html       # Registration
└── components/
    ├── navbar.html         # Reusable navbar
    └── footer.html         # Reusable footer
```

## Error Handling

### Template Not Found
```rust
// Graceful fallback
match ctx.view("missing-template", data) {
    Ok(response) => Ok(response),
    Err(_) => ctx.view("errors/404", json!({"message": "Page not found"}))
}
```

### Syntax Errors
- **Total.js**: Errors shown with line numbers during development
- **Tera**: Compile-time checking with detailed error messages

## Migration Guide

### From Tera to Total.js

| Tera | Total.js |
|------|----------|
| `{{ variable }}` | `@{variable}` |
| `{% if condition %}` | `@{if condition}` |
| `{% endif %}` | `@{fi}` |
| `{% for item in items %}` | `@{foreach item in items}` |
| `{% endfor %}` | `@{end}` |
| `{{ var \| escape }}` | `@{var}` (auto-escaped) |
| `{{ var \| safe }}` | `@{!var}` |

### From Total.js to Tera

| Total.js | Tera |
|----------|------|
| `@{variable}` | `{{ variable }}` |
| `@{if condition}` | `{% if condition %}` |
| `@{fi}` | `{% endif %}` |
| `@{foreach item in items}` | `{% for item in items %}` |
| `@{end}` | `{% endfor %}` |
| `@{!raw_html}` | `{{ raw_html \| safe }}` |

## Best Practices

1. **Use layouts** - Define common structure once
2. **Escape user input** - Use auto-escaping by default
3. **Cache in production** - Enable template caching
4. **Organize templates** - Group by feature/module
5. **Keep logic minimal** - Complex logic belongs in controllers
6. **Use components** - Create reusable template parts
7. **Validate data** - Don't trust template input
8. **Handle errors** - Provide fallback templates

## Troubleshooting

### Templates Not Loading
- Check `views.directory` in config.toml
- Verify file extensions match `views.extension`
- Ensure template names don't include extension in controller

### Variables Not Rendering
- Check variable names match exactly
- Verify data is passed to `ctx.view()`
- Use `@{json(variable)}` to debug data structure

### Layout Issues
- Verify layout path is correct
- Check `@{content}` placeholder exists in layout
- Ensure child template specifies layout

### Performance Issues
- Enable `cache_enabled` in production
- Use `embedded` storage for deployment
- Minimize template complexity
- Move heavy logic to controllers

## Examples

### Complete Controller with Views and Repositories

```rust
use rustf::prelude::*;

#[rustf::install]
impl ProductController {
    #[route(GET, "/products")]
    pub async fn index(mut ctx: Context) -> Result<Response> {
        let products = vec![
            json!({"id": 1, "name": "Widget", "price": 19.99}),
            json!({"id": 2, "name": "Gadget", "price": 29.99}),
        ];
        
        // Set context repository for this page
        ctx.repository_set("view_mode", "grid")
           .repository_set("filters_available", json!(["price", "category", "brand"]))
           .repository_set("user_preferences", json!({
               "currency": "USD",
               "show_tax": true
           }));
        
        ctx.view("products/index", json!({
            "title": "Our Products",
            "products": products,
            "featured": true
        }))
    }
    
    #[route(GET, "/products/:id")]
    pub async fn show(mut ctx: Context) -> Result<Response> {
        let id = ctx.param("id")?;
        
        // Fetch product from database
        let product = json!({
            "id": id,
            "name": "Premium Widget",
            "price": 49.99,
            "description": "High-quality widget"
        });
        
        // Set repository data for product page
        ctx.repository_set("breadcrumbs", json!([
            {"name": "Home", "url": "/"},
            {"name": "Products", "url": "/products"},
            {"name": "Premium Widget", "url": null}
        ]))
        .repository_set("show_reviews", true)
        .repository_set("related_products_count", 5);
        
        ctx.view("products/show", product)
    }
    
    #[route(POST, "/products/:id/buy")]
    pub async fn purchase(ctx: Context) -> Result<Response> {
        let id = ctx.param("id")?;
        
        // Process purchase...
        
        ctx.flash("success", "Purchase completed!");
        ctx.redirect(&format!("/products/{}", id))
    }
}
```

### Template with All Features Including Repositories

```html
@{layout('layouts/default')}

<!-- Page specific CSS -->
@{css('/static/css/products.css')}

<!-- Breadcrumbs from repository -->
@{if R.breadcrumbs}
    <nav class="breadcrumbs">
        @{foreach crumb in R.breadcrumbs}
            @{if crumb.url}
                <a href="@{crumb.url}">@{crumb.name}</a> /
            @{else}
                <span>@{crumb.name}</span>
            @{fi}
        @{end}
    </nav>
@{fi}

<div class="products-page view-@{R.view_mode}">
    <h1>@{title}</h1>
    
    <!-- Filters from repository -->
    @{if R.filters_available}
        <div class="filters">
            @{foreach filter in R.filters_available}
                <button class="filter-btn">Filter by @{filter}</button>
            @{end}
        </div>
    @{fi}
    
    <!-- User preferences from repository -->
    <div class="preferences">
        Currency: @{R.user_preferences.currency}
        @{if R.user_preferences.show_tax}
            <small>(Prices include tax)</small>
        @{fi}
    </div>
    
    <!-- Conditional rendering -->
    @{if featured}
        <div class="featured-banner">
            <p>Check out our featured products!</p>
        </div>
    @{fi}
    
    <!-- Loop through products -->
    <div class="product-grid">
        @{foreach product in products}
            <div class="product-card">
                <h3>@{product.name}</h3>
                <p class="price">@{format_price(product.price)}</p>
                <a href="@{root}/products/@{product.id}" class="btn">
                    View Details
                </a>
            </div>
        @{end}
    </div>
    
    <!-- Reviews section if enabled -->
    @{if R.show_reviews}
        <div class="reviews-section">
            <h2>Customer Reviews</h2>
            <!-- Reviews content -->
        </div>
    @{fi}
    
    <!-- Related products -->
    @{if R.related_products_count > 0}
        <div class="related">
            <p>See @{R.related_products_count} related products</p>
        </div>
    @{fi}
    
    <!-- Raw HTML inclusion -->
    @{if marketing_content}
        @{!marketing_content}
    @{fi}
</div>

<!-- Footer with global repository data -->
<footer>
    <p>@{APP.site_name} v@{APP.version}</p>
    <p>@{MAIN.copyright}</p>
    <p>Contact: @{APP.contact_email}</p>
</footer>

<!-- Page specific JS -->
@{js('/static/js/products.js')}
```

This documentation provides a complete guide to using the RustF view system with practical examples and best practices for both Total.js and Tera template engines.