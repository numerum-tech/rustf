# Database & Schemas

RustF’s CLI supports a database-first workflow and a schema-first workflow.

## Database-First

Start from an existing database:

```bash
rustf-cli db generate-schema -o schemas
rustf-cli schema generate models -o src/models
```

`db generate-schema` writes YAML schema files and, by default, a full DDL dump:

```text
schemas/_database_dump.sql
```

That SQL dump is the canonical source-controlled snapshot of the live database
structure.

## Schema-First

Start from YAML schema files:

```bash
rustf-cli schema validate
rustf-cli schema generate models
rustf-cli schema generate sql
```

## Most Useful Commands

### Inspect the Database

```bash
rustf-cli db test-connection
rustf-cli db list-tables --metadata
rustf-cli db describe users
```

### Sync Schemas

```bash
rustf-cli db diff-schema schemas/users.yaml
rustf-cli schema check-consistency
```

### Generate Code

```bash
rustf-cli schema generate models --force
rustf-cli schema generate sql --output sql
```

## Backups and Safety

Commands with `--force` create backups under `.rustf/backups/` before
overwriting generated artifacts.

## Related Documentation

- [Database Integration](../guides/database.md)
- [Schema Format Reference](../guides/schema-format.md)
- [Definitions System](../guides/schemas.md)
