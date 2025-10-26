# CSS Directory

This directory contains your application's stylesheets and CSS files. These files are served statically by the RustF framework and are accessible via the `/static/css/` URL path.

## 🤖 AI Agent Quick Reference

**Purpose**: Application stylesheets, CSS frameworks, and design system files  
**URL Path**: Files are served at `/static/css/filename.css`  
**Processing**: Static files (no preprocessing by default)  
**Organization**: Component-based CSS architecture recommended  
**Performance**: Minify and combine files for production

## 📁 File Organization

```
public/css/
├── main.css             # Main application stylesheet
├── components.css       # Reusable component styles
├── layout.css          # Layout and grid system
├── forms.css           # Form styling
├── utilities.css       # Utility classes
├── variables.css       # CSS custom properties
├── vendor/             # Third-party CSS libraries
│   ├── bootstrap.min.css
│   ├── fontawesome.min.css
│   └── prism.css
└── pages/              # Page-specific stylesheets
    ├── home.css
    ├── dashboard.css
    └── admin.css
```

## 🚀 Quick Start Template

### Main Stylesheet (`main.css`)
```css
/* Main Application Stylesheet */
/* Import other stylesheets */
@import url('variables.css');
@import url('layout.css');
@import url('components.css');
@import url('forms.css');
@import url('utilities.css');

/* Global Styles */
* {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
}

html {
    font-size: 16px;
    line-height: 1.6;
    scroll-behavior: smooth;
}

body {
    font-family: var(--font-family-base);
    font-size: var(--font-size-base);
    line-height: var(--line-height-base);
    color: var(--color-text);
    background-color: var(--color-background);
    -webkit-font-smoothing: antialiased;
    -moz-osx-font-smoothing: grayscale;
}

/* Focus management for accessibility */
*:focus {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
}

/* Skip link for screen readers */
.skip-link {
    position: absolute;
    top: -40px;
    left: 6px;
    background: var(--color-primary);
    color: white;
    padding: 8px;
    text-decoration: none;
    transition: top 0.3s;
}

.skip-link:focus {
    top: 6px;
}

/* Print styles */
@media print {
    * {
        background: transparent !important;
        color: black !important;
        box-shadow: none !important;
        text-shadow: none !important;
    }
    
    a, a:visited {
        text-decoration: underline;
    }
    
    .no-print {
        display: none !important;
    }
}
```

### CSS Variables (`variables.css`)
```css
/* CSS Custom Properties (Variables) */
:root {
    /* Colors */
    --color-primary: #007bff;
    --color-primary-dark: #0056b3;
    --color-primary-light: #66b3ff;
    
    --color-secondary: #6c757d;
    --color-secondary-dark: #545b62;
    --color-secondary-light: #adb5bd;
    
    --color-success: #28a745;
    --color-success-dark: #1e7e34;
    --color-success-light: #71dd8a;
    
    --color-danger: #dc3545;
    --color-danger-dark: #c82333;
    --color-danger-light: #f1aeb5;
    
    --color-warning: #ffc107;
    --color-warning-dark: #e0a800;
    --color-warning-light: #fff3cd;
    
    --color-info: #17a2b8;
    --color-info-dark: #138496;
    --color-info-light: #9fdfea;
    
    --color-light: #f8f9fa;
    --color-dark: #343a40;
    
    /* Text colors */
    --color-text: #333333;
    --color-text-light: #666666;
    --color-text-muted: #999999;
    --color-background: #ffffff;
    --color-background-alt: #f8f9fa;
    
    /* Typography */
    --font-family-base: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
    --font-family-mono: SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', 'Courier New', monospace;
    --font-family-heading: var(--font-family-base);
    
    --font-size-xs: 0.75rem;    /* 12px */
    --font-size-sm: 0.875rem;   /* 14px */
    --font-size-base: 1rem;     /* 16px */
    --font-size-lg: 1.125rem;   /* 18px */
    --font-size-xl: 1.25rem;    /* 20px */
    --font-size-2xl: 1.5rem;    /* 24px */
    --font-size-3xl: 1.875rem;  /* 30px */
    --font-size-4xl: 2.25rem;   /* 36px */
    
    --font-weight-light: 300;
    --font-weight-normal: 400;
    --font-weight-medium: 500;
    --font-weight-semibold: 600;
    --font-weight-bold: 700;
    
    --line-height-tight: 1.25;
    --line-height-base: 1.6;
    --line-height-relaxed: 1.75;
    
    /* Spacing */
    --spacing-xs: 0.25rem;   /* 4px */
    --spacing-sm: 0.5rem;    /* 8px */
    --spacing-md: 1rem;      /* 16px */
    --spacing-lg: 1.5rem;    /* 24px */
    --spacing-xl: 2rem;      /* 32px */
    --spacing-2xl: 3rem;     /* 48px */
    --spacing-3xl: 4rem;     /* 64px */
    
    /* Border radius */
    --border-radius-sm: 0.125rem;  /* 2px */
    --border-radius-base: 0.25rem; /* 4px */
    --border-radius-md: 0.375rem;  /* 6px */
    --border-radius-lg: 0.5rem;    /* 8px */
    --border-radius-xl: 0.75rem;   /* 12px */
    --border-radius-full: 9999px;
    
    /* Shadows */
    --shadow-sm: 0 1px 2px 0 rgba(0, 0, 0, 0.05);
    --shadow-base: 0 1px 3px 0 rgba(0, 0, 0, 0.1), 0 1px 2px 0 rgba(0, 0, 0, 0.06);
    --shadow-md: 0 4px 6px -1px rgba(0, 0, 0, 0.1), 0 2px 4px -1px rgba(0, 0, 0, 0.06);
    --shadow-lg: 0 10px 15px -3px rgba(0, 0, 0, 0.1), 0 4px 6px -2px rgba(0, 0, 0, 0.05);
    --shadow-xl: 0 20px 25px -5px rgba(0, 0, 0, 0.1), 0 10px 10px -5px rgba(0, 0, 0, 0.04);
    
    /* Transitions */
    --transition-fast: 150ms ease-in-out;
    --transition-base: 250ms ease-in-out;
    --transition-slow: 350ms ease-in-out;
    
    /* Breakpoints (for reference in media queries) */
    --breakpoint-sm: 576px;
    --breakpoint-md: 768px;
    --breakpoint-lg: 992px;
    --breakpoint-xl: 1200px;
    --breakpoint-2xl: 1400px;
    
    /* Container widths */
    --container-sm: 540px;
    --container-md: 720px;
    --container-lg: 960px;
    --container-xl: 1140px;
    --container-2xl: 1320px;
}

/* Dark theme variables */
@media (prefers-color-scheme: dark) {
    :root {
        --color-text: #ffffff;
        --color-text-light: #cccccc;
        --color-text-muted: #999999;
        --color-background: #1a1a1a;
        --color-background-alt: #2d2d2d;
    }
}

/* Dark theme class (for manual toggle) */
.dark-theme {
    --color-text: #ffffff;
    --color-text-light: #cccccc;
    --color-text-muted: #999999;
    --color-background: #1a1a1a;
    --color-background-alt: #2d2d2d;
}
```

### Layout Styles (`layout.css`)
```css
/* Layout and Grid System */

/* Container */
.container {
    width: 100%;
    max-width: var(--container-xl);
    margin: 0 auto;
    padding: 0 var(--spacing-md);
}

@media (min-width: 576px) {
    .container { max-width: var(--container-sm); }
}

@media (min-width: 768px) {
    .container { max-width: var(--container-md); }
}

@media (min-width: 992px) {
    .container { max-width: var(--container-lg); }
}

@media (min-width: 1200px) {
    .container { max-width: var(--container-xl); }
}

@media (min-width: 1400px) {
    .container { max-width: var(--container-2xl); }
}

/* Fluid container */
.container-fluid {
    width: 100%;
    padding: 0 var(--spacing-md);
}

/* Grid System */
.row {
    display: flex;
    flex-wrap: wrap;
    margin: 0 calc(var(--spacing-md) * -0.5);
}

.col {
    flex: 1;
    padding: 0 calc(var(--spacing-md) * 0.5);
}

/* Column sizes */
.col-1 { flex: 0 0 8.333333%; max-width: 8.333333%; }
.col-2 { flex: 0 0 16.666667%; max-width: 16.666667%; }
.col-3 { flex: 0 0 25%; max-width: 25%; }
.col-4 { flex: 0 0 33.333333%; max-width: 33.333333%; }
.col-5 { flex: 0 0 41.666667%; max-width: 41.666667%; }
.col-6 { flex: 0 0 50%; max-width: 50%; }
.col-7 { flex: 0 0 58.333333%; max-width: 58.333333%; }
.col-8 { flex: 0 0 66.666667%; max-width: 66.666667%; }
.col-9 { flex: 0 0 75%; max-width: 75%; }
.col-10 { flex: 0 0 83.333333%; max-width: 83.333333%; }
.col-11 { flex: 0 0 91.666667%; max-width: 91.666667%; }
.col-12 { flex: 0 0 100%; max-width: 100%; }

/* Responsive columns */
@media (min-width: 768px) {
    .col-md-1 { flex: 0 0 8.333333%; max-width: 8.333333%; }
    .col-md-2 { flex: 0 0 16.666667%; max-width: 16.666667%; }
    .col-md-3 { flex: 0 0 25%; max-width: 25%; }
    .col-md-4 { flex: 0 0 33.333333%; max-width: 33.333333%; }
    .col-md-6 { flex: 0 0 50%; max-width: 50%; }
    .col-md-8 { flex: 0 0 66.666667%; max-width: 66.666667%; }
    .col-md-9 { flex: 0 0 75%; max-width: 75%; }
    .col-md-12 { flex: 0 0 100%; max-width: 100%; }
}

/* Header */
.header {
    background: var(--color-background);
    border-bottom: 1px solid var(--color-background-alt);
    padding: var(--spacing-md) 0;
    position: sticky;
    top: 0;
    z-index: 1000;
}

.header-brand {
    font-size: var(--font-size-xl);
    font-weight: var(--font-weight-bold);
    color: var(--color-primary);
    text-decoration: none;
}

/* Navigation */
.nav {
    display: flex;
    align-items: center;
    gap: var(--spacing-lg);
}

.nav-link {
    color: var(--color-text);
    text-decoration: none;
    padding: var(--spacing-sm) var(--spacing-md);
    border-radius: var(--border-radius-base);
    transition: var(--transition-fast);
}

.nav-link:hover,
.nav-link.active {
    background-color: var(--color-primary);
    color: white;
}

/* Main content area */
.main {
    min-height: calc(100vh - 200px);
    padding: var(--spacing-xl) 0;
}

/* Footer */
.footer {
    background: var(--color-background-alt);
    padding: var(--spacing-xl) 0;
    margin-top: var(--spacing-3xl);
    text-align: center;
    color: var(--color-text-muted);
}
```

### Component Styles (`components.css`)
```css
/* Reusable Component Styles */

/* Buttons */
.btn {
    display: inline-block;
    padding: var(--spacing-sm) var(--spacing-md);
    font-size: var(--font-size-base);
    font-weight: var(--font-weight-medium);
    line-height: var(--line-height-tight);
    text-align: center;
    text-decoration: none;
    border: 1px solid transparent;
    border-radius: var(--border-radius-base);
    cursor: pointer;
    transition: var(--transition-fast);
    user-select: none;
}

.btn:hover {
    text-decoration: none;
    transform: translateY(-1px);
}

.btn:focus {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
}

.btn:active {
    transform: translateY(0);
}

.btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
    transform: none;
}

/* Button variants */
.btn-primary {
    background-color: var(--color-primary);
    border-color: var(--color-primary);
    color: white;
}

.btn-primary:hover {
    background-color: var(--color-primary-dark);
    border-color: var(--color-primary-dark);
}

.btn-secondary {
    background-color: var(--color-secondary);
    border-color: var(--color-secondary);
    color: white;
}

.btn-success {
    background-color: var(--color-success);
    border-color: var(--color-success);
    color: white;
}

.btn-danger {
    background-color: var(--color-danger);
    border-color: var(--color-danger);
    color: white;
}

.btn-outline {
    background-color: transparent;
    color: var(--color-primary);
    border-color: var(--color-primary);
}

.btn-outline:hover {
    background-color: var(--color-primary);
    color: white;
}

/* Button sizes */
.btn-sm {
    padding: var(--spacing-xs) var(--spacing-sm);
    font-size: var(--font-size-sm);
}

.btn-lg {
    padding: var(--spacing-md) var(--spacing-lg);
    font-size: var(--font-size-lg);
}

/* Cards */
.card {
    background: var(--color-background);
    border: 1px solid var(--color-background-alt);
    border-radius: var(--border-radius-lg);
    box-shadow: var(--shadow-base);
    overflow: hidden;
}

.card-header {
    padding: var(--spacing-md) var(--spacing-lg);
    background: var(--color-background-alt);
    border-bottom: 1px solid var(--color-background-alt);
    font-weight: var(--font-weight-semibold);
}

.card-body {
    padding: var(--spacing-lg);
}

.card-footer {
    padding: var(--spacing-md) var(--spacing-lg);
    background: var(--color-background-alt);
    border-top: 1px solid var(--color-background-alt);
}

/* Alerts */
.alert {
    padding: var(--spacing-md);
    margin-bottom: var(--spacing-md);
    border: 1px solid transparent;
    border-radius: var(--border-radius-base);
    position: relative;
}

.alert-success {
    background-color: var(--color-success-light);
    border-color: var(--color-success);
    color: var(--color-success-dark);
}

.alert-danger {
    background-color: var(--color-danger-light);
    border-color: var(--color-danger);
    color: var(--color-danger-dark);
}

.alert-warning {
    background-color: var(--color-warning-light);
    border-color: var(--color-warning);
    color: var(--color-warning-dark);
}

.alert-info {
    background-color: var(--color-info-light);
    border-color: var(--color-info);
    color: var(--color-info-dark);
}

.alert-dismissible {
    padding-right: calc(var(--spacing-md) + 24px);
}

.alert-close {
    position: absolute;
    top: var(--spacing-sm);
    right: var(--spacing-sm);
    background: none;
    border: none;
    font-size: var(--font-size-lg);
    cursor: pointer;
    opacity: 0.7;
}

.alert-close:hover {
    opacity: 1;
}

/* Flash messages (from RustF framework) */
.flash {
    padding: var(--spacing-md);
    margin-bottom: var(--spacing-md);
    border-radius: var(--border-radius-base);
    border-left: 4px solid;
}

.flash-error {
    background-color: #f8d7da;
    border-left-color: var(--color-danger);
    color: #721c24;
}

.flash-success {
    background-color: #d4edda;
    border-left-color: var(--color-success);
    color: #155724;
}

.flash-info {
    background-color: #d1ecf1;
    border-left-color: var(--color-info);
    color: #0c5460;
}

.flash-warning {
    background-color: var(--color-warning-light);
    border-left-color: var(--color-warning);
    color: var(--color-warning-dark);
}

/* Tables */
.table {
    width: 100%;
    border-collapse: collapse;
    margin-bottom: var(--spacing-lg);
}

.table th,
.table td {
    padding: var(--spacing-sm) var(--spacing-md);
    text-align: left;
    border-bottom: 1px solid var(--color-background-alt);
}

.table th {
    background-color: var(--color-background-alt);
    font-weight: var(--font-weight-semibold);
}

.table-striped tbody tr:nth-child(even) {
    background-color: var(--color-background-alt);
}

.table-hover tbody tr:hover {
    background-color: var(--color-background-alt);
}

/* Loading spinner */
.spinner {
    width: 20px;
    height: 20px;
    border: 2px solid var(--color-background-alt);
    border-top: 2px solid var(--color-primary);
    border-radius: 50%;
    animation: spin 1s linear infinite;
}

@keyframes spin {
    0% { transform: rotate(0deg); }
    100% { transform: rotate(360deg); }
}

/* Responsive utilities */
@media (max-width: 767px) {
    .hide-mobile { display: none !important; }
}

@media (min-width: 768px) {
    .hide-desktop { display: none !important; }
}
```

## 🎨 CSS Best Practices

### 1. Use CSS Custom Properties
```css
/* Good: Use CSS variables */
.button {
    background-color: var(--color-primary);
    padding: var(--spacing-md);
}

/* Avoid: Hard-coded values */
.button {
    background-color: #007bff;
    padding: 16px;
}
```

### 2. Mobile-First Responsive Design
```css
/* Mobile-first approach */
.component {
    font-size: var(--font-size-sm);
}

@media (min-width: 768px) {
    .component {
        font-size: var(--font-size-base);
    }
}

@media (min-width: 1200px) {
    .component {
        font-size: var(--font-size-lg);
    }
}
```

### 3. Component-Based Architecture
```css
/* Block Element Modifier (BEM) methodology */
.user-card { /* Block */ }
.user-card__avatar { /* Element */ }
.user-card__name { /* Element */ }
.user-card--featured { /* Modifier */ }
.user-card--small { /* Modifier */ }
```

### 4. Performance Optimization
```css
/* Use efficient selectors */
.nav-item { /* Good: class selector */ }
div.nav ul li a { /* Avoid: complex descendant selectors */ }

/* Minimize repaints/reflows */
.animated-element {
    transform: translateX(100px); /* Good: use transform */
    /* left: 100px; Avoid: causes reflow */
}
```

## 🔧 CSS Processing and Build Tools

### Basic CSS Minification
Create `build-css.sh`:
```bash
#!/bin/bash
# Simple CSS minification and concatenation

# Concatenate main CSS files
cat public/css/variables.css \
    public/css/layout.css \
    public/css/components.css \
    public/css/forms.css \
    public/css/utilities.css > public/css/app.css

# Minify (requires csso or similar tool)
if command -v csso &> /dev/null; then
    csso public/css/app.css --output public/css/app.min.css
    echo "CSS minified to app.min.css"
else
    echo "Install csso for minification: npm install -g csso"
fi
```

### PostCSS Configuration (Optional)
If using PostCSS, create `postcss.config.js`:
```javascript
module.exports = {
    plugins: [
        require('autoprefixer'),
        require('cssnano')({
            preset: 'default',
        }),
    ],
};
```

## 🌙 Dark Mode Implementation
```css
/* Automatic dark mode based on system preference */
@media (prefers-color-scheme: dark) {
    :root {
        --color-background: #1a1a1a;
        --color-text: #ffffff;
        /* Override other colors as needed */
    }
}

/* Manual dark mode toggle */
.dark-mode {
    --color-background: #1a1a1a;
    --color-text: #ffffff;
    /* Override other colors as needed */
}

/* Dark mode specific styles */
@media (prefers-color-scheme: dark) {
    .logo {
        filter: invert(1);
    }
}
```

## 📱 Responsive Design Utilities
```css
/* Responsive display utilities */
@media (max-width: 767px) {
    .d-none-mobile { display: none !important; }
}

@media (min-width: 768px) {
    .d-none-desktop { display: none !important; }
}

/* Responsive text utilities */
@media (max-width: 767px) {
    .text-center-mobile { text-align: center !important; }
}

/* Responsive spacing utilities */
@media (max-width: 767px) {
    .p-sm-mobile { padding: var(--spacing-sm) !important; }
}
```

## 🤖 AI Agent Instructions

When working with CSS:
1. **Use CSS custom properties** for consistent theming
2. **Follow mobile-first responsive design** principles
3. **Organize styles by component** for maintainability
4. **Use semantic class names** that describe purpose, not appearance
5. **Implement proper accessibility** with focus states and contrast
6. **Optimize for performance** with efficient selectors
7. **Create consistent spacing** using spacing variables
8. **Support dark mode** with CSS custom properties
9. **Use modern CSS features** like Grid and Flexbox appropriately
10. **Document complex styles** with comments

**Framework Integration**: Reference CSS files in templates with `/static/css/filename.css`

**Performance**: Combine and minify CSS files for production, use efficient selectors, minimize repaint/reflow operations.