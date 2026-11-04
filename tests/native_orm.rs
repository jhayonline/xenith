use xenith::native::orm::{ColumnType, ModelSchema, SchemaPlan, SqliteDatabase, Store, Value};
use std::collections::BTreeMap;

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

#[test]
fn sqlite_driver_applies_each_named_migration_once() {
    let database = SqliteDatabase::memory().unwrap();
    let mut schema = SchemaPlan::new();
    schema.create("users").id();
    assert!(database.migrate("001_users", &schema).unwrap());
    assert!(!database.migrate("001_users", &schema).unwrap());
    let rows = database.rows(&Store::new("__xenith_migrations").all()).unwrap();
    assert_eq!(rows.len(), 1);
}

#[test]
fn model_schema_validates_fields_and_hydrates_relations() {
    let mut user = ModelSchema::new("User", "users");
    user.field("id", ColumnType::Integer)
        .field("name", ColumnType::Text)
        .has_many("posts", "posts", "user_id");
    let mut fields = BTreeMap::new();
    fields.insert("id".into(), Value::Integer(1));
    fields.insert("name".into(), Value::Text("Ada".into()));
    let mut post = BTreeMap::new();
    post.insert("user_id".into(), Value::Integer(1));
    let mut relations = BTreeMap::new();
    relations.insert("posts".into(), vec![post]);
    let hydrated = user.hydrate(fields, relations).unwrap();
    assert_eq!(hydrated.fields.get("name"), Some(&Value::Text("Ada".into())));
    assert_eq!(hydrated.relations["posts"].len(), 1);
}

#[test]
fn model_schema_rejects_wrong_field_types() {
    let mut user = ModelSchema::new("User", "users");
    user.field("id", ColumnType::Integer);
    let mut fields = BTreeMap::new();
    fields.insert("id".into(), Value::Text("not an id".into()));
    let error = user.hydrate(fields, BTreeMap::new()).unwrap_err();
    assert!(error.to_string().contains("wrong type"));
}
