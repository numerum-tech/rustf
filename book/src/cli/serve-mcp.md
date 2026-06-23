# Serve & MCP

`rustf-cli` can run a local serving endpoint for AI tooling and project
inspection workflows.

## Start the Server

```bash
rustf-cli serve start
```

Common options:

```bash
rustf-cli serve start --bind 0.0.0.0 --port 8080
rustf-cli serve start --allow-writes
```

Use `--allow-writes` only when you explicitly want remote tools to be able to
mutate the project. The default posture is safer.

## Manage Running Instances

```bash
rustf-cli serve list
rustf-cli serve stop 8080
```

## Related Commands

```bash
rustf-cli query route "/users/{id}"
rustf-cli query model-metadata User --format json
```

The `query` family is useful when an AI tool or human reviewer needs one
specific item instead of a full project analysis.

## Security Notes

- Prefer the default read-oriented mode
- Avoid exposing a write-enabled endpoint to untrusted networks
- Bind to loopback unless you have a deliberate remote workflow

## Full Command Catalog

See [Full Reference](full-reference.md) for the complete list of `serve`,
`query`, and related commands.
