use super::{
    syntax::{self, Diagnostic, Program},
    NativeError,
};
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};
#[derive(Debug, Clone)]
pub struct Project {
    pub root: PathBuf,
    pub manifest_path: PathBuf,
    pub main_path: PathBuf,
    pub target_dir: PathBuf,
}

pub fn load_program(main_path: &Path) -> Result<(Program, Vec<PathBuf>), NativeError> {
    if !main_path.is_file() {
        return Err(NativeError::NotImplemented("missing src/main.xen".into()));
    }
    let mut visiting = HashSet::new();
    let mut loaded = HashSet::new();
    let mut paths = Vec::new();
    let program = load_module(main_path, true, &mut visiting, &mut loaded, &mut paths)?;
    Ok((program, paths))
}

fn load_module(
    path: &Path,
    root: bool,
    visiting: &mut HashSet<PathBuf>,
    loaded: &mut HashSet<PathBuf>,
    paths: &mut Vec<PathBuf>,
) -> Result<Program, NativeError> {
    let is_standard = path.to_string_lossy().starts_with("std::");
    let path = if is_standard {
        path.to_path_buf()
    } else {
        path.canonicalize().map_err(NativeError::io)?
    };
    if !visiting.insert(path.clone()) {
        return Err(NativeError::diagnostics(
            path,
            String::new(),
            vec![Diagnostic {
                message: "circular native module import".into(),
                span: None,
            }],
        ));
    }
    let source = if is_standard {
        let path_name = path.to_string_lossy();
        let name = path_name.trim_start_matches("std::");
        standard_native_source(name)
            .ok_or_else(|| NativeError::NotImplemented(format!("unknown standard module `{name}`")))?
            .to_string()
    } else {
        fs::read_to_string(&path).map_err(NativeError::io)?
    };
    let parsed = syntax::parse(&path, &source)
        .map_err(|e| NativeError::diagnostics(path.clone(), source.clone(), e))?;
    let mut structs = Vec::new();
    let mut enums = Vec::new();
    let mut functions = Vec::new();
    let mut imported_names = HashSet::new();
    for import in &parsed.imports {
        let imported_path = if import.path.starts_with("std::") {
            PathBuf::from(&import.path)
        } else {
            path.parent()
                .unwrap()
                .join(&import.path)
                .with_extension("xen")
        };
        let dependency = load_module(&imported_path, false, visiting, loaded, paths)?;
        if let Some(namespace) = &import.namespace {
            for function in dependency.functions.iter().filter(|item| item.exported) {
                let mut function = function.clone();
                function.name = format!("{namespace}__{}", function.name);
                functions.push(function);
            }
            for structure in dependency.structs.iter().filter(|item| item.exported) {
                let mut structure = structure.clone();
                structure.name = format!("{namespace}__{}", structure.name);
                structs.push(structure);
            }
            for enumeration in dependency.enums.iter().filter(|item| item.exported) {
                let mut enumeration = enumeration.clone();
                enumeration.name = format!("{namespace}__{}", enumeration.name);
                enums.push(enumeration);
            }
            continue;
        }
        for name in &import.names {
            if !imported_names.insert(name.local.clone()) {
                return Err(NativeError::diagnostics(
                    path.clone(),
                    source.clone(),
                    vec![Diagnostic {
                        message: format!("ambiguous imported name `{}`", name.local),
                        span: None,
                    }],
                ));
            }
            let function = dependency
                .functions
                .iter()
                .find(|f| f.name == name.original && f.exported)
                .cloned();
            let structure = dependency
                .structs
                .iter()
                .find(|s| s.name == name.original && s.exported)
                .cloned();
            let enumeration = dependency
                .enums
                .iter()
                .find(|e| e.name == name.original && e.exported)
                .cloned();
            match (function, structure, enumeration) {
                (Some(mut f), None, None) => {
                    f.name = name.local.clone();
                    functions.push(f)
                }
                (None, Some(mut s), None) => {
                    s.name = name.local.clone();
                    structs.push(s)
                }
                (None, None, Some(mut e)) => {
                    e.name = name.local.clone();
                    enums.push(e)
                }
                _ => {
                    return Err(NativeError::diagnostics(
                        path.clone(),
                        source.clone(),
                        vec![Diagnostic {
                            message: format!(
                                "module `{}` does not export `{}`",
                                import.path, name.original
                            ),
                            span: None,
                        }],
                    ))
                }
            }
        }
    }
    if root {
        structs.extend(parsed.structs);
        enums.extend(parsed.enums);
        functions.extend(parsed.functions);
    } else {
        structs.extend(parsed.structs.into_iter().filter(|item| item.exported));
        enums.extend(parsed.enums.into_iter().filter(|item| item.exported));
        functions.extend(parsed.functions.into_iter().filter(|item| item.exported));
    }
    visiting.remove(&path);
    if loaded.insert(path.clone()) && !is_standard {
        paths.push(path);
    }
    Ok(Program {
        file: "project".into(),
        source: String::new(),
        imports: vec![],
        structs,
        enums,
        functions,
    })
}

fn standard_native_source(name: &str) -> Option<&'static str> {
    match name {
        "env" => Some(
            "export function get(name: string) -> tuple<string, bool> { return env_get(name) }\n\
             export function set(name: string, value: string) -> null { return env_set(name, value) }\n\
             export function unset(name: string) -> null { return env_unset(name) }\n\
             export function all() -> map<string, string> { return env_vars() }\n\
             export function args() -> list<string> { return env_args() }\n\
             export function cwd() -> tuple<string, string> { return env_cwd() }\n\
             export function exit(code: i32) -> null { return env_exit(code) }",
        ),
        "config" => Some(
            "export function load_config(path: string) -> i64 { return config_load(path) }\n\
             export function config_string(config: i64, key: string, fallback: string) -> string { return config_get_string(config, key, fallback) }\n\
             export function config_i32(config: i64, key: string, fallback: i32) -> i32 { return config_get_i32(config, key, fallback) }\n\
             export function config_i64(config: i64, key: string, fallback: i64) -> i64 { return config_get_i64(config, key, fallback) }\n\
             export function config_bool(config: i64, key: string, fallback: bool) -> bool { return config_get_bool(config, key, fallback) }\n\
             export function required_config(config: i64, key: string) -> string { return config_required(config, key) }\n\
             export function close_config(config: i64) -> null { return config_close(config) }",
        ),
        "fs" => Some(
            "export function read(path: string) -> tuple<string, string> { return fs_read(path) }\n\
             export function write(path: string, contents: string) -> string { return fs_write(path, contents) }\n\
             export function append(path: string, contents: string) -> string { return fs_append(path, contents) }\n\
             export function read_bytes(path: string) -> tuple<bytes, string> { return fs_read_bytes(path) }\n\
             export function write_bytes(path: string, contents: bytes) -> string { return fs_write_bytes(path, contents) }\n\
             export function append_bytes(path: string, contents: bytes) -> string { return fs_append_bytes(path, contents) }\n\
             export function remove(path: string) -> string { return fs_remove(path) }\n\
             export function exists(path: string) -> bool { return fs_exists(path) }\n\
             export function is_file(path: string) -> bool { return fs_is_file(path) }\n\
             export function is_dir(path: string) -> bool { return fs_is_dir(path) }\n\
             export function size(path: string) -> tuple<i64, string> { return fs_size(path) }\n\
             export function list_dir(path: string) -> tuple<list<string>, string> { return fs_list(path) }\n\
             export function create_dir(path: string) -> string { return fs_create_dir(path) }\n\
             export function remove_dir(path: string) -> string { return fs_remove_dir(path) }",
        ),
        "db" => Some(
            "export function open(path: string) -> i64 { return db_open(path) }\n\
             export function pool_new(path: string, minimum: i64, maximum: i64) -> i64 { return db_pool_new(path, minimum, maximum) }\n\
             export function pool_acquire(pool: i64) -> i64 { return db_pool_acquire(pool) }\n\
             export function pool_release(pool: i64, handle: i64) -> null { return db_pool_release(pool, handle) }\n\
             export function pool_close(pool: i64) -> null { return db_pool_close(pool) }\n\
             export function schema(table: string) -> i64 { return db_schema(table) }\n\
             export function field(schema: i64, name: string, kind: string) -> string { return db_field(schema, name, kind) }\n\
             export function nullable(schema: i64, name: string) -> string { return db_nullable(schema, name) }\n\
             export function default_value(schema: i64, name: string, value: string) -> string { return db_default(schema, name, value) }\n\
             export function unique(schema: i64, name: string) -> string { return db_unique(schema, name) }\n\
             export function index(schema: i64, name: string) -> string { return db_index(schema, name) }\n\
             export function migrate(handle: i64, name: string, schema: i64) -> string { return db_migrate(handle, name, schema) }\n\
             export function schema_close(schema: i64) -> null { return db_schema_close(schema) }\n\
             export function execute(handle: i64, sql: string) -> string { return db_exec(handle, sql) }\n\
             export function store(handle: i64, table: string) -> i64 { return db_store(handle, table) }\n\
             export function find(store: i64, id: i64) -> tuple<string, string> { return db_find(store, id) }\n\
             export function find_map(store: i64, id: i64) -> map<string, string> { return db_find_map(store, id) }\n\
             export function find_model(store: i64, id: i64, model: map<string, string>) -> map<string, string> { return db_find_model(store, id, model) }\n\
             export function related(store: i64, foreign_key: string, id: i64) -> list<map<string, string>> { return db_related(store, foreign_key, id) }\n\
             export function as_i64(row: map<string, string>, key: string) -> i64 { return db_map_i64(row, key) }\n\
             export function as_i32(row: map<string, string>, key: string) -> i32 { return db_map_i32(row, key) }\n\
             export function as_bool(row: map<string, string>, key: string) -> bool { return db_map_bool(row, key) }\n\
             export function as_string(row: map<string, string>, key: string) -> string { return db_map_string(row, key) }\n\
             export function set_i32(row: map<string, string>, key: string, value: i32) -> null { return db_set_i32(row, key, value) }\n\
             export function set_i64(row: map<string, string>, key: string, value: i64) -> null { return db_set_i64(row, key, value) }\n\
             export function set_bool(row: map<string, string>, key: string, value: bool) -> null { return db_set_bool(row, key, value) }\n\
             export function set_string(row: map<string, string>, key: string, value: string) -> null { return db_set_string(row, key, value) }\n\
             export function validate(schema: map<string, string>, row: map<string, string>) -> string { return db_validate(schema, row) }\n\
             export function insert_validated(store: i64, schema: map<string, string>, row: map<string, string>) -> i64 { return db_insert_validated(store, schema, row) }\n\
             export function insert_model(store: i64, model: map<string, string>, row: map<string, string>) -> i64 { return db_insert_model(store, model, row) }\n\
             export function update_validated(store: i64, id: i64, schema: map<string, string>, row: map<string, string>) -> string { return db_update_validated(store, id, schema, row) }\n\
             export function update_model(store: i64, id: i64, model: map<string, string>, row: map<string, string>) -> string { return db_update_model(store, id, model, row) }\n\
             export function insert(store: i64, row: map<string, string>) -> i64 { return db_insert(store, row) }\n\
             export function delete(store: i64, id: i64) -> string { return db_delete(store, id) }\n\
             export function update(store: i64, id: i64, row: map<string, string>) -> string { return db_update(store, id, row) }\n\
             export function all(store: i64) -> list<map<string, string>> { return db_all(store) }\n\
             export function filter(store: i64, column: string, value: string) -> list<map<string, string>> { return db_filter(store, column, value) }\n\
             export function order(store: i64, column: string, descending: bool) -> list<map<string, string>> { return db_order(store, column, descending) }\n\
             export function page(store: i64, limit: i64, offset: i64) -> list<map<string, string>> { return db_page(store, limit, offset) }\n\
             export function query(store: i64, filter_column: string, filter_value: string, order_column: string, descending: bool, limit: i64, offset: i64) -> list<map<string, string>> { return db_query(store, filter_column, filter_value, order_column, descending, limit, offset) }\n\
             export function begin(handle: i64) -> string { return db_begin(handle) }\n\
             export function commit(handle: i64) -> string { return db_commit(handle) }\n\
             export function rollback(handle: i64) -> string { return db_rollback(handle) }\n\
             export function close(handle: i64) -> null { return db_close(handle) }",
        ),
        "http" => Some(
            "export function bind(address: string) -> i64 { return http_bind(address) }\n\
             export function accept(listener: i64) -> i64 { return http_accept(listener) }\n\
             export function read(connection: i64) -> tuple<string, string> { return http_read(connection) }\n\
             export function write(connection: i64, status: i32, body: string) -> string { return http_write(connection, status, body) }\n\
             export function close(connection: i64) -> null { return http_close(connection) }\n\
             export function router() -> i64 { return http_router_new() }\n\
             export function get(router: i64, pattern: string, handler: fn(string, string, string) -> tuple<i32, string>) -> string { return http_router_get(router, pattern, handler) }\n\
             export function post(router: i64, pattern: string, handler: fn(string, string, string) -> tuple<i32, string>) -> string { return http_router_post(router, pattern, handler) }\n\
             export function use(router: i64, middleware: fn(string, string, string) -> tuple<i32, string>) -> string { return http_router_use(router, middleware) }\n\
             export function get_request(router: i64, pattern: string, handler: fn(i64) -> tuple<i32, string>) -> string { return http_router_get_request(router, pattern, handler) }\n\
             export function post_request(router: i64, pattern: string, handler: fn(i64) -> tuple<i32, string>) -> string { return http_router_post_request(router, pattern, handler) }\n\
             export function get_response(router: i64, pattern: string, handler: fn(i64) -> i64) -> string { return http_router_get_response(router, pattern, handler) }\n\
             export function post_response(router: i64, pattern: string, handler: fn(i64) -> i64) -> string { return http_router_post_response(router, pattern, handler) }\n\
             export function use_request(router: i64, middleware: fn(i64) -> i64) -> string { return http_router_use_request(router, middleware) }\n\
             export function use_session(router: i64, database: i64, secret: string) -> string { return http_router_use_session(router, database, secret) }\n\
             export function use_role(router: i64, database: i64, secret: string, role: string) -> string { return http_router_use_role(router, database, secret, role) }\n\
             export function use_csrf(router: i64, database: i64, secret: string) -> string { return http_router_use_csrf(router, database, secret) }\n\
             export function dispatch(router: i64, method: string, path: string, body: string) -> tuple<i32, string> { return http_router_dispatch(router, method, path, body) }\n\
             export function dispatch_request(router: i64, request: string) -> tuple<i32, string> { return http_router_dispatch_request(router, request) }\n\
             export function dispatch_request_pool(router: i64, pool: i64, request: string) -> tuple<i32, string> { return http_router_dispatch_request_pool(router, pool, request) }\n\
             export function dispatch_response(router: i64, request: string) -> i64 { return http_router_dispatch_response(router, request) }\n\
             export function request_db() -> i64 { return http_request_db() }\n\
             export function request_begin() -> string { return http_request_begin() }\n\
             export function request_commit() -> string { return http_request_commit() }\n\
             export function request_rollback() -> string { return http_request_rollback() }\n\
             export function request_method(request: i64) -> string { return http_request_method(request) }\n\
             export function request_path(request: i64) -> string { return http_request_path(request) }\n\
             export function request_body(request: i64) -> string { return http_request_body(request) }\n\
             export function request_header(request: i64, name: string) -> string { return http_request_header(request, name) }\n\
             export function request_cookie(request: i64, name: string) -> string { return http_request_cookie(request, name) }\n\
             export function request_query(request: i64, key: string) -> string { return http_request_query(request, key) }\n\
             export function request_param(request: i64, pattern: string, name: string) -> string { return http_request_param(request, pattern, name) }\n\
             export function require_header(request: i64, name: string) -> tuple<string, string> { return http_require_header(request, name) }\n\
             export function require_query(request: i64, key: string) -> tuple<string, string> { return http_require_query(request, key) }\n\
             export function require_param(request: i64, pattern: string, name: string) -> tuple<string, string> { return http_require_param(request, pattern, name) }\n\
             export function json_validate_schema(body: string, schema: map<string, string>) -> string { return http_json_validate_schema(body, schema) }\n\
             export function validate(input: map<string, string>, schema: map<string, i64>) -> map<string, string> { return http_validate(input, schema) }\n\
             export function validate_json(body: string, schema: map<string, i64>) -> map<string, string> { return http_validate_json(body, schema) }\n\
             export function validate_query(request: i64, schema: map<string, i64>) -> map<string, string> { return http_validate_query(request, schema) }\n\
             export function validate_params(request: i64, pattern: string, schema: map<string, i64>) -> map<string, string> { return http_validate_params(request, pattern, schema) }\n\
             export function validate_form(request: i64, schema: map<string, i64>) -> map<string, string> { return http_validate_form(request, schema) }\n\
             export function validate_status(errors: map<string, string>) -> i32 { return http_validate_status(errors) }\n\
             export function required() -> i64 { return 1i64 }\n\
             export function optional() -> i64 { return 2i64 }\n\
             export function string() -> i64 { return 4i64 }\n\
             export function i64() -> i64 { return 8i64 }\n\
             export function email() -> i64 { return 16i64 }\n\
             export function min(value: i64) -> i64 { return value }\n\
             export function max(value: i64) -> i64 { return value }\n\
             export function rules(first: i64) -> i64 { return first }\n\
             export function response_new(status: i32, body: string) -> i64 { return http_response_new(status, body) }\n\
             export function response_header(response: i64, name: string, value: string) -> string { return http_response_header(response, name, value) }\n\
             export function response_cookie(response: i64, name: string, value: string, max_age: i64, secure: bool, http_only: bool) -> string { return http_response_cookie(response, name, value, max_age, secure, http_only) }\n\
             export function response_status(response: i64) -> i32 { return http_response_status(response) }\n\
             export function response_body(response: i64) -> string { return http_response_body(response) }\n\
             export function response_header_value(response: i64, name: string) -> string { return http_response_header_value(response, name) }\n\
             export function response_close(response: i64) -> null { return http_response_close(response) }\n\
             export function router_close(router: i64) -> null { return http_router_close(router) }\n\
             export function serve(router: i64, address: string) -> string { return http_serve(router, address) }\n\
             export function query(path: string, key: string) -> string { return http_query(path, key) }\n\
             export function json(status: i32, body: string) -> tuple<i32, string> { return http_json(status, body) }\n\
             export function json_body(request: string) -> tuple<string, string> { return http_json_body(request) }\n\
             export function json_validate(body: string) -> string { return http_json_validate(body) }\n\
             export function json_escape(value: string) -> string { return http_json_escape(value) }\n\
             export function json_object(body: string) -> tuple<map<string, string>, string> { return http_json_object(body) }\n\
             export function json_string(value: string) -> tuple<string, string> { return http_json_string(value) }\n\
             export function json_i64(value: string) -> tuple<i64, string> { return http_json_i64(value) }\n\
             export function json_bool(value: string) -> tuple<bool, string> { return http_json_bool(value) }\n\
             export function header(request: string, name: string) -> string { return http_header(request, name) }\n\
             export function param(path: string, pattern: string, name: string) -> string { return http_param(path, pattern, name) }\n\
             export function server(address: string) -> i64 { return http_server_bind(address) }\n\
             export function server_with_config(address: string, max_request_size: i64, read_timeout_ms: i64, write_timeout_ms: i64, backlog: i64) -> i64 { return http_server_bind_config(address, max_request_size, read_timeout_ms, write_timeout_ms, backlog) }\n\
             export function shutdown(server: i64) -> null { return http_server_shutdown(server) }\n\
             export function run(server: i64, router: i64) -> string { return http_server_run(server, router) }\n\
             export function run_with_pool(server: i64, router: i64, pool: i64) -> string { return http_server_run_pool(server, router, pool) }\n\
             export function run_for(server: i64, router: i64, max_requests: i64) -> string { return http_server_run_for(server, router, max_requests) }\n\
             export function run_for_with_pool(server: i64, router: i64, pool: i64, max_requests: i64) -> string { return http_server_run_for_pool(server, router, pool, max_requests) }\n\
             export function run_workers(server: i64, router: i64, workers: i64) -> string { return http_server_run_workers(server, router, workers) }\n\
             export function run_workers_with_pool(server: i64, router: i64, pool: i64, workers: i64) -> string { return http_server_run_workers_pool(server, router, pool, workers) }",
        ),
        "auth" => Some(
            "export function hash_password(password: string) -> string { return auth_hash_password(password) }\n\
             export function verify_password(password: string, encoded: string) -> bool { return auth_verify_password(password, encoded) }\n\
             export function session_open(database: i64) -> string { return auth_session_open(database) }\n\
             export function session_create(database: i64, user_id: i64, ttl_seconds: i64) -> string { return auth_session_create(database, user_id, ttl_seconds) }\n\
             export function session_user(database: i64, token: string) -> i64 { return auth_session_user(database, token) }\n\
             export function session_revoke(database: i64, token: string) -> string { return auth_session_revoke(database, token) }\n\
             export function session_cookie_create(database: i64, user_id: i64, ttl_seconds: i64, secret: string) -> string { return auth_session_cookie_create(database, user_id, ttl_seconds, secret) }\n\
             export function session_cookie_user(database: i64, cookie: string, secret: string) -> i64 { return auth_session_cookie_user(database, cookie, secret) }\n\
             export function session_request_user(database: i64, request: i64, secret: string) -> i64 { return auth_session_request_user(database, request, secret) }\n\
             export function role_grant(database: i64, user_id: i64, role: string) -> string { return auth_role_grant(database, user_id, role) }\n\
             export function role_revoke(database: i64, user_id: i64, role: string) -> string { return auth_role_revoke(database, user_id, role) }\n\
             export function user_has_role(database: i64, user_id: i64, role: string) -> bool { return auth_user_has_role(database, user_id, role) }\n\
             export function csrf_token(database: i64, user_id: i64, ttl_seconds: i64) -> string { return auth_csrf_token(database, user_id, ttl_seconds) }\n\
             export function csrf_verify(database: i64, user_id: i64, token: string) -> bool { return auth_csrf_verify(database, user_id, token) }",
        ),
        _ => None,
    }
}
impl Project {
    pub fn discover(start: &Path) -> Result<Self, NativeError> {
        let mut cursor = if start.is_file() {
            start.parent().unwrap_or(start)
        } else {
            start
        };
        loop {
            if cursor.join("xenith.toml").is_file() {
                return Ok(Self::at(cursor));
            }
            cursor = cursor.parent().ok_or(NativeError::ProjectNotFound)?;
        }
    }
    fn at(root: &Path) -> Self {
        Self {
            root: root.into(),
            manifest_path: root.join("xenith.toml"),
            main_path: root.join("src/main.xen"),
            target_dir: root.join("target"),
        }
    }
}
pub fn create_project(path: &Path, name: &str) -> Result<Project, NativeError> {
    if !valid_name(name) {
        return Err(NativeError::InvalidPackageName(name.into()));
    }
    let root = path.join(name);
    if root.exists() {
        return Err(NativeError::AlreadyExists(root.display().to_string()));
    }
    fs::create_dir_all(root.join("src")).map_err(NativeError::io)?;
    fs::write(
        root.join("xenith.toml"),
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2027\"\n"),
    )
    .map_err(NativeError::io)?;
    fs::write(
        root.join("src/main.xen"),
        "fn main() -> i32 {\n    return 0\n}\n",
    )
    .map_err(NativeError::io)?;
    Ok(Project::at(&root))
}
fn valid_name(name: &str) -> bool {
    let mut chars = name.chars();
    matches!(chars.next(), Some(first) if first.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}
