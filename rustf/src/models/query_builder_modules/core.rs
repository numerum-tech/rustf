//! Core query building logic for RustF framework
//!
//! This module contains the database-agnostic query building structures
//! and methods, working with the dialect system for database-specific SQL generation.

use super::dialects::{create_dialect, DatabaseBackend, QueryError, SqlDialect};
use crate::database::types::SqlValue;
use anyhow::Result;

// Note: SqlValue is now imported from crate::database::types::SqlValue
// The type system has been unified in the database::types module

/// Main query builder that works with any database
pub struct QueryBuilder {
    pub(crate) dialect: Box<dyn SqlDialect>,
    pub(crate) backend: DatabaseBackend,
    pub(crate) table: Option<String>,
    pub(crate) table_alias: Option<String>,
    pub(crate) select_columns: Vec<String>,
    pub(crate) where_conditions: Vec<WhereCondition>,
    pub(crate) joins: Vec<JoinClause>,
    pub(crate) order_by: Vec<OrderByClause>,
    pub(crate) limit: Option<i64>,
    pub(crate) offset: Option<i64>,
    pub(crate) _group_by: Vec<String>,
    pub(crate) _having_conditions: Vec<WhereCondition>,
    pub(crate) returning: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct WhereCondition {
    pub column: String,
    pub operator: String,
    pub value: SqlValue,
    pub connector: WhereConnector,
}

#[derive(Clone, Debug)]
pub enum WhereConnector {
    And,
    Or,
}

#[derive(Clone, Debug)]
pub struct JoinClause {
    pub join_type: JoinType,
    pub table: String,
    pub on_condition: String,
}

#[derive(Clone, Debug)]
pub enum JoinType {
    Inner,
    Left,
    Right,
    Full,
}

#[derive(Clone, Debug)]
pub struct OrderByClause {
    pub column: String,
    pub direction: OrderDirection,
}

#[derive(Clone, Debug)]
pub enum OrderDirection {
    Asc,
    Desc,
}

impl QueryBuilder {
    /// Create a new query builder for the specified database backend
    pub fn new(backend: DatabaseBackend) -> Self {
        let dialect = create_dialect(backend);

        QueryBuilder {
            dialect,
            backend,
            table: None,
            table_alias: None,
            select_columns: vec!["*".to_string()],
            where_conditions: Vec::new(),
            joins: Vec::new(),
            order_by: Vec::new(),
            limit: None,
            offset: None,
            _group_by: Vec::new(),
            _having_conditions: Vec::new(),
            returning: Vec::new(),
        }
    }

    /// Set the table to query from
    pub fn from<S: Into<String>>(mut self, table: S) -> Self {
        self.table = Some(table.into());
        self
    }

    /// Set an alias for the main table
    ///
    /// # Example
    /// ```
    /// let query = QueryBuilder::new(DatabaseBackend::Postgres)
    ///     .from("users")
    ///     .as_alias("u")
    ///     .select(vec!["u.id", "u.name"])
    ///     .where_eq("u.is_active", true);
    /// ```
    pub fn as_alias<S: Into<String>>(mut self, alias: S) -> Self {
        self.table_alias = Some(alias.into());
        self
    }

    /// Set the columns to select
    pub fn select<I, S>(mut self, columns: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.select_columns = columns.into_iter().map(|s| s.into()).collect();
        if self.select_columns.is_empty() {
            self.select_columns = vec!["*".to_string()];
        }
        self
    }

    /// Add WHERE column = value condition
    pub fn where_eq<S: Into<String>, V: Into<SqlValue>>(mut self, column: S, value: V) -> Self {
        self.where_conditions.push(WhereCondition {
            column: column.into(),
            operator: "=".to_string(),
            value: value.into(),
            connector: WhereConnector::And,
        });
        self
    }

    /// Add WHERE column != value condition
    pub fn where_ne<S: Into<String>, V: Into<SqlValue>>(mut self, column: S, value: V) -> Self {
        self.where_conditions.push(WhereCondition {
            column: column.into(),
            operator: "!=".to_string(),
            value: value.into(),
            connector: WhereConnector::And,
        });
        self
    }

    /// Add WHERE column > value condition
    pub fn where_gt<S: Into<String>, V: Into<SqlValue>>(mut self, column: S, value: V) -> Self {
        self.where_conditions.push(WhereCondition {
            column: column.into(),
            operator: ">".to_string(),
            value: value.into(),
            connector: WhereConnector::And,
        });
        self
    }

    /// Add WHERE column < value condition
    pub fn where_lt<S: Into<String>, V: Into<SqlValue>>(mut self, column: S, value: V) -> Self {
        self.where_conditions.push(WhereCondition {
            column: column.into(),
            operator: "<".to_string(),
            value: value.into(),
            connector: WhereConnector::And,
        });
        self
    }

    /// Add WHERE column IS NULL condition
    pub fn where_null<S: Into<String>>(mut self, column: S) -> Self {
        self.where_conditions.push(WhereCondition {
            column: column.into(),
            operator: "IS".to_string(),
            value: SqlValue::Null,
            connector: WhereConnector::And,
        });
        self
    }

    /// Add WHERE column IS NOT NULL condition
    pub fn where_not_null<S: Into<String>>(mut self, column: S) -> Self {
        self.where_conditions.push(WhereCondition {
            column: column.into(),
            operator: "IS NOT".to_string(),
            value: SqlValue::Null,
            connector: WhereConnector::And,
        });
        self
    }

    /// WHERE column IN (values)
    pub fn where_in<S: Into<String>, V: Into<SqlValue>>(
        mut self,
        column: S,
        values: Vec<V>,
    ) -> Self {
        // Special handling for IN clause - store as comma-separated string
        let value_strings: Vec<String> = values
            .into_iter()
            .map(|v| {
                let val: SqlValue = v.into();
                val.to_sql_string()
            })
            .collect();

        self.where_conditions.push(WhereCondition {
            column: column.into(),
            operator: format!("IN ({})", value_strings.join(", ")),
            value: SqlValue::String("".to_string()), // Placeholder - values already in operator
            connector: WhereConnector::And,
        });
        self
    }

    /// Add ORDER BY clause
    pub fn order_by<S: Into<String>>(mut self, column: S, direction: OrderDirection) -> Self {
        self.order_by.push(OrderByClause {
            column: column.into(),
            direction,
        });
        self
    }

    /// Add LIMIT clause
    pub fn limit(mut self, limit: i64) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Add OFFSET clause
    pub fn offset(mut self, offset: i64) -> Self {
        self.offset = Some(offset);
        self
    }

    /// Add RETURNING clause (PostgreSQL/SQLite)
    pub fn returning<S: Into<String>>(mut self, columns: Vec<S>) -> Self {
        self.returning = columns.into_iter().map(|c| c.into()).collect();
        self
    }

    /// Add single column to RETURNING clause
    pub fn returning_column<S: Into<String>>(mut self, column: S) -> Self {
        self.returning.push(column.into());
        self
    }

    /// Build the SQL query string with dialect-specific syntax
    pub fn build(&self) -> Result<(String, Vec<SqlValue>), QueryError> {
        if self.table.is_none() {
            return Err(QueryError::MissingClause {
                clause: "from".to_string(),
            });
        }

        let mut sql = String::new();
        let mut params = Vec::new();
        let param_count = 1;

        // SELECT clause
        sql.push_str("SELECT ");
        sql.push_str(&self.select_columns.join(", "));

        // FROM clause with quoted identifier and optional alias
        sql.push_str(" FROM ");
        sql.push_str(&self.dialect.quote_identifier(self.table.as_ref().unwrap()));
        if let Some(alias) = &self.table_alias {
            sql.push_str(" AS ");
            sql.push_str(&self.dialect.quote_identifier(alias));
        }

        // JOIN clauses
        for join in &self.joins {
            match join.join_type {
                JoinType::Inner => sql.push_str(" INNER JOIN "),
                JoinType::Left => sql.push_str(" LEFT JOIN "),
                JoinType::Right => sql.push_str(" RIGHT JOIN "),
                JoinType::Full => {
                    if matches!(
                        self.backend,
                        DatabaseBackend::MySQL | DatabaseBackend::MariaDB
                    ) {
                        return Err(QueryError::UnsupportedFeature {
                            backend: self.backend,
                            feature: "FULL JOIN".to_string(),
                        });
                    }
                    sql.push_str(" FULL JOIN ");
                }
            }

            // Handle table alias in JOIN clause (e.g., "users AS u" or "users as u")
            let table_parts: Vec<&str> = join.table.split_whitespace().collect();
            if table_parts.len() == 3 && (table_parts[1].eq_ignore_ascii_case("as")) {
                // Format: "table AS alias"
                sql.push_str(&self.dialect.quote_identifier(table_parts[0]));
                sql.push_str(" AS ");
                sql.push_str(table_parts[2]); // Alias should not be quoted
            } else if table_parts.len() == 2 {
                // Could be "table alias" without AS keyword
                sql.push_str(&self.dialect.quote_identifier(table_parts[0]));
                sql.push(' ');
                sql.push_str(table_parts[1]); // Alias should not be quoted
            } else {
                // No alias, just quote the table name
                sql.push_str(&self.dialect.quote_identifier(&join.table));
            }

            sql.push_str(" ON ");
            sql.push_str(&join.on_condition);
        }

        // WHERE clause with proper enum handling
        let (where_sql, where_params, _new_param_count) = self.build_where_clause(param_count);
        sql.push_str(&where_sql);
        params.extend(where_params);

        // GROUP BY clause
        if !self._group_by.is_empty() {
            sql.push_str(" GROUP BY ");
            let group_clauses: Vec<String> = self
                ._group_by
                .iter()
                .map(|col| self.dialect.quote_identifier(col))
                .collect();
            sql.push_str(&group_clauses.join(", "));
        }

        // ORDER BY clause
        if !self.order_by.is_empty() {
            sql.push_str(" ORDER BY ");
            let order_clauses: Vec<String> = self
                .order_by
                .iter()
                .map(|clause| {
                    let direction = match clause.direction {
                        OrderDirection::Asc => "ASC",
                        OrderDirection::Desc => "DESC",
                    };
                    format!(
                        "{} {}",
                        self.dialect.quote_identifier(&clause.column),
                        direction
                    )
                })
                .collect();
            sql.push_str(&order_clauses.join(", "));
        }

        // LIMIT/OFFSET clause (database-specific)
        sql.push_str(&self.dialect.limit_syntax(self.limit, self.offset));

        Ok((sql, params))
    }

    /// Add WHERE column >= value condition
    pub fn where_gte<S: Into<String>, V: Into<SqlValue>>(mut self, column: S, value: V) -> Self {
        self.where_conditions.push(WhereCondition {
            column: column.into(),
            operator: ">=".to_string(),
            value: value.into(),
            connector: WhereConnector::And,
        });
        self
    }

    /// Add WHERE column <= value condition
    pub fn where_lte<S: Into<String>, V: Into<SqlValue>>(mut self, column: S, value: V) -> Self {
        self.where_conditions.push(WhereCondition {
            column: column.into(),
            operator: "<=".to_string(),
            value: value.into(),
            connector: WhereConnector::And,
        });
        self
    }

    /// Add WHERE column LIKE value condition
    pub fn where_like<S: Into<String>, V: Into<SqlValue>>(mut self, column: S, value: V) -> Self {
        self.where_conditions.push(WhereCondition {
            column: column.into(),
            operator: "LIKE".to_string(),
            value: value.into(),
            connector: WhereConnector::And,
        });
        self
    }

    /// Add WHERE column NOT LIKE value condition
    pub fn where_not_like<S: Into<String>>(mut self, column: S, pattern: S) -> Self {
        self.where_conditions.push(WhereCondition {
            column: column.into(),
            operator: "NOT LIKE".to_string(),
            value: SqlValue::String(pattern.into()),
            connector: WhereConnector::And,
        });
        self
    }

    /// WHERE column NOT IN (values)
    pub fn where_not_in<S: Into<String>, V: Into<SqlValue>>(
        mut self,
        column: S,
        values: Vec<V>,
    ) -> Self {
        let value_strings: Vec<String> = values
            .into_iter()
            .map(|v| {
                let val: SqlValue = v.into();
                val.to_sql_string()
            })
            .collect();

        self.where_conditions.push(WhereCondition {
            column: column.into(),
            operator: format!("NOT IN ({})", value_strings.join(", ")),
            value: SqlValue::String("".to_string()), // Placeholder - values already in operator
            connector: WhereConnector::And,
        });
        self
    }

    /// WHERE column BETWEEN start AND end
    pub fn where_between<S: Into<String>, V: Into<SqlValue>>(
        mut self,
        column: S,
        start: V,
        end: V,
    ) -> Self {
        let start_val: SqlValue = start.into();
        let start_str = start_val.to_sql_string();

        let end_val: SqlValue = end.into();
        let end_str = end_val.to_sql_string();

        self.where_conditions.push(WhereCondition {
            column: column.into(),
            operator: "BETWEEN".to_string(),
            value: SqlValue::String(format!("{} AND {}", start_str, end_str)),
            connector: WhereConnector::And,
        });
        self
    }

    /// OR WHERE column = value condition
    pub fn or_where_eq<S: Into<String>, V: Into<SqlValue>>(mut self, column: S, value: V) -> Self {
        self.where_conditions.push(WhereCondition {
            column: column.into(),
            operator: "=".to_string(),
            value: value.into(),
            connector: WhereConnector::Or,
        });
        self
    }

    /// OR WHERE column != value condition
    pub fn or_where_ne<S: Into<String>, V: Into<SqlValue>>(mut self, column: S, value: V) -> Self {
        self.where_conditions.push(WhereCondition {
            column: column.into(),
            operator: "!=".to_string(),
            value: value.into(),
            connector: WhereConnector::Or,
        });
        self
    }

    /// OR WHERE column > value condition
    pub fn or_where_gt<S: Into<String>, V: Into<SqlValue>>(mut self, column: S, value: V) -> Self {
        self.where_conditions.push(WhereCondition {
            column: column.into(),
            operator: ">".to_string(),
            value: value.into(),
            connector: WhereConnector::Or,
        });
        self
    }

    /// OR WHERE column < value condition
    pub fn or_where_lt<S: Into<String>, V: Into<SqlValue>>(mut self, column: S, value: V) -> Self {
        self.where_conditions.push(WhereCondition {
            column: column.into(),
            operator: "<".to_string(),
            value: value.into(),
            connector: WhereConnector::Or,
        });
        self
    }

    /// OR WHERE column LIKE pattern condition
    pub fn or_where_like<S: Into<String>>(mut self, column: S, pattern: S) -> Self {
        self.where_conditions.push(WhereCondition {
            column: column.into(),
            operator: "LIKE".to_string(),
            value: SqlValue::String(pattern.into()),
            connector: WhereConnector::Or,
        });
        self
    }

    /// OR WHERE column IN (values)
    pub fn or_where_in<S: Into<String>, V: Into<SqlValue>>(
        mut self,
        column: S,
        values: Vec<V>,
    ) -> Self {
        let value_strings: Vec<String> = values
            .into_iter()
            .map(|v| {
                let val: SqlValue = v.into();
                val.to_sql_string()
            })
            .collect();

        self.where_conditions.push(WhereCondition {
            column: column.into(),
            operator: format!("IN ({})", value_strings.join(", ")),
            value: SqlValue::String("".to_string()), // Placeholder - values already in operator
            connector: WhereConnector::Or,
        });
        self
    }

    /// OR WHERE column IS NULL condition
    pub fn or_where_null<S: Into<String>>(mut self, column: S) -> Self {
        self.where_conditions.push(WhereCondition {
            column: column.into(),
            operator: "IS".to_string(),
            value: SqlValue::Null,
            connector: WhereConnector::Or,
        });
        self
    }

    /// Add JOIN clause
    pub fn join<S: Into<String>>(mut self, table: S, on: S) -> Self {
        self.joins.push(JoinClause {
            join_type: JoinType::Inner,
            table: table.into(),
            on_condition: on.into(),
        });
        self
    }

    /// Add LEFT JOIN clause
    pub fn left_join<S: Into<String>>(mut self, table: S, on: S) -> Self {
        self.joins.push(JoinClause {
            join_type: JoinType::Left,
            table: table.into(),
            on_condition: on.into(),
        });
        self
    }

    /// Add RIGHT JOIN clause - returns Result for compatibility
    pub fn right_join<S: Into<String>>(mut self, table: S, on: S) -> Result<Self, QueryError> {
        self.joins.push(JoinClause {
            join_type: JoinType::Right,
            table: table.into(),
            on_condition: on.into(),
        });
        Ok(self)
    }

    /// Convert to COUNT query
    pub fn count(mut self) -> Self {
        self.select_columns = vec!["COUNT(*)".to_string()];
        self
    }

    /// COUNT specific column
    pub fn count_column<S: Into<String>>(mut self, column: S) -> Self {
        self.select_columns = vec![format!("COUNT({})", column.into())];
        self
    }

    /// Add raw WHERE clause
    pub fn where_raw<S: Into<String>>(mut self, sql: S) -> Self {
        // For raw SQL, we'll store it as a placeholder condition
        self.where_conditions.push(WhereCondition {
            column: "".to_string(), // Empty column indicates raw SQL
            operator: sql.into(),
            value: SqlValue::String("".to_string()),
            connector: WhereConnector::And,
        });
        self
    }

    /// Paginate results
    pub fn paginate(mut self, page: u32, per_page: u32) -> Self {
        let offset = (page.saturating_sub(1)) * per_page;
        self.limit = Some(per_page as i64);
        self.offset = Some(offset as i64);
        self
    }

    /// Add GROUP BY clause
    pub fn group_by<S: Into<String>>(mut self, column: S) -> Self {
        self._group_by.push(column.into());
        self
    }

    /// Helper function to generate SQL value expression for a given SqlValue
    /// Returns (sql_expression, should_bind_param)
    fn generate_value_expression(&self, value: &SqlValue, param_index: usize) -> (String, bool) {
        match value {
            SqlValue::Null => {
                // Use literal NULL for null values
                ("NULL".to_string(), false)
            }
            SqlValue::Default => {
                // Use literal DEFAULT for default values
                ("DEFAULT".to_string(), false)
            }
            SqlValue::Enum(enum_str) if enum_str.contains("::") => {
                // PostgreSQL enum with type info
                if let Some((_, pg_type)) = enum_str.split_once("::") {
                    match self.backend {
                        DatabaseBackend::Postgres => {
                            (format!("${}::{}", param_index, pg_type), true)
                        }
                        _ => (self.dialect.placeholder(param_index), true),
                    }
                } else {
                    (self.dialect.placeholder(param_index), true)
                }
            }
            _ => {
                // Regular value with placeholder
                (self.dialect.placeholder(param_index), true)
            }
        }
    }

    /// Build WHERE clause with proper enum handling for all query types
    /// Returns (sql_where_clause, params, next_param_index)
    fn build_where_clause(&self, start_param_index: usize) -> (String, Vec<SqlValue>, usize) {
        if self.where_conditions.is_empty() {
            return (String::new(), Vec::new(), start_param_index);
        }

        let mut sql = String::from(" WHERE ");
        let mut params = Vec::new();
        let mut param_count = start_param_index;

        for (i, condition) in self.where_conditions.iter().enumerate() {
            if i > 0 {
                match condition.connector {
                    WhereConnector::And => sql.push_str(" AND "),
                    WhereConnector::Or => sql.push_str(" OR "),
                }
            }

            // Handle IS NULL and IS NOT NULL specially (they don't take parameters)
            if condition.operator == "IS" || condition.operator == "IS NOT" {
                sql.push_str(&format!(
                    "{} {} NULL",
                    self.dialect.quote_identifier(&condition.column),
                    condition.operator
                ));
                // Don't add parameter or increment param_count for NULL checks
            } else if condition.operator.contains("IN (") {
                // Handle IN/NOT IN operators that have values embedded in the operator string
                sql.push_str(&format!(
                    "{} {}",
                    self.dialect.quote_identifier(&condition.column),
                    condition.operator
                ));
                // Don't add parameter since values are already in the operator string
            } else if condition.operator == "BETWEEN" {
                // Handle BETWEEN operator - value contains "X AND Y"
                sql.push_str(&format!(
                    "{} {} {}",
                    self.dialect.quote_identifier(&condition.column),
                    condition.operator,
                    condition.value.to_sql_string()
                ));
                // Don't add parameter since values are already formatted
            } else {
                // Check if the value is an enum with type info and generate proper placeholder
                let placeholder = if let SqlValue::Enum(enum_str) = &condition.value {
                    if enum_str.contains("::") && matches!(self.backend, DatabaseBackend::Postgres)
                    {
                        // Extract the type name after ::
                        if let Some((_, pg_type)) = enum_str.split_once("::") {
                            format!("{}::{}", self.dialect.placeholder(param_count), pg_type)
                        } else {
                            self.dialect.placeholder(param_count)
                        }
                    } else {
                        self.dialect.placeholder(param_count)
                    }
                } else {
                    self.dialect.placeholder(param_count)
                };

                sql.push_str(&format!(
                    "{} {} {}",
                    self.dialect.quote_identifier(&condition.column),
                    condition.operator,
                    placeholder
                ));
                params.push(condition.value.clone());
                param_count += 1;
            }
        }

        (sql, params, param_count)
    }

    /// Build an INSERT query
    pub fn build_insert(
        &self,
        data: &std::collections::HashMap<String, SqlValue>,
    ) -> Result<(String, Vec<SqlValue>)> {
        if self.table.is_none() {
            return Err(QueryError::MissingClause {
                clause: "table".to_string(),
            }
            .into());
        }

        if data.is_empty() {
            return Err(QueryError::InvalidSyntax {
                backend: self.backend,
                message: "No data provided for INSERT".to_string(),
            }
            .into());
        }

        let mut sql = String::new();

        // INSERT INTO clause
        sql.push_str("INSERT INTO ");
        sql.push_str(&self.dialect.quote_identifier(self.table.as_ref().unwrap()));

        // Collect ordered keys for consistent ordering
        let ordered_keys: Vec<String> = data.keys().cloned().collect();

        // Build columns list
        let columns: Vec<String> = ordered_keys
            .iter()
            .map(|k| self.dialect.quote_identifier(k))
            .collect();
        sql.push_str(" (");
        sql.push_str(&columns.join(", "));
        sql.push_str(") VALUES (");

        // Build values list with mix of placeholders and literal NULLs
        let mut param_index = 1;
        let mut value_parts = Vec::new();
        let mut params = Vec::new();

        for key in &ordered_keys {
            let value = &data[key];
            let (expression, should_bind) = self.generate_value_expression(value, param_index);
            value_parts.push(expression);

            if should_bind {
                params.push(value.clone());
                param_index += 1;
            }
        }

        sql.push_str(&value_parts.join(", "));
        sql.push(')');

        // Add RETURNING clause if specified
        if !self.returning.is_empty() {
            match self.backend {
                DatabaseBackend::Postgres | DatabaseBackend::SQLite => {
                    sql.push_str(" RETURNING ");
                    sql.push_str(&self.returning.join(", "));
                }
                DatabaseBackend::MySQL | DatabaseBackend::MariaDB => {
                    // MySQL doesn't support RETURNING clause
                    // Would need to use LAST_INSERT_ID() separately
                }
            }
        }

        // Log generated SQL in development mode
        #[cfg(debug_assertions)]
        {
            log::debug!("QueryBuilder INSERT SQL: {}", sql);
            log::debug!("  Parameters to bind: {:?}", params);
        }

        Ok((sql, params))
    }

    /// Build an UPDATE query
    pub fn build_update(
        &self,
        data: &std::collections::HashMap<String, SqlValue>,
    ) -> Result<(String, Vec<SqlValue>)> {
        if self.table.is_none() {
            return Err(QueryError::MissingClause {
                clause: "table".to_string(),
            }
            .into());
        }

        if data.is_empty() {
            return Err(QueryError::InvalidSyntax {
                backend: self.backend,
                message: "No data provided for UPDATE".to_string(),
            }
            .into());
        }

        let mut sql = String::new();
        let mut params = Vec::new();
        let mut param_count = 1;

        // UPDATE clause
        sql.push_str("UPDATE ");
        sql.push_str(&self.dialect.quote_identifier(self.table.as_ref().unwrap()));
        sql.push_str(" SET ");

        // SET clause using shared value expression logic
        let mut set_clauses = Vec::new();

        for (key, value) in data.iter() {
            let (expression, should_bind) = self.generate_value_expression(value, param_count);
            let clause = format!("{} = {}", self.dialect.quote_identifier(key), expression);
            set_clauses.push(clause);

            if should_bind {
                params.push(value.clone());
                param_count += 1;
            }
        }

        sql.push_str(&set_clauses.join(", "));

        // WHERE clause with proper enum handling
        let (where_sql, where_params, _new_param_count) = self.build_where_clause(param_count);
        sql.push_str(&where_sql);
        params.extend(where_params);

        // Add RETURNING clause if specified
        if !self.returning.is_empty() {
            match self.backend {
                DatabaseBackend::Postgres | DatabaseBackend::SQLite => {
                    sql.push_str(" RETURNING ");
                    sql.push_str(&self.returning.join(", "));
                }
                DatabaseBackend::MySQL | DatabaseBackend::MariaDB => {
                    // MySQL doesn't support RETURNING clause
                }
            }
        }

        Ok((sql, params))
    }

    /// Build a DELETE query
    pub fn build_delete(&self) -> Result<(String, Vec<SqlValue>)> {
        if self.table.is_none() {
            return Err(QueryError::MissingClause {
                clause: "table".to_string(),
            }
            .into());
        }

        let mut sql = String::new();
        let mut params = Vec::new();
        let param_count = 1;

        // DELETE FROM clause
        sql.push_str("DELETE FROM ");
        sql.push_str(&self.dialect.quote_identifier(self.table.as_ref().unwrap()));

        // WHERE clause with proper enum handling
        let (where_sql, where_params, _new_param_count) = self.build_where_clause(param_count);
        sql.push_str(&where_sql);
        params.extend(where_params);

        // Add RETURNING clause if specified
        if !self.returning.is_empty() {
            match self.backend {
                DatabaseBackend::Postgres | DatabaseBackend::SQLite => {
                    sql.push_str(" RETURNING ");
                    sql.push_str(&self.returning.join(", "));
                }
                DatabaseBackend::MySQL | DatabaseBackend::MariaDB => {
                    // MySQL doesn't support RETURNING clause
                }
            }
        }

        Ok((sql, params))
    }
}

// Note: All basic From implementations for SqlValue have been moved to
// crate::database::types::value to create a single point of truth for the type system.
// The PostgreSQL enum detection logic ("::" pattern) has been preserved there.

// Special implementations that depend on models module types remain here:

// NOTE: This generic impl conflicts with the reflexive Into impl and is commented out.
// Use manual conversions or implement specific From impls if needed.
// impl<T> From<crate::models::FieldUpdate<T>> for SqlValue
// where
//     T: Into<SqlValue>,
// {
//     fn from(field_update: crate::models::FieldUpdate<T>) -> Self {
//         match field_update {
//             crate::models::FieldUpdate::Set(value) => value.into(),
//             crate::models::FieldUpdate::SetNull => SqlValue::Null,
//         }
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_string_ref() {
        let s = String::from("test");
        let value: SqlValue = (&s).into();
        match value {
            SqlValue::String(str) => assert_eq!(str, "test"),
            _ => panic!("Expected SqlValue::String"),
        }
    }

    #[test]
    fn test_from_i32_ref() {
        let i = 42i32;
        let value: SqlValue = (&i).into();
        match value {
            SqlValue::Int(int) => assert_eq!(int, 42),
            _ => panic!("Expected SqlValue::Int"),
        }
    }

    #[test]
    fn test_from_bool_ref() {
        let b = true;
        let value: SqlValue = (&b).into();
        match value {
            SqlValue::Bool(bool) => assert_eq!(bool, true),
            _ => panic!("Expected SqlValue::Bool"),
        }
    }

    #[test]
    fn test_where_eq_accepts_string_ref() {
        let email = String::from("test@example.com");
        let query = QueryBuilder::new(DatabaseBackend::Postgres)
            .from("users")
            .where_eq("email", &email); // Should accept &String

        let result = query.build();
        assert!(result.is_ok());
    }
}

// Support for HashMap (for JSON serialization)
// NOTE: These HashMap impls conflict with the generic Into system and are commented out.
// Use SqlValue::Json(serde_json::to_value(map)?) instead if needed.
/*
impl From<std::collections::HashMap<String, SqlValue>> for SqlValue {
    fn from(map: std::collections::HashMap<String, SqlValue>) -> Self {
        // Convert HashMap to JSON string for storage
        let json_map: std::collections::HashMap<String, serde_json::Value> = map
            .into_iter()
            .map(|(k, v)| {
                let json_value = match v {
                    SqlValue::Null => serde_json::Value::Null,
                    SqlValue::Bool(b) => serde_json::Value::Bool(b),

                    // Integer types
                    SqlValue::TinyInt(i) => serde_json::Value::Number(i.into()),
                    SqlValue::SmallInt(i) => serde_json::Value::Number(i.into()),
                    SqlValue::Int(i) => serde_json::Value::Number(i.into()),
                    SqlValue::BigInt(i) => serde_json::Value::Number(i.into()),

                    // Unsigned integers
                    SqlValue::UnsignedTinyInt(i) => serde_json::Value::Number(i.into()),
                    SqlValue::UnsignedSmallInt(i) => serde_json::Value::Number(i.into()),
                    SqlValue::UnsignedInt(i) => serde_json::Value::Number(i.into()),
                    SqlValue::UnsignedBigInt(i) => serde_json::Value::Number(i.into()),

                    // Floating point
                    SqlValue::Float(f) => serde_json::json!(f),
                    SqlValue::Double(f) => serde_json::json!(f),
                    #[cfg(feature = "decimal")]
                    SqlValue::Decimal(d) => serde_json::Value::String(d.to_string()),
                    #[cfg(not(feature = "decimal"))]
                    SqlValue::Decimal(s) => serde_json::Value::String(s),

                    // Text types
                    SqlValue::String(s) => serde_json::Value::String(s),
                    SqlValue::Text(s) => serde_json::Value::String(s),

                    // Binary
                    SqlValue::Bytes(bytes) => serde_json::Value::String(base64::Engine::encode(
                        &base64::engine::general_purpose::STANDARD,
                        bytes,
                    )),

                    // Semantic types
                    SqlValue::Enum(val) => serde_json::Value::String(val),
                    SqlValue::Uuid(uuid) => serde_json::Value::String(uuid),
                    SqlValue::Json(j) => j,
                    SqlValue::Date(date) => serde_json::Value::String(date),
                    SqlValue::Time(time) => serde_json::Value::String(time),
                    SqlValue::DateTime(dt) => serde_json::Value::String(dt),
                    SqlValue::Timestamp(ts) => serde_json::Value::Number(ts.into()),

                    // Special types
                    SqlValue::Default => serde_json::Value::String("DEFAULT".to_string()),
                    SqlValue::Array(values) => {
                        serde_json::Value::Array(values.into_iter().map(|v| v.to_json()).collect())
                    }

                    // Network types - serialize as strings
                    SqlValue::Inet(ip) => serde_json::Value::String(ip.to_string()),
                    SqlValue::Cidr(ip, prefix) => serde_json::Value::String(format!("{}/{}", ip, prefix)),
                };
                (k, json_value)
            })
            .collect();

        SqlValue::String(serde_json::to_string(&json_map).unwrap_or_else(|_| "{}".to_string()))
    }
}

impl From<std::collections::HashMap<String, String>> for SqlValue {
    fn from(map: std::collections::HashMap<String, String>) -> Self {
        // Convert HashMap<String, String> to JSON string for storage
        SqlValue::String(serde_json::to_string(&map).unwrap_or_else(|_| "{}".to_string()))
    }
}
*/

// Note: FieldUpdate<T> implementation has been commented out earlier in this file due to conflicts
