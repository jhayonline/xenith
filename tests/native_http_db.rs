use std::fs;
use std::process::Command;

#[test]
fn xenith_http_request_dispatch_exposes_pooled_database() {
    let directory = tempfile::tempdir().unwrap();
    let source = r#"
grab { router, get_request, dispatch_request_pool, request_db, router_close } from "std::http"
grab { pool_new, pool_close } from "std::db"
function show(request: i64) -> tuple<i32, string> {
    if (request_db() != 0i64) { return (200, "db") }
    return (500, "missing")
}
function main() -> i32 {
    let pool: i64 = pool_new(":memory:", 1i64, 1i64)
    let routes: i64 = router()
    get_request(routes, "/", show)
    let response: tuple<i32, string> = dispatch_request_pool(routes, pool, "GET / HTTP/1.1")
    let second: tuple<i32, string> = dispatch_request_pool(routes, pool, "GET / HTTP/1.1")
    pool_close(pool)
    router_close(routes)
    if (response[0] == 200 && response[1] == "db" && second[0] == 200) { return 0 }
    return 1
}
"#;
    let main = directory.path().join("main.xen");
    fs::write(&main, source).unwrap();
    let (program, _) = xenith::native::project::load_program(&main).unwrap();
    let typed = xenith::native::check::check(&program).unwrap();
    let object = directory.path().join("main.o");
    let executable = directory.path().join("main");
    xenith::native::codegen::emit_object(&typed, &object).unwrap();
    xenith::native::codegen::link_host(&object, &executable).unwrap();
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(0));
}
