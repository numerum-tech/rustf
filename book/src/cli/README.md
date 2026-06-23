# RustF CLI

`rustf-cli` is the fastest way to create, inspect, and maintain a RustF
project. It is not just a scaffolder: it also handles schema workflows,
database inspection, validation, project analysis, and MCP serving for AI
tools.

## Start Here

If you are new to the CLI, this is the shortest useful path:

1. Install it: see [Installation](../getting-started/installation.md)
2. Create an app: [Create a Project](new-project.md)
3. Generate pieces as the app grows: [Generate Components](generate-components.md)
4. Add database workflow: [Database & Schemas](database-schemas.md)
5. Audit the project before commits: [Analysis & Validation](analysis-validation.md)

## Most Common Commands

```bash
rustf-cli new project my-app
rustf-cli new controller -n users
rustf-cli new crud -n posts
rustf-cli schema generate models
rustf-cli analyze project --detailed
rustf-cli validate
```

## Task-Focused Guides

- [Create a Project](new-project.md) - bootstrap a new RustF app
- [Generate Components](generate-components.md) - controllers, middleware, modules, workers, CRUD
- [Database & Schemas](database-schemas.md) - schema generation, code generation, database inspection
- [Analysis & Validation](analysis-validation.md) - route checks, project inspection, consistency validation
- [Serve & MCP](serve-mcp.md) - local CLI server and AI-tool integration
- [Full Reference](full-reference.md) - one-page command catalog

## What the CLI Covers

- Project scaffolding
- Component generation
- Database introspection
- YAML schema workflows
- Route and project analysis
- Validation and consistency checks
- Local MCP serving for AI assistants

## Related Guides

- [Getting Started](../getting-started/README.md)
- [Database Integration](../guides/database.md)
- [Schema Format Reference](../guides/schema-format.md)
