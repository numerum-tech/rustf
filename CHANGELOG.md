# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0-rc1] - 2025-12-10

### Added
- **Module System**: New explicit developer control for module registration with `ModuleRegistry`.
- **CLI Improvements**: Simplified module generation with `rustf-cli`, added `--shared` flag.
- **Middleware**: Dual-phase middleware architecture (Inbound/Outbound).
- **Workers**: New `rustf-cli new worker` command and simplified worker templates.
- **Performance**: Significant log cleanup in database and view layers (removed ~46 debug logs).
- **Core**: Framework prelude added to utility modules.

### Changed
- Refactored `auto_modules!` macro to specified declaration-only behavior.
- Simplified environment configuration from 4 to 2 environments.
- Optimized database adapters (MySQL, PostgreSQL, SQLite) by removing excessive debug logging.

### Fixed
- Middleware template dual-phase architecture implementation.
- Various test failures in configuration and core modules.
- Compilation warnings in sample application.

### Removed
- Removed automatic `MODULE::init()` calls in favor of explicit initialization.
- Removed deprecated CLI flags (`--module-type`).
