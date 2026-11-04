use xenith::native::orm::SchemaPlan;

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
