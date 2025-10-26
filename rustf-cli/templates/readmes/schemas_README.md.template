# Schemas Directory

This directory contains YAML schema definitions that describe your application's data models and database structure. Schemas are used to generate base models, database migrations, and API documentation.

## 🤖 AI Agent Quick Reference

**Purpose**: Define data models, database structure, and relationships in YAML format  
**File Pattern**: `*.yaml` or `*.yml` files  
**Generation**: Use `rustf-cli schema generate` to create base models from schemas  
**Validation**: Use `rustf-cli schema validate` to check schema correctness  
**Migration**: Generate database migrations from schema changes

## 📁 File Organization

```
schemas/
├── _meta.yaml          # Schema metadata and global settings
├── users.yaml          # User model definition
├── posts.yaml          # Post model definition
├── categories.yaml     # Category model definition
├── comments.yaml       # Comment model definition
└── tags.yaml           # Tag model definition
```

## 🚀 Quick Start Template

### Basic Model Schema
Create `schemas/users.yaml`:

```yaml
# User model schema definition
name: users
table: users
description: "User accounts and authentication"

# Database table configuration
database:
  engine: postgresql  # postgresql, mysql, sqlite
  charset: utf8mb4
  collation: utf8mb4_unicode_ci

# Model fields definition
fields:
  id:
    type: integer
    primary_key: true
    auto_increment: true
    description: "Unique user identifier"
    
  name:
    type: string
    max_length: 100
    required: true
    description: "User's full name"
    validation:
      min_length: 2
      pattern: "^[a-zA-Z\\s]+$"
      
  email:
    type: string
    max_length: 255
    required: true
    unique: true
    description: "User's email address"
    validation:
      format: email
      
  password_hash:
    type: string
    max_length: 255
    required: true
    description: "Hashed password (bcrypt)"
    sensitive: true  # Exclude from API responses
    
  avatar_url:
    type: string
    max_length: 500
    nullable: true
    description: "Profile avatar URL"
    
  is_active:
    type: boolean
    default: true
    description: "Account status"
    
  is_admin:
    type: boolean
    default: false
    description: "Administrator privileges"
    
  email_verified_at:
    type: timestamp
    nullable: true
    description: "Email verification timestamp"
    
  last_login_at:
    type: timestamp
    nullable: true
    description: "Last login timestamp"
    
  created_at:
    type: timestamp
    default: now
    description: "Account creation time"
    
  updated_at:
    type: timestamp
    default: now
    on_update: now
    description: "Last update time"

# Database indexes for performance
indexes:
  - name: idx_users_email
    fields: [email]
    unique: true
    
  - name: idx_users_active_created
    fields: [is_active, created_at]
    
  - name: idx_users_name
    fields: [name]
    type: btree

# Model relationships
relations:
  posts:
    type: has_many
    model: Post
    foreign_key: user_id
    description: "User's blog posts"
    
  comments:
    type: has_many
    model: Comment
    foreign_key: user_id
    description: "User's comments"
    
  profile:
    type: has_one
    model: UserProfile
    foreign_key: user_id
    description: "Extended user profile"

# API configuration
api:
  endpoints:
    - path: "/api/users"
      methods: [GET, POST]
      description: "List and create users"
      
    - path: "/api/users/{id}"
      methods: [GET, PUT, DELETE]
      description: "Get, update, delete user"
      
  serialization:
    exclude: [password_hash]  # Never include in API responses
    include_relations: [posts, profile]
    
# Validation rules
validation:
  rules:
    - field: email
      unique: true
      message: "Email address already exists"
      
    - field: name
      custom: validate_username
      message: "Username contains invalid characters"

# Migration settings
migration:
  create_table: true
  drop_table_if_exists: false
  add_timestamps: true
```

### Relationship Schema
Create `schemas/posts.yaml`:

```yaml
name: posts
table: posts
description: "Blog posts and articles"

fields:
  id:
    type: integer
    primary_key: true
    auto_increment: true
    
  user_id:
    type: integer
    required: true
    foreign_key:
      table: users
      field: id
      on_delete: cascade
      on_update: cascade
    description: "Post author"
    
  category_id:
    type: integer
    nullable: true
    foreign_key:
      table: categories
      field: id
      on_delete: set_null
      on_update: cascade
    description: "Post category"
    
  title:
    type: string
    max_length: 200
    required: true
    description: "Post title"
    
  slug:
    type: string
    max_length: 250
    required: true
    unique: true
    description: "URL-friendly post identifier"
    
  content:
    type: text
    required: true
    description: "Post content (Markdown)"
    
  excerpt:
    type: text
    nullable: true
    description: "Post summary/excerpt"
    
  featured_image:
    type: string
    max_length: 500
    nullable: true
    description: "Featured image URL"
    
  status:
    type: enum
    values: [draft, published, archived]
    default: draft
    description: "Post publication status"
    
  published_at:
    type: timestamp
    nullable: true
    description: "Publication timestamp"
    
  created_at:
    type: timestamp
    default: now
    
  updated_at:
    type: timestamp
    default: now
    on_update: now

indexes:
  - name: idx_posts_user_status
    fields: [user_id, status]
    
  - name: idx_posts_slug
    fields: [slug]
    unique: true
    
  - name: idx_posts_published
    fields: [status, published_at]
    where: "status = 'published'"

relations:
  author:
    type: belongs_to
    model: User
    foreign_key: user_id
    
  category:
    type: belongs_to
    model: Category
    foreign_key: category_id
    
  comments:
    type: has_many
    model: Comment
    foreign_key: post_id
    
  tags:
    type: many_to_many
    model: Tag
    through: post_tags
    foreign_key: post_id
    other_key: tag_id

api:
  endpoints:
    - path: "/api/posts"
      methods: [GET, POST]
      filters: [status, category_id, user_id]
      
    - path: "/api/posts/{slug}"
      methods: [GET, PUT, DELETE]
      
  serialization:
    include_relations: [author, category, tags]
    exclude_when:
      - field: status
        value: draft
        unless_owner: true
```

### Many-to-Many Relationship
Create `schemas/post_tags.yaml`:

```yaml
name: post_tags
table: post_tags
description: "Many-to-many relationship between posts and tags"

fields:
  id:
    type: integer
    primary_key: true
    auto_increment: true
    
  post_id:
    type: integer
    required: true
    foreign_key:
      table: posts
      field: id
      on_delete: cascade
      
  tag_id:
    type: integer
    required: true
    foreign_key:
      table: tags
      field: id
      on_delete: cascade
      
  created_at:
    type: timestamp
    default: now

indexes:
  - name: idx_post_tags_unique
    fields: [post_id, tag_id]
    unique: true
    
  - name: idx_post_tags_post
    fields: [post_id]
    
  - name: idx_post_tags_tag
    fields: [tag_id]

# Pivot table configuration
pivot:
  models: [Post, Tag]
  additional_fields: []  # Extra fields beyond foreign keys
```

## 📊 Schema Configuration Reference

### Field Types
```yaml
fields:
  # Numeric types
  integer_field:
    type: integer
    size: 11  # MySQL specific
    
  big_integer_field:
    type: bigint
    
  decimal_field:
    type: decimal
    precision: 10
    scale: 2
    
  float_field:
    type: float
    
  # String types
  string_field:
    type: string
    max_length: 255
    
  text_field:
    type: text
    
  longtext_field:
    type: longtext
    
  # Date/time types
  date_field:
    type: date
    
  time_field:
    type: time
    
  datetime_field:
    type: datetime
    
  timestamp_field:
    type: timestamp
    
  # Other types
  boolean_field:
    type: boolean
    
  json_field:
    type: json
    
  enum_field:
    type: enum
    values: [active, inactive, pending]
    
  uuid_field:
    type: uuid
```

### Field Attributes
```yaml
fields:
  example_field:
    type: string
    # Basic attributes
    required: true
    nullable: false
    unique: true
    primary_key: false
    auto_increment: false
    
    # String attributes
    max_length: 255
    min_length: 1
    
    # Numeric attributes
    precision: 10
    scale: 2
    unsigned: true
    
    # Default values
    default: "default_value"
    default: now  # For timestamps
    default: null
    
    # Update behavior
    on_update: now  # For timestamps
    
    # Documentation
    description: "Field description"
    
    # API behavior
    sensitive: true  # Exclude from API responses
    readonly: true   # Read-only in API
    
    # Validation
    validation:
      format: email
      pattern: "^[A-Z][a-z]+$"
      min_value: 0
      max_value: 100
      custom: custom_validator_function
```

### Relationships
```yaml
relations:
  # One-to-many
  posts:
    type: has_many
    model: Post
    foreign_key: user_id
    dependent: destroy  # delete, nullify, restrict
    
  # Many-to-one
  user:
    type: belongs_to
    model: User
    foreign_key: user_id
    
  # One-to-one
  profile:
    type: has_one
    model: UserProfile
    foreign_key: user_id
    
  # Many-to-many
  tags:
    type: many_to_many
    model: Tag
    through: post_tags  # Join table
    foreign_key: post_id
    other_key: tag_id
    
  # Polymorphic
  commentable:
    type: morphs
    models: [Post, Page]
```

### Indexes
```yaml
indexes:
  # Simple index
  - name: idx_users_email
    fields: [email]
    
  # Unique index
  - name: idx_users_email_unique
    fields: [email]
    unique: true
    
  # Composite index
  - name: idx_posts_user_status
    fields: [user_id, status, created_at]
    
  # Partial index
  - name: idx_active_users
    fields: [email]
    where: "is_active = true"
    
  # Index type (database-specific)
  - name: idx_posts_title_fulltext
    fields: [title, content]
    type: fulltext  # fulltext, btree, hash, gin, gist
```

## 🔧 CLI Commands

### Schema Validation
```bash
# Validate specific schema
rustf-cli schema validate --file schemas/users.yaml

# Validate all schemas
rustf-cli schema validate --all

# Validate with detailed output
rustf-cli schema validate --all --verbose
```

### Model Generation
```bash
# Generate specific model
rustf-cli schema generate --model users

# Generate all models
rustf-cli schema generate --all

# Generate with validation first
rustf-cli schema generate --all --validate

# Dry run (show what would be generated)
rustf-cli schema generate --model users --dry-run
```

### Migration Generation
```bash
# Generate migration from schema changes
rustf-cli schema migrate --model users

# Generate migration for all models
rustf-cli schema migrate --all

# Generate specific migration type
rustf-cli schema migrate --model users --type create_table
```

### Schema Analysis
```bash
# Analyze schema relationships
rustf-cli schema analyze --relationships

# Check for schema conflicts
rustf-cli schema analyze --conflicts

# Generate documentation
rustf-cli schema docs --output docs/models.md
```

## 📝 Meta Configuration

Create `schemas/_meta.yaml` for global settings:

```yaml
# Global schema configuration
version: "1.0"
database:
  default_engine: postgresql
  default_charset: utf8mb4
  default_collation: utf8mb4_unicode_ci
  
# Global field defaults
field_defaults:
  string:
    max_length: 255
    charset: utf8mb4
    
  timestamp:
    default: now
    on_update: now
    
# Global validation settings
validation:
  strict_mode: true
  validate_foreign_keys: true
  require_descriptions: false
  
# Code generation settings
generation:
  namespace: "App\\Models"
  base_class: "Model"
  use_traits: [HasTimestamps, SoftDeletes]
  
# API defaults
api:
  base_path: "/api"
  version: "v1"
  pagination:
    default_per_page: 15
    max_per_page: 100
    
# Migration settings
migrations:
  path: "database/migrations"
  timestamp_format: "Y_m_d_His"
```

## 🧪 Schema Testing

### Validation Testing
```yaml
# In schema file, add test cases
tests:
  valid_data:
    - name: "John Doe"
      email: "john@example.com"
      password_hash: "$2b$10$..."
      
  invalid_data:
    - name: ""  # Should fail: required field
    - email: "invalid-email"  # Should fail: format validation
    - name: "a"  # Should fail: min_length validation
```

### Schema Validation
```bash
# Test schema against sample data
rustf-cli schema test --file schemas/users.yaml

# Run all schema tests
rustf-cli schema test --all
```

## 🚀 Advanced Features

### Custom Validation
```yaml
fields:
  username:
    type: string
    validation:
      custom: validate_username
      message: "Username must be alphanumeric and 3-20 characters"
      
# Custom validation function (implement in Rust)
# fn validate_username(value: &str) -> ValidationResult
```

### Database-Specific Features
```yaml
# PostgreSQL specific
fields:
  tags:
    type: array
    element_type: string
    
  metadata:
    type: jsonb
    
  search_vector:
    type: tsvector
    
# MySQL specific  
fields:
  coordinates:
    type: point
    
  data:
    type: json
```

### Conditional Fields
```yaml
fields:
  user_type:
    type: enum
    values: [individual, business]
    
  business_name:
    type: string
    required_if:
      field: user_type
      value: business
      
  tax_id:
    type: string
    required_if:
      field: user_type
      value: business
```

## 🤖 AI Agent Instructions

When working with schemas:
1. **Start with schema definitions** before creating models
2. **Use descriptive field names and descriptions**
3. **Define proper relationships** between models
4. **Add appropriate indexes** for performance
5. **Include validation rules** for data integrity
6. **Use enum types** for limited value sets
7. **Add API configuration** for automatic endpoint generation
8. **Document field purposes** with descriptions
9. **Use foreign keys** with proper cascade rules
10. **Test schemas** with sample data

**Workflow**: Define Schema → Validate → Generate Models → Create Migrations → Test

**Best Practices**: Keep schemas in version control, validate before generation, use consistent naming conventions, document relationships clearly.