# RustF Enum Types Implementation Plan

## Executive Summary
Transform the current string-based enum handling into type-safe Rust enums, leveraging the types section in YAML schemas to generate proper Rust types that integrate seamlessly with the database layer.

## Current State (As of 2025-01-05)
- ✅ Types section added to schema generation
- ✅ PostgreSQL array types properly detected (`array<currency>` instead of `json`)
- ✅ Array default values correctly parsed
- ❌ Enums still represented as strings in generated models
- ❌ Arrays of enums are `serde_json::Value` instead of `Vec<EnumType>`
- ❌ No compile-time type safety for enum values

## Proposed Architecture

### 1. Generated Types Module Structure
```
src/models/
├── types/
│   ├── mod.rs           # Module exports and common traits
│   ├── generated.rs     # Auto-generated enum definitions
│   └── custom.rs        # User-defined type extensions (optional)
├── base/
│   └── *.inc.rs         # Generated model bases (updated to use enums)
└── *.rs                 # Model wrappers with business logic
```

### 2. Generated Enum Example
```rust
// src/models/types/generated.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "clearing_system_type", rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ClearingSystemType {
    RTGS,
    ACH,
    NEFT,
    IMPS,
    SEPA,
    SWIFT,
    TARGET2,
    CHIPS,
    FEDWIRE,
    CUSTOM,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "currency", rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Currency {
    XOF,
    USD,
    EUR,
    GBP,
    GHS,
    NGN,
    KES,
}
```

## Implementation Phases

### Phase 1: Schema Parser Enhancement (2-3 days)
**Files to modify:**
- `/rustf-schema/src/lib.rs` - Add types field to Schema struct
- `/rustf-schema/src/types.rs` - Define TypeDefinition struct
- `/rustf-cli/src/commands/schema/parser.rs` - Parse types section

**Tasks:**
1. Add `types: HashMap<String, TypeDefinition>` to Schema struct
2. Create TypeDefinition struct:
   ```rust
   pub struct TypeDefinition {
       pub kind: TypeKind,  // Enum, Struct, Union
       pub values: Option<Vec<String>>,  // For enums
       pub fields: Option<HashMap<String, FieldDefinition>>,  // For structs
       pub description: Option<String>,
   }
   ```
3. Update YAML parser to extract types section
4. Validate type references in fields

### Phase 2: Code Generator for Types (3-4 days)
**Files to create:**
- `/rustf-cli/templates/types/enum.rs.template` - Enum generation template
- `/rustf-cli/src/commands/schema/type_generator.rs` - Type generation logic

**Files to modify:**
- `/rustf-cli/src/commands/schema/postgres.rs` - Call type generator
- `/rustf-cli/src/commands/schema/mysql.rs` - Call type generator
- `/rustf-cli/src/commands/schema/sqlite.rs` - Call type generator

**Tasks:**
1. Create enum generation template with:
   - sqlx::Type derive for database integration
   - Serde derives with proper naming
   - Display and FromStr implementations
   - Conversion methods (as_str, from_str)
2. Generate types/mod.rs with exports
3. Handle naming conflicts (reserved words, duplicates)
4. Generate documentation from schema

### Phase 3: Model Generation Updates (2-3 days)
**Files to modify:**
- `/rustf-cli/templates/models/base_model.rs.template` - Use enum types
- `/rustf-cli/src/commands/schema/field_mapper.rs` - Map to enum types

**Tasks:**
1. Update field type mapping:
   - `clearing_system_type` → `Option<types::ClearingSystemType>`
   - `array<currency>` → `Option<Vec<types::Currency>>`
2. Update imports to include types module
3. Remove string constants for enums
4. Update builder methods to accept enum types

### Phase 4: Database Layer Integration (4-5 days)
**Files to modify:**
- `/rustf/src/database/types/postgres_converter.rs` - Enum conversions
- `/rustf/src/database/types/mysql_converter.rs` - Enum conversions
- `/rustf/src/database/types/sqlite_converter.rs` - Enum conversions
- `/rustf/src/models/query_builder.rs` - Handle enum values

**Tasks:**
1. Extend SqlValue enum:
   ```rust
   pub enum SqlValue {
       // ... existing variants
       Enum { type_name: String, value: String },
       ArrayEnum { type_name: String, values: Vec<String> },
   }
   ```
2. Implement enum to string conversion for queries
3. Implement string to enum conversion for results
4. Handle array of enums in PostgreSQL
5. Add validation for enum values

### Phase 5: Migration Strategy (1-2 days)
**Files to create:**
- `/rustf-cli/src/commands/migrate/enum_migration.rs` - Migration tool
- `/rustf/docs/ENUM_MIGRATION_GUIDE.md` - User guide

**Tasks:**
1. Create migration tool to update existing code
2. Provide compatibility layer for string-based API
3. Generate migration script for each project
4. Document breaking changes and migration path

### Phase 6: Testing & Validation (2-3 days)
**Files to create:**
- `/rustf/tests/enum_types_test.rs` - Integration tests
- `/rustf-example/tests/models_with_enums_test.rs` - Example tests

**Tasks:**
1. Test enum CRUD operations
2. Test array of enums
3. Test invalid enum value handling
4. Test database round-trip
5. Test migration from strings to enums
6. Performance benchmarks

## Database-Specific Considerations

### PostgreSQL
- Native enum type support via `CREATE TYPE`
- Arrays work with `_typename` convention
- Use sqlx::Type for automatic conversion
- Consider enum alterations in migrations

### MySQL
- No native enum support (uses CHECK constraints)
- Store as strings with validation
- Arrays stored as JSON
- Implement custom validation layer

### SQLite
- No native enum or array support
- Store enums as TEXT with CHECK constraints
- Arrays as JSON strings
- Full validation in application layer

## Breaking Changes & Migration

### Breaking Changes:
1. Field types change from `Option<String>` to `Option<EnumType>`
2. Array fields change from `serde_json::Value` to `Vec<EnumType>`
3. String constants removed in favor of enum variants
4. Builder methods signature changes

### Migration Path:
1. **Phase 1**: Add new enum-based API alongside string API
2. **Phase 2**: Deprecate string-based methods
3. **Phase 3**: Provide automated migration tool
4. **Phase 4**: Remove deprecated APIs in next major version

### Compatibility Layer:
```rust
impl PaymentSchemes {
    // New enum-based method
    pub fn set_clearing_system(&mut self, value: ClearingSystemType) { ... }
    
    // Deprecated string-based method
    #[deprecated(note = "Use set_clearing_system with enum type")]
    pub fn set_clearing_system_str(&mut self, value: &str) -> Result<()> {
        let enum_val = ClearingSystemType::from_str(value)?;
        self.set_clearing_system(enum_val);
        Ok(())
    }
}
```

## Benefits

### For Developers:
- **Compile-time safety**: Invalid enum values caught at compilation
- **IDE support**: Auto-completion for enum variants
- **Self-documenting code**: Types clearly show valid values
- **Refactoring safety**: Rename enum variants with confidence

### For AI Agents:
- **Clear type information**: Types section provides complete enum definitions
- **Predictable patterns**: Consistent enum naming and structure
- **Better code generation**: Can generate valid enum values
- **Reduced errors**: Can't generate invalid string values

### For Performance:
- **Smaller memory footprint**: Enums vs strings
- **Faster comparisons**: Integer vs string comparison
- **Better optimization**: Compiler can optimize enum matches

## Risk Assessment

### Risks:
1. **Large breaking change** affecting all existing models
2. **Database migration complexity** for existing data
3. **Learning curve** for developers used to strings
4. **Potential sqlx version conflicts**

### Mitigations:
1. Provide comprehensive migration tooling
2. Extensive testing before release
3. Clear documentation and examples
4. Phased rollout with compatibility layer

## Timeline Estimate

- **Total Duration**: 15-20 working days
- **Phase 1**: 2-3 days (Schema Parser)
- **Phase 2**: 3-4 days (Type Generator)
- **Phase 3**: 2-3 days (Model Updates)
- **Phase 4**: 4-5 days (Database Integration)
- **Phase 5**: 1-2 days (Migration)
- **Phase 6**: 2-3 days (Testing)

## Success Criteria

1. ✅ All enum types from schema are generated as Rust enums
2. ✅ Models use type-safe enums instead of strings
3. ✅ Database operations work seamlessly with enums
4. ✅ Arrays of enums are properly typed
5. ✅ Migration tool successfully converts existing projects
6. ✅ No performance regression
7. ✅ All tests pass on PostgreSQL, MySQL, and SQLite

## Next Steps

1. **Review and approve** this plan
2. **Create feature branch** `feature/type-safe-enums`
3. **Start with Phase 1** (Schema Parser Enhancement)
4. **Weekly progress reviews** during implementation
5. **Beta testing** with select projects
6. **Documentation** and migration guides
7. **Release as v2.0** (major version due to breaking changes)

## Open Questions

1. Should we support enum variants with associated data?
2. How to handle enum evolution (adding/removing variants)?
3. Should we generate From/Into traits for all enums?
4. Support for bitflags/multi-select enums?
5. Custom validation rules per enum type?

## References

- [SQLx Type Derives](https://docs.rs/sqlx/latest/sqlx/derive.Type.html)
- [PostgreSQL Enum Types](https://www.postgresql.org/docs/current/datatype-enum.html)
- [Serde Enum Representations](https://serde.rs/enum-representations.html)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)

---

*Document created: 2025-01-05*
*Last updated: 2025-01-05*
*Status: DRAFT - Pending Review*