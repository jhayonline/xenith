use xenith::native::orm::{SchemaPlan, SqliteDatabase, Store, Value};

#[test]
fn schema_builder_describes_a_users_table_without_raw_sql() {
    let mut schema = SchemaPlan::new();
    let users = schema.create("users");
    users.id();
    users.string("name");
    users.string("email").unique();
    users.timestamps();

    assert_eq!(schema.statements(), vec![
        "CREATE TABLE \"users\" (\"id\" INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL, \"name\" TEXT NOT NULL, \"email\" TEXT UNIQUE NOT NULL, \"created_at\" TIMESTAMP NOT NULL, \"updated_at\" TIMESTAMP NOT NULL)"
    ]);
}

#[test]
fn schema_builder_quotes_identifiers_and_supports_relations() {
    let mut schema = SchemaPlan::new();
    let posts = schema.create("posts");
    posts.id();
    posts.foreign("user_id").references("users", "id");
    schema.drop("old_posts");

    assert_eq!(schema.to_string(), "CREATE TABLE \"posts\" (\"id\" INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL, \"user_id\" INTEGER NOT NULL REFERENCES \"users\"(\"id\"));\nDROP TABLE \"old_posts\"");
}

#[test]
fn stores_build_parameterized_queries_without_raw_sql() {
    let users = Store::new("users");
    assert_eq!(users.find(42), xenith::native::orm::QueryPlan {
        sql: "SELECT * FROM \"users\" WHERE \"id\" = ? LIMIT 1".into(),
        parameters: vec![Value::Integer(42)],
    });
    assert_eq!(users.where_eq("email", Value::Text("ada@example.com".into())).sql,
        "SELECT * FROM \"users\" WHERE \"email\" = ?");
    assert_eq!(users.insert([
        ("name".into(), Value::Text("Ada".into())),
        ("active".into(), Value::Boolean(true)),
    ]).sql, "INSERT INTO \"users\" (\"name\", \"active\") VALUES (?, ?)");
}

#[test]
fn sqlite_driver_applies_schema_and_executes_parameterized_store_plans() {
    let mut schema = SchemaPlan::new();
    let users = schema.create("users");
    users.id();
    users.string("name");

    let database = SqliteDatabase::memory().unwrap();
    database.apply(&schema).unwrap();
    let store = Store::new("users");
    database.execute(&store.insert([
        ("name".into(), Value::Text("Ada".into())),
    ])).unwrap();

    let rows = database.rows(&store.find(1)).unwrap();
    assert_eq!(rows[0].get("name"), Some(&Value::Text("Ada".into())));
}
