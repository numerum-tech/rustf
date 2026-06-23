# Full CLI Reference

Use this page as an index. For task-first guidance, start from
[CLI Overview](README.md).

## Core Command Groups

### Project and Code Generation

```bash
rustf-cli new project <PROJECT_NAME> [--path <DIR>] [--force]
rustf-cli new controller -n <NAMES> [--crud] [--routes]
rustf-cli new middleware -n <NAME> [--auth] [--logging] [-p <PRIORITY>]
rustf-cli new module -n <NAME> [--shared] [--with-methods]
rustf-cli new worker -n <NAME>
rustf-cli new event -n <NAME> [--lifecycle] [--custom]
rustf-cli new crud -n <PLURAL_NAME>
```

### Analysis and Validation

```bash
rustf-cli analyze <SUBCOMMAND>
rustf-cli validate [--fix] [-w, --watch]
rustf-cli export [-f <FORMAT>] [--include-code] [-o <FILE>]
```

### Database and Schema

```bash
rustf-cli db <SUBCOMMAND>
rustf-cli schema <SUBCOMMAND>
rustf-cli translations <SUBCOMMAND>
```

### Query and Serve

```bash
rustf-cli query <ITEM_TYPE> <ITEM_NAME> [-f <FORMAT>]
rustf-cli serve <SUBCOMMAND>
```

### Performance

```bash
rustf-cli perf <SUBCOMMAND>
```

## Global Options

```bash
-P, --project <PATH>
-v, --verbose
-h, --help
-V, --version
```

## Deep Reference

The original monolithic command guide is still available here:

- [Legacy Complete CLI Guide](../advanced/cli.md)

Keep that page for exhaustive command-by-command details. Use the new CLI
section for normal navigation.
