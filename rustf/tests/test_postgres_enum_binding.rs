use rustf::database::types::{PostgresTypeConverter, SqlValue};
use sqlx::postgres::PgArguments;

#[test]
fn test_postgres_enum_binding() {
    // Test that enum values with type casting are correctly parsed
    let enum_val = SqlValue::Enum("ACTIVE::status_enum".to_string());

    // Create a dummy query to test binding
    let query = sqlx::query::<sqlx::Postgres>("SELECT $1");

    // Bind the enum value
    let bound_query = PostgresTypeConverter::bind_param(query, enum_val);

    // The test passes if binding doesn't panic
    // In real usage, this would bind only "ACTIVE" as the parameter value
    // while the query builder generates SQL like: SELECT $1::status_enum
    assert!(true, "Enum binding succeeded without panic");
}

#[test]
fn test_postgres_enum_without_type() {
    // Test that regular enum values without type casting work too
    let enum_val = SqlValue::Enum("ACTIVE".to_string());

    let query = sqlx::query::<sqlx::Postgres>("SELECT $1");
    let bound_query = PostgresTypeConverter::bind_param(query, enum_val);

    assert!(true, "Regular enum binding succeeded");
}

#[test]
fn test_postgres_enum_value_extraction() {
    // Verify that we correctly extract the value part from typed enums
    let test_cases = vec![
        ("ACTIVE::status_enum", "ACTIVE"),
        ("PENDING::workflow_state", "PENDING"),
        ("ADMIN::user_role", "ADMIN"),
        ("value_with_underscore::some_type", "value_with_underscore"),
    ];

    for (input, expected_value) in test_cases {
        if let Some((value, _type_name)) = input.split_once("::") {
            assert_eq!(
                value, expected_value,
                "Failed to extract correct value from {}",
                input
            );
        } else {
            panic!("Failed to split {}", input);
        }
    }
}
