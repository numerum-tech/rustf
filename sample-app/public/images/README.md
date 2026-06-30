# Images Directory

This directory contains your application's static images, icons, graphics, and visual assets. These files are served statically by the RustF framework and are accessible via the `/static/images/` URL path.

## 🤖 AI Agent Quick Reference

**Purpose**: Static images, icons, logos, graphics, and visual assets  
**URL Path**: Files are served at `/static/images/filename.ext`  
**Formats**: JPG, PNG, GIF, SVG, WebP, ICO supported  
**Organization**: Organized by type and purpose for easy maintenance  
**Optimization**: Compress images for web delivery and performance

## 📁 File Organization

```
public/images/
├── logo/                   # Brand logos and variations
│   ├── logo.svg           # Main logo (vector)
│   ├── logo.png           # Main logo (raster)
│   ├── logo-white.svg     # White version for dark backgrounds
│   ├── logo-small.png     # Small/favicon version
│   └── favicon.ico        # Browser favicon
├── icons/                 # UI icons and graphics
│   ├── ui/               # General UI icons
│   │   ├── home.svg
│   │   ├── user.svg
│   │   ├── settings.svg
│   │   └── search.svg
│   ├── social/           # Social media icons
│   │   ├── twitter.svg
│   │   ├── facebook.svg
│   │   ├── linkedin.svg
│   │   └── github.svg
│   └── status/           # Status and state icons
│       ├── success.svg
│       ├── error.svg
│       ├── warning.svg
│       └── info.svg
├── hero/                  # Hero images and banners
│   ├── hero-home.jpg
│   ├── hero-about.jpg
│   └── hero-contact.jpg
├── content/               # Content-related images
│   ├── blog/             # Blog post images
│   ├── products/         # Product images
│   ├── gallery/          # Image gallery
│   └── testimonials/     # User testimonials
├── backgrounds/           # Background images and textures
│   ├── patterns/         # Repeating patterns
│   ├── gradients/        # Gradient backgrounds
│   └── textures/         # Texture backgrounds
├── placeholders/          # Placeholder images
│   ├── user-avatar.png   # Default user avatar
│   ├── no-image.png      # Missing image placeholder
│   └── loading.gif       # Loading animations
└── generated/            # Auto-generated images (thumbnails, etc.)
    ├── thumbnails/       # Thumbnail versions
    ├── optimized/        # Optimized versions
    └── responsive/       # Responsive image variants
```

## 🚀 Image Usage Examples

### In HTML Templates
```html
<!-- Logo usage -->
<img src="/static/images/logo/logo.svg" alt="Sample App Smoke Logo" class="logo">

<!-- Hero image -->
<div class="hero" style="background-image: url('/static/images/hero/hero-home.jpg')">
    <h1>Welcome to Sample App Smoke</h1>
</div>

<!-- Content images -->
<img src="/static/images/content/blog/article-1.jpg" 
     alt="Article featured image"
     loading="lazy"
     width="800" 
     height="400">

<!-- Icon usage -->
<img src="/static/images/icons/ui/user.svg" alt="User" class="icon">
<span class="icon-user"></span> <!-- If using CSS sprites or icon fonts -->

<!-- Responsive images -->
<picture>
    <source media="(min-width: 768px)" 
            srcset="/static/images/hero/hero-home-large.webp">
    <source media="(min-width: 768px)" 
            srcset="/static/images/hero/hero-home-large.jpg">
    <source srcset="/static/images/hero/hero-home-small.webp">
    <img src="/static/images/hero/hero-home-small.jpg" 
         alt="Hero image" 
         loading="lazy">
</picture>

<!-- Placeholder for missing images -->
<img src="/static/images/placeholders/no-image.png" 
     alt="Image placeholder"
     onerror="this.src='/static/images/placeholders/no-image.png'">
```

### In CSS
```css
/* Background images */
.hero-section {
    background-image: url('/static/images/hero/hero-home.jpg');
    background-size: cover;
    background-position: center;
    background-repeat: no-repeat;
}

/* Icon sprites */
.icon {
    background-image: url('/static/images/icons/sprite.png');
    background-repeat: no-repeat;
    display: inline-block;
    width: 16px;
    height: 16px;
}

.icon-user { background-position: 0 0; }
.icon-settings { background-position: -16px 0; }
.icon-home { background-position: -32px 0; }

/* Responsive backgrounds */
@media (min-width: 768px) {
    .hero-section {
        background-image: url('/static/images/hero/hero-home-large.jpg');
    }
}

/* CSS for lazy loading */
.lazy-image {
    opacity: 0;
    transition: opacity 0.3s;
}

.lazy-image.loaded {
    opacity: 1;
}
```

## 🎨 Image Types and Best Practices

### Logo and Branding
```
logo/
├── logo.svg              # Vector format (scalable, small file size)
├── logo.png              # Raster format (fallback, transparency)
├── logo-white.svg        # White version for dark backgrounds
├── logo-horizontal.svg   # Horizontal layout variant
├── logo-icon.svg         # Icon-only version
└── favicon.ico           # Browser favicon (16x16, 32x32, 48x48)
```

**Best Practices:**
- Use SVG for logos when possible (scalable, small file size)
- Provide PNG fallbacks for older browsers
- Create variations for different backgrounds and contexts
- Optimize SVG files by removing unnecessary metadata

### Icons and Graphics
```
icons/
├── ui/                   # User interface icons
│   ├── *.svg            # Vector icons (preferred)
│   └── sprite.png       # Icon sprite for raster icons
├── social/              # Social media icons
└── status/              # Status indicators
```

**Best Practices:**
- Use SVG for icons when possible
- Create icon sprites to reduce HTTP requests
- Maintain consistent sizing (16x16, 24x24, 32x32)
- Use semantic naming conventions

### Content Images
```
content/
├── blog/                # Blog post images
│   ├── article-1.jpg    # Original image
│   ├── article-1-thumb.jpg  # Thumbnail version
│   └── article-1-large.jpg  # Large version
├── products/            # Product images
└── gallery/            # Image galleries
```

**Best Practices:**
- Provide multiple sizes for responsive design
- Use appropriate compression levels
- Include alt text for accessibility
- Use lazy loading for performance

## 🔧 Image Optimization

### Compression Settings
```bash
# JPEG compression (use for photos)
# Quality: 85-95 for high quality, 75-85 for web optimized

# PNG compression (use for graphics with transparency)
# Use PNG-8 for simple graphics, PNG-24 for complex images

# WebP format (modern browsers)
# 20-30% smaller than JPEG/PNG with similar quality
```

### Optimization Tools
```bash
# ImageOptim (macOS)
imageoptim *.jpg *.png

# TinyPNG API
curl --user api:YOUR_API_KEY \
     --data-binary @input.png \
     --output output.png \
     https://api.tinify.com/shrink

# Command line tools
jpegoptim --max=85 *.jpg
optipng -o7 *.png
svgo *.svg
```

### Responsive Images Script
Create `optimize-images.sh`:
```bash
#!/bin/bash
# Image optimization and responsive variants generation

SOURCE_DIR="public/images/content"
OUTPUT_DIR="public/images/generated"

# Create output directories
mkdir -p "$OUTPUT_DIR/thumbnails"
mkdir -p "$OUTPUT_DIR/optimized" 
mkdir -p "$OUTPUT_DIR/responsive"

# Function to optimize and resize images
optimize_image() {
    local input_file="$1"
    local output_file="$2"
    local width="$3"
    local quality="$4"
    
    # Check if ImageMagick is available
    if command -v convert &> /dev/null; then
        convert "$input_file" \
            -resize "${width}x" \
            -quality "$quality" \
            -strip \
            "$output_file"
    else
        echo "ImageMagick not found. Install with: brew install imagemagick"
        return 1
    fi
}

# Process all images in content directory
find "$SOURCE_DIR" -type f \( -iname "*.jpg" -o -iname "*.jpeg" -o -iname "*.png" \) | while read -r file; do
    filename=$(basename "$file")
    name="${filename%.*}"
    ext="${filename##*.}"
    
    echo "Processing: $filename"
    
    # Generate thumbnail (300px wide, 80% quality)
    optimize_image "$file" "$OUTPUT_DIR/thumbnails/${name}-thumb.${ext}" 300 80
    
    # Generate small version (600px wide, 85% quality)
    optimize_image "$file" "$OUTPUT_DIR/responsive/${name}-small.${ext}" 600 85
    
    # Generate medium version (1200px wide, 85% quality)
    optimize_image "$file" "$OUTPUT_DIR/responsive/${name}-medium.${ext}" 1200 85
    
    # Generate large version (1920px wide, 90% quality)
    optimize_image "$file" "$OUTPUT_DIR/responsive/${name}-large.${ext}" 1920 90
    
    # Generate WebP versions if cwebp is available
    if command -v cwebp &> /dev/null; then
        cwebp -q 85 "$file" -o "$OUTPUT_DIR/optimized/${name}.webp"
    fi
done

echo "✅ Image optimization complete!"
```

## 📱 Responsive Images Implementation

### HTML Picture Element
```html
<!-- Responsive image with multiple formats -->
<picture>
    <!-- WebP format for modern browsers -->
    <source media="(min-width: 1200px)" 
            srcset="/static/images/generated/responsive/hero-large.webp"
            type="image/webp">
    <source media="(min-width: 768px)" 
            srcset="/static/images/generated/responsive/hero-medium.webp"
            type="image/webp">
    <source srcset="/static/images/generated/responsive/hero-small.webp"
            type="image/webp">
    
    <!-- Fallback JPEG images -->
    <source media="(min-width: 1200px)" 
            srcset="/static/images/generated/responsive/hero-large.jpg">
    <source media="(min-width: 768px)" 
            srcset="/static/images/generated/responsive/hero-medium.jpg">
    
    <!-- Final fallback -->
    <img src="/static/images/generated/responsive/hero-small.jpg" 
         alt="Hero image"
         loading="lazy"
         width="1200" 
         height="600">
</picture>
```

### CSS Responsive Backgrounds
```css
/* Mobile first approach */
.hero-bg {
    background-image: url('/static/images/generated/responsive/hero-small.jpg');
    background-size: cover;
    background-position: center;
}

/* Tablet */
@media (min-width: 768px) {
    .hero-bg {
        background-image: url('/static/images/generated/responsive/hero-medium.jpg');
    }
}

/* Desktop */
@media (min-width: 1200px) {
    .hero-bg {
        background-image: url('/static/images/generated/responsive/hero-large.jpg');
    }
}

/* High DPI displays */
@media (-webkit-min-device-pixel-ratio: 2), (min-resolution: 192dpi) {
    .hero-bg {
        background-image: url('/static/images/generated/responsive/hero-large.jpg');
    }
}
```

## ⚡ Performance Optimization

### Lazy Loading JavaScript
```javascript
// Intersection Observer for lazy loading
class LazyImageLoader {
    constructor() {
        this.imageObserver = new IntersectionObserver((entries, observer) => {
            entries.forEach(entry => {
                if (entry.isIntersecting) {
                    const img = entry.target;
                    this.loadImage(img);
                    observer.unobserve(img);
                }
            });
        });
        
        this.init();
    }
    
    init() {
        const lazyImages = document.querySelectorAll('img[data-src], source[data-srcset]');
        lazyImages.forEach(img => this.imageObserver.observe(img));
    }
    
    loadImage(img) {
        // Handle img elements
        if (img.tagName === 'IMG') {
            if (img.dataset.src) {
                img.src = img.dataset.src;
                img.removeAttribute('data-src');
            }
            if (img.dataset.srcset) {
                img.srcset = img.dataset.srcset;
                img.removeAttribute('data-srcset');
            }
        }
        
        // Handle source elements
        if (img.tagName === 'SOURCE') {
            if (img.dataset.srcset) {
                img.srcset = img.dataset.srcset;
                img.removeAttribute('data-srcset');
            }
        }
        
        img.classList.add('loaded');
    }
}

// Initialize lazy loading when DOM is ready
document.addEventListener('DOMContentLoaded', () => {
    new LazyImageLoader();
});
```

### Image Error Handling
```javascript
// Handle broken images gracefully
class ImageErrorHandler {
    constructor() {
        this.init();
    }
    
    init() {
        document.addEventListener('error', (e) => {
            if (e.target.tagName === 'IMG') {
                this.handleImageError(e.target);
            }
        }, true);
    }
    
    handleImageError(img) {
        // Don't handle if already showing placeholder
        if (img.src.includes('placeholders/no-image.png')) {
            return;
        }
        
        // Show placeholder image
        img.src = '/static/images/placeholders/no-image.png';
        img.alt = 'Image not available';
        img.classList.add('image-error');
        
        // Log error for debugging
        console.warn('Failed to load image:', img.dataset.originalSrc || img.src);
    }
}

new ImageErrorHandler();
```

## 🎭 Image Sprites and Icon Systems

### CSS Sprite Generation
```css
/* Icon sprite styles */
.icon {
    background-image: url('/static/images/icons/sprite.png');
    background-repeat: no-repeat;
    display: inline-block;
    width: 24px;
    height: 24px;
}

/* Individual icon positions */
.icon-home { background-position: 0 0; }
.icon-user { background-position: -24px 0; }
.icon-settings { background-position: -48px 0; }
.icon-search { background-position: -72px 0; }
.icon-logout { background-position: 0 -24px; }
```

### SVG Icon System
```html
<!-- SVG sprite (inline in HTML) -->
<svg style="display: none;">
    <defs>
        <symbol id="icon-home" viewBox="0 0 24 24">
            <path d="M10 20v-6h4v6h5v-8h3L12 3 2 12h3v8z"/>
        </symbol>
        <symbol id="icon-user" viewBox="0 0 24 24">
            <path d="M12 12c2.21 0 4-1.79 4-4s-1.79-4-4-4-4 1.79-4 4 1.79 4 4 4zm0 2c-2.67 0-8 1.34-8 4v2h16v-2c0-2.66-5.33-4-8-4z"/>
        </symbol>
    </defs>
</svg>

<!-- Usage -->
<svg class="icon">
    <use href="#icon-home"></use>
</svg>
```

## 📊 Image Management Best Practices

### File Naming Conventions
```
# Use descriptive, lowercase names with hyphens
logo-main.svg                 ✅ Good
Logo_Main.SVG                ❌ Avoid

# Include size or purpose in filename
hero-home-1920x1080.jpg      ✅ Good
user-avatar-thumbnail.png    ✅ Good
image1.jpg                   ❌ Avoid

# Use consistent prefixes for related images
product-laptop-front.jpg     ✅ Good
product-laptop-side.jpg      ✅ Good
product-laptop-back.jpg      ✅ Good
```

### Version Control Considerations
```gitignore
# .gitignore entries for images
public/images/generated/     # Generated/optimized images
public/images/uploads/       # User uploaded images
*.psd                        # Photoshop files
*.ai                         # Illustrator files
*.sketch                     # Sketch files

# Keep source files in separate directory
design-assets/               # Original design files
```

### Image Metadata Management
```javascript
// Image metadata for CMS or dynamic content
const imageMetadata = {
    'hero-home.jpg': {
        alt: 'Modern office workspace with natural lighting',
        credit: 'Photo by John Doe',
        license: 'Unsplash License',
        tags: ['office', 'workspace', 'business'],
        sizes: {
            small: 'hero-home-small.jpg',
            medium: 'hero-home-medium.jpg',
            large: 'hero-home-large.jpg'
        }
    }
};
```

## 🤖 AI Agent Instructions

When working with images:
1. **Organize images by type and purpose** for easy maintenance
2. **Use appropriate formats** (SVG for vectors, JPEG for photos, PNG for transparency)
3. **Optimize images for web** with proper compression
4. **Provide responsive variants** for different screen sizes
5. **Include proper alt text** for accessibility
6. **Implement lazy loading** for performance
7. **Handle image errors gracefully** with fallbacks
8. **Use consistent naming conventions** for easy organization
9. **Create image sprites or use SVG symbols** for icons
10. **Consider modern formats** like WebP for better compression

**Framework Integration**: Reference images with `/static/images/` URL path, handle user uploads in `uploads/` directory, generate responsive variants as needed.

**Performance**: Optimize file sizes, use lazy loading, implement proper caching headers, provide multiple formats for different browsers.