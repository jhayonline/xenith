//! Native database schema and query primitives.
//!
//! This is deliberately a small, SQL-free schema API.  Xenith migration
//! functions will use these values through the native ABI; the API here keeps
//! the migration description typed and deterministic before a driver is
//! involved.

use std::fmt;
use std::collections::BTreeMap;

#[derive(Debug)]
pub enum DatabaseError {
    Sqlite(rusqlite::Error),
    InvalidValue(String),
}

impl fmt::Display for DatabaseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sqlite(error) => write!(formatter, "database error: {error}"),
            Self::InvalidValue(error) => write!(formatter, "invalid database value: {error}"),
        }
    }
}

impl std::error::Error for DatabaseError {}
impl From<rusqlite::Error> for DatabaseError {
    fn from(error: rusqlite::Error) -> Self { Self::Sqlite(error) }
}

pub type Row = BTreeMap<String, Value>;

pub struct SqliteDatabase {
    connection: rusqlite::Connection,
}

impl SqliteDatabase {
    pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self, DatabaseError> {
        Ok(Self { connection: rusqlite::Connection::open(path)? })
    }

    pub fn memory() -> Result<Self, DatabaseError> {
        Ok(Self { connection: rusqlite::Connection::open_in_memory()? })
    }

    pub fn apply(&self, schema: &SchemaPlan) -> Result<(), DatabaseError> {
        let transaction = self.connection.unchecked_transaction()?;
        for statement in schema.statements() {
            transaction.execute_batch(&statement)?;
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn execute(&self, query: &QueryPlan) -> Result<usize, DatabaseError> {
        let values = query.parameters.iter().map(to_sql_value).collect::<Result<Vec<_>, _>>()?;
        Ok(self.connection.execute(&query.sql, rusqlite::params_from_iter(values))?)
    }

    pub fn rows(&self, query: &QueryPlan) -> Result<Vec<Row>, DatabaseError> {
        let values = query.parameters.iter().map(to_sql_value).collect::<Result<Vec<_>, _>>()?;
        let mut statement = self.connection.prepare(&query.sql)?;
        let columns: Vec<String> = statement.column_names().iter().map(|name| (*name).to_owned()).collect();
        let rows = statement.query_map(rusqlite::params_from_iter(values), |row| {
            let mut result = Row::new();
            for (index, name) in columns.iter().enumerate() {
                result.insert(name.clone(), from_sql_value(row.get_ref(index)?));
            }
            Ok(result)
        })?;
        rows.collect::<Result<Vec<_>, rusqlite::Error>>().map_err(DatabaseError::from)
    }
}

fn to_sql_value(value: &Value) -> Result<rusqlite::types::Value, DatabaseError> {
    Ok(match value {
        Value::Null => rusqlite::types::Value::Null,
        Value::Integer(value) => rusqlite::types::Value::Integer(*value),
        Value::Text(value) => rusqlite::types::Value::Text(value.clone()),
        Value::Boolean(value) => rusqlite::types::Value::Integer(i64::from(*value)),
        Value::Bytes(value) => rusqlite::types::Value::Blob(value.clone()),
    })
}

fn from_sql_value(value: rusqlite::types::ValueRef<'_>) -> Value {
    match value {
        rusqlite::types::ValueRef::Null => Value::Null,
        rusqlite::types::ValueRef::Integer(value) => Value::Integer(value),
        rusqlite::types::ValueRef::Real(value) => Value::Text(value.to_string()),
        rusqlite::types::ValueRef::Text(value) => Value::Text(String::from_utf8_lossy(value).into_owned()),
        rusqlite::types::ValueRef::Blob(value) => Value::Bytes(value.to_vec()),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Null,
    Integer(i64),
    Text(String),
    Boolean(bool),
    Bytes(Vec<u8>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryPlan {
    pub sql: String,
    pub parameters: Vec<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Store {
    pub table: String,
}

impl Store {
    pub fn new(table: impl Into<String>) -> Self { Self { table: table.into() } }

    pub fn find(&self, id: i64) -> QueryPlan {
        QueryPlan {
            sql: format!("SELECT * FROM {} WHERE \"id\" = ? LIMIT 1", ident(&self.table)),
            parameters: vec![Value::Integer(id)],
        }
    }

    pub fn all(&self) -> QueryPlan {
        QueryPlan { sql: format!("SELECT * FROM {}", ident(&self.table)), parameters: vec![] }
    }

    pub fn where_eq(&self, field: impl AsRef<str>, value: Value) -> QueryPlan {
        QueryPlan {
            sql: format!("SELECT * FROM {} WHERE {} = ?", ident(&self.table), ident(field.as_ref())),
            parameters: vec![value],
        }
    }

    pub fn insert(&self, fields: impl IntoIterator<Item = (String, Value)>) -> QueryPlan {
        let fields: Vec<_> = fields.into_iter().collect();
        let names = fields.iter().map(|(name, _)| ident(name)).collect::<Vec<_>>().join(", ");
        let placeholders = std::iter::repeat_n("?", fields.len()).collect::<Vec<_>>().join(", ");
        QueryPlan {
            sql: format!("INSERT INTO {} ({names}) VALUES ({placeholders})", ident(&self.table)),
            parameters: fields.into_iter().map(|(_, value)| value).collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ColumnType {
    Integer,
    Text,
    Boolean,
    DateTime,
    Bytes,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForeignKey {
    pub table: String,
    pub column: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Column {
    pub name: String,
    pub ty: ColumnType,
    pub nullable: bool,
    pub is_unique: bool,
    pub is_primary: bool,
    pub is_generated: bool,
    pub default: Option<String>,
    pub references: Option<ForeignKey>,
}

impl Column {
    fn new(name: impl Into<String>, ty: ColumnType) -> Self {
        Self {
            name: name.into(),
            ty,
            nullable: false,
            is_unique: false,
            is_primary: false,
            is_generated: false,
            default: None,
            references: None,
        }
    }

    pub fn unique(&mut self) -> &mut Self { self.is_unique = true; self }
    pub fn nullable(&mut self) -> &mut Self { self.nullable = true; self }
    pub fn required(&mut self) -> &mut Self { self.nullable = false; self }
    pub fn default(&mut self, value: impl Into<String>) -> &mut Self {
        self.default = Some(value.into());
        self
    }
    pub fn references(&mut self, table: impl Into<String>, column: impl Into<String>) -> &mut Self {
        self.references = Some(ForeignKey { table: table.into(), column: column.into() });
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table {
    pub name: String,
    pub columns: Vec<Column>,
}

impl Table {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), columns: Vec::new() }
    }

    pub fn id(&mut self) -> &mut Column {
        let mut column = Column::new("id", ColumnType::Integer);
        column.is_primary = true;
        column.is_generated = true;
        self.columns.push(column);
        self.columns.last_mut().unwrap()
    }

    pub fn string(&mut self, name: impl Into<String>) -> &mut Column {
        self.push(name, ColumnType::Text)
    }

    pub fn integer(&mut self, name: impl Into<String>) -> &mut Column {
        self.push(name, ColumnType::Integer)
    }

    pub fn boolean(&mut self, name: impl Into<String>) -> &mut Column {
        self.push(name, ColumnType::Boolean)
    }

    pub fn datetime(&mut self, name: impl Into<String>) -> &mut Column {
        self.push(name, ColumnType::DateTime)
    }

    pub fn bytes(&mut self, name: impl Into<String>) -> &mut Column {
        self.push(name, ColumnType::Bytes)
    }

    pub fn foreign(&mut self, name: impl Into<String>) -> &mut Column {
        self.push(name, ColumnType::Integer)
    }

    pub fn timestamps(&mut self) {
        self.datetime("created_at");
        self.datetime("updated_at");
    }

    fn push(&mut self, name: impl Into<String>, ty: ColumnType) -> &mut Column {
        self.columns.push(Column::new(name, ty));
        self.columns.last_mut().unwrap()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SchemaPlan {
    pub creates: Vec<Table>,
    pub drops: Vec<String>,
}

impl SchemaPlan {
    pub fn new() -> Self { Self::default() }

    pub fn create(&mut self, name: impl Into<String>) -> &mut Table {
        self.creates.push(Table::new(name));
        self.creates.last_mut().unwrap()
    }

    pub fn drop(&mut self, name: impl Into<String>) {
        self.drops.push(name.into());
    }

    /// Render a portable migration plan for a driver.  This is intentionally
    /// the only place SQL text is produced; migration authors define fields
    /// through the typed builder instead of writing queries.
    pub fn statements(&self) -> Vec<String> {
        let mut statements = Vec::with_capacity(self.creates.len() + self.drops.len());
        for table in &self.creates {
            let columns = table.columns.iter().map(column_sql).collect::<Vec<_>>().join(", ");
            statements.push(format!("CREATE TABLE {} ({})", ident(&table.name), columns));
        }
        statements.extend(self.drops.iter().map(|name| format!("DROP TABLE {}", ident(name))));
        statements
    }
}

fn ident(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

fn column_sql(column: &Column) -> String {
    let ty = match column.ty {
        ColumnType::Integer => "INTEGER",
        ColumnType::Text => "TEXT",
        ColumnType::Boolean => "BOOLEAN",
        ColumnType::DateTime => "TIMESTAMP",
        ColumnType::Bytes => "BLOB",
    };
    let mut sql = format!("{} {}", ident(&column.name), ty);
    if column.is_primary { sql.push_str(" PRIMARY KEY"); }
    if column.is_generated { sql.push_str(" AUTOINCREMENT"); }
    if column.is_unique { sql.push_str(" UNIQUE"); }
    if !column.nullable { sql.push_str(" NOT NULL"); }
    if let Some(default) = &column.default { sql.push_str(&format!(" DEFAULT {default}")); }
    if let Some(reference) = &column.references {
        sql.push_str(&format!(" REFERENCES {}({})", ident(&reference.table), ident(&reference.column)));
    }
    sql
}

impl fmt::Display for SchemaPlan {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.statements().join(";\n"))
    }
}
