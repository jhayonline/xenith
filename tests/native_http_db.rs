use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::Command;
use std::thread;
use std::time::Duration;

fn compile(source: &str, directory: &tempfile::TempDir) -> std::path::PathBuf {
    let main = directory.path().join("main.xen");
    fs::write(&main, source).unwrap();
    let (program, _) = xenith::native::project::load_program(&main).unwrap();
    let typed = xenith::native::check::check(&program).unwrap();
    let object = directory.path().join("main.o");
    let executable = directory.path().join("main");
    xenith::native::codegen::emit_object(&typed, &object).unwrap();
    xenith::native::codegen::link_host(&object, &executable).unwrap();
    executable
}

#[test]
fn xenith_http_request_dispatch_exposes_pooled_database() {
    let directory = tempfile::tempdir().unwrap();
    let executable = compile(r#"
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
"#, &directory);
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(0));
}

#[test]
fn xenith_http_server_run_with_pool_exposes_database_to_tcp_handlers() {
    let reservation = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = reservation.local_addr().unwrap().port();
    drop(reservation);
    let directory = tempfile::tempdir().unwrap();
    let source = format!(r#"
grab {{ server_with_config, router, get_request, run_for_with_pool, request_db }} from "std::http"
grab {{ pool_new, pool_close }} from "std::db"
function show(request: i64) -> tuple<i32, string> {{
    if (request_db() != 0i64) {{ return (200, "db") }}
    return (500, "missing")
}}
function main() -> i32 {{
    let routes: i64 = router()
    get_request(routes, "/", show)
    let pool: i64 = pool_new(":memory:", 1i64, 1i64)
    let server: i64 = server_with_config("127.0.0.1:{port}", 65536i64, 1000i64, 1000i64, 16i64)
    let error: string = run_for_with_pool(server, routes, pool, 1i64)
    pool_close(pool)
    return len(error)
}}
"#);
    let executable = compile(&source, &directory);
    let mut child = Command::new(&executable).spawn().unwrap();
    thread::sleep(Duration::from_millis(100));
    let mut client = TcpStream::connect(("127.0.0.1", port)).unwrap();
    client.write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
    let mut response = Vec::new();
    client.read_to_end(&mut response).unwrap();
    assert!(response.ends_with(b"db"));
    assert_eq!(child.wait().unwrap().code(), Some(0));
}
