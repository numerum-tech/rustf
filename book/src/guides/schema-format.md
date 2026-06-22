# Schema Format Reference

RustF schemas are YAML files that describe your data model. They are the
**single source of truth**: the CLI generates both Rust models
(`schema generate models`) and full SQL DDL (`schema generate sql`)
from them. SQL is a standard, but this YAML format is RustF-specific — this
page is its complete specification.

This reference is derived from the actual parser (`rustf-schema`). Keys not
listed here are **ignored** by the parser; a few keys are accepted for
forward-compatibility but have no effect yet — those are marked
*(accepted, ignored)*.

> **`schema generate sql` is experimental.** The DDL generator is verified on
> representative schemas across all three dialects, but has not yet been
> exercised at scale. Review the generated SQL before applying it. Model
> generation (`schema generate models`) is stable.

## File layout

```text
schemas/
├── _meta.yaml        # optional global metadata (see below)
├── users.yaml        # one file per table (or group several tables per file)
└── orders.yaml
```

Each table file maps a **logical table name** to a table definition. You may
put several tables in one file.

```yaml
# schemas/users.yaml
Users:                  # logical name (used for the generated model)
  table: users          # physical table name
  version: 1
  fields:
    id:
      type: mediumint
      primary_key: true
      auto: true
```

> **`fields` is a map, not a list.** Each field is a key (`id:`), not a
> `- name: id` list item. List form does not parse.

## Table definition

| Key | Type | Required | Purpose |
|-----|------|:--------:|---------|
| `table` | string | **yes** | Physical table name used in generated SQL/queries. |
| `version` | integer | **yes** | Schema version for change tracking and consistency checks. |
| `name` | string | no | Override for the generated model name (defaults to the YAML key). |
| `description` | string | no | Human-readable description (emitted as a SQL comment). |
| `database_type` | string | no | `mysql` \| `postgres` \| `sqlite` (per-table override of `_meta`). |
| `database_name` | string | no | Source database instance name (informational). |
| `element_type` | string | no | `table` \| `view` \| `materialized_view`. |
| `tags` | string[] | no | Free-form categorization. |
| `ai_context` | string | no | Extra guidance for AI code assistants. |
| `fields` | map | **yes** | Field definitions (see below). |
| `relations` | map | no | Relationships to other tables (see below). |
| `indexes` | list | no | Index definitions (see below). |
| `constraints` | list | no | Table-level constraints. *(reserved — parsed but not yet enforced)* |

## Field definition

A field maps a column name to its type and constraints.

```yaml
email:
  type: string(150)
  required: true
  unique: true
  description: User's primary email
```

### Field keys

The **Effect** column says where a key actually changes output:
- **DDL** — emitted in the generated SQL schema.
- **model** — changes the generated Rust struct/types.
- **metadata** — parsed and exposed to tooling and AI assistants (via the CLI),
  but **not enforced** at runtime or in SQL. Validation keys live here today:
  they document intent but the generated model does not check them.

| Key | Type | Effect | Purpose |
|-----|------|--------|---------|
| `type` | string | DDL + model | Column type. See [Type system](#type-system). **Required.** |
| `values` | string[] | DDL + model | Allowed values — required when `type: enum`. |
| `primary_key` | bool | DDL + model | Marks the primary key. |
| `auto` | bool | DDL + model | Auto-increment (`SERIAL` / `AUTO_INCREMENT` / `AUTOINCREMENT`). |
| `required` | bool | DDL + model | Emits `NOT NULL`; checked by the model builder. |
| `nullable` | bool | DDL + model | `nullable: false` also emits `NOT NULL`; otherwise `Option<T>`. |
| `unique` | bool | DDL | Emits a `UNIQUE` constraint. |
| `default` | scalar | DDL + model | Column default (string/number/bool). |
| `foreign_key` | string | DDL | `"table.column"` (or `"table"`, defaulting to `id`). |
| `on_delete` | enum | DDL | FK action: `cascade`, `restrict`, `set_null`, `set_default`, `no_action`. |
| `on_update` | enum | DDL | Same actions as `on_delete`. |
| `lang_type` | string | model | Override the generated Rust type (e.g. `Option<u8>`). |
| `min` / `max` | number | metadata | Intended numeric range — **not enforced**. |
| `min_length` / `max_length` | integer | metadata | Intended string length — **not enforced**. |
| `pattern` | string | metadata | Intended regex — **not enforced**. |
| `validation` | string \| map | metadata | Named validator (`email`) or rule map — **not enforced**. |
| `computed` | string | metadata | Marks a derived field — not generated. |
| `hidden` | bool | metadata | Intended to hide from serialization — **not enforced**. |
| `postgres_type_name` | string | metadata | Named Postgres enum type hint. |
| `transitions` | map | metadata | Intended enum state transitions — not enforced. |
| `ai` | string | metadata | AI hint, emitted as a doc comment. |
| `example` | any | metadata | Example value (documentation). |
| `enum`, `index`, `indexed`, `search_weight`, `column_comment` | — | ignored | *(accepted, parsed, no effect)* |

`NOT NULL` is emitted when `required: true`, `nullable: false`, or the field
is the primary key.

> **Validation is not yet enforced.** `min`, `max`, `min_length`,
> `max_length`, `pattern`, and `validation` are recorded as schema metadata and
> surfaced to tooling, but the generated models do not validate against them at
> runtime. Enforce these in your own code (e.g. a controller `before` hook)
> until runtime validation lands.

## Type system

`type` is a string. Three forms are supported.

### Simple types

These map to dialect-correct SQL (see the per-dialect table in the
[Database guide](database.md#generating-sql-ddl)):

| Schema type | Notes |
|-------------|-------|
| `tinyint`, `smallint`, `mediumint`, `int` / `integer`, `bigint` | Integer family; width preserved on MySQL, widened sensibly elsewhere. |
| `serial` | Auto-increment integer. |
| `boolean` / `bool` | `TINYINT(1)` on MySQL, `BOOLEAN` elsewhere. |
| `decimal` / `numeric` | Fixed-point (prefer the parameterized form below). |
| `float`, `double` | Floating point. |
| `string` / `text`, `mediumtext`, `longtext`, `tinytext` | Text. |
| `date`, `time`, `datetime`, `timestamp` | Temporal. |
| `json`, `jsonb` | `JSONB` on Postgres, `JSON` on MySQL, `TEXT` on SQLite. |
| `uuid` | `UUID` on Postgres, `CHAR(36)` on MySQL, `TEXT` on SQLite. |
| `blob` / `binary` | Binary data. |

Native database type names produced by `db generate-schema` (e.g. `mediumint`)
are accepted directly, so round-tripping an existing database works.

### Parameterized types

```yaml
username: { type: string(150) }     # VARCHAR(150)
code:     { type: char(8) }         # CHAR(8)
price:    { type: decimal(10,2) }   # DECIMAL(10,2)
```

### Enum types

Enums require a sibling `values` list:

```yaml
status:
  type: enum
  values: [active, inactive, pending]
  default: pending
```

This generates a native `ENUM(...)` on MySQL and a
`TEXT CHECK (status IN (...))` constraint on Postgres and SQLite.

## Relations

```yaml
relations:
  has_many:
    posts:
      model: Posts
      local_field: id
      foreign_field: author_id
  belongs_to:
    company:
      model: Companies
      local_field: company_id
      foreign_field: id
      on_delete: set_null
```

| Relation | Required fields | Optional |
|----------|-----------------|----------|
| `has_many` | `model`, `local_field`, `foreign_field` | `cascade`, `ai` |
| `has_one` | `model`, `local_field`, `foreign_field` | `cascade`, `ai` |
| `belongs_to` | `model`, `local_field`, `foreign_field` | `on_delete`, `on_update`, `ai` |
| `many_to_many` | `model`, `through`, `local_through_field`, `foreign_through_field`, `local_field`, `foreign_field` | `ai` |

Relations drive **model** generation. Foreign-key DDL comes from
a field's `foreign_key` key, not from `relations`.

## Indexes

Three forms:

```yaml
indexes:
  - email                              # single-column
  - [last_name, first_name]            # composite
  - fields: [email]                    # detailed
    unique: true
    type: btree
```

## Table-level constraints

> **Reserved — not yet enforced.** A `constraints:` block parses but is
> currently **ignored** by both SQL and model generation. It is reserved for a
> future release; don't rely on it for validation today. Use field-level keys
> (`min`, `max`, `min_length`, `max_length`, `pattern`, `validation`) instead.

The reserved shape, for forward-compatibility:

```yaml
constraints:
  - field: age
    min: 18
    message: "Must be at least 18"
  - sql: "price >= 0"
    message: "Price cannot be negative"
```

## Global metadata (`_meta.yaml`)

Optional. Provides defaults and informational metadata for the schema set.
Only these keys are honored:

```yaml
version: "1.0"
database_type: mysql          # default dialect for SQL generation
database_name: myapp
description: "Application schema"
ai_context: "Guidance for AI assistants working with this schema"
```

> The SQL generator detects the target dialect from `description` /
> `database_type`. Set `database_type` (or mention the dialect in
> `description`) so the right SQL flavour is produced. Any other keys in
> `_meta.yaml` are ignored.

## Complete example

```yaml
# schemas/users.yaml
Users:
  table: users
  version: 1
  description: Application user accounts
  fields:
    id:
      type: mediumint
      primary_key: true
      auto: true
      required: true

    email:
      type: string(150)
      required: true
      unique: true
      validation: email

    password_hash:
      type: string(255)
      required: true
      hidden: true

    role:
      type: enum
      values: [admin, user, moderator]
      default: user
      required: true

    manager_id:
      type: mediumint
      foreign_key: users.id
      on_delete: set_null

    is_active:
      type: boolean
      default: true

    created_at:
      type: timestamp

  relations:
    belongs_to:
      manager:
        model: Users
        local_field: manager_id
        foreign_field: id

  indexes:
    - email
```

Generate from it:

```bash
rustf-cli schema generate models       # -> src/models/base/users.inc.rs
rustf-cli schema generate sql          # -> sql/schema.sql
```
