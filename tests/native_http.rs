use std::{fs, io::{Read, Write}, net::{TcpListener, TcpStream}, process::Command, thread, time::Duration};
use xenith::native::http::{parse_request, HttpError, Response, Server};

#[test]
fn http_parser_decodes_request_line_headers_and_body() {
    let request = parse_request(b"POST /users HTTP/1.1\r\nHost: example.test\r\nContent-Length: 7\r\nX-Trace: abc\r\n\r\nAda=one").unwrap();
    assert_eq!(request.method, "POST");
    assert_eq!(request.target, "/users");
    assert_eq!(request.headers["host"], "example.test");
    assert_eq!(request.body, b"Ada=one");
}

#[test]
fn http_parser_rejects_incomplete_or_malformed_requests() {
    assert!(matches!(parse_request(b"GET / HTTP/1.1\r\n"), Err(HttpError::Incomplete)));
    assert!(matches!(parse_request(b"NOPE\r\n\r\n"), Err(HttpError::Malformed(_))));
}

#[test]
fn http_response_serializes_status_headers_and_body() {
    let response = Response::new(201, b"created".to_vec()).header("Content-Type", "text/plain");
    assert_eq!(response.to_bytes(), b"HTTP/1.1 201 Created\r\nContent-Type: text/plain\r\nContent-Length: 7\r\nConnection: close\r\n\r\ncreated");
}

#[test]
fn http_server_accepts_a_request_and_writes_the_handler_response() {
    let server = Server::bind("127.0.0.1:0").unwrap();
    let address = server.local_addr();
    let worker = thread::spawn(move || {
        server.accept_once(|request| Response::new(200, format!("{} {}", request.method, request.target).into_bytes()))
    });
    let mut client = TcpStream::connect(address).unwrap();
    client.write_all(b"GET /health HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
    let mut response = Vec::new();
    client.read_to_end(&mut response).unwrap();
    worker.join().unwrap().unwrap();
    assert!(response.ends_with(b"GET /health"));
}

#[test]
fn xenith_http_module_serves_a_real_tcp_request() {
    let reservation = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = reservation.local_addr().unwrap().port();
    drop(reservation);
    let directory = tempfile::tempdir().unwrap();
    let source = format!(r#"
grab {{ bind, accept, read, write, close }} from "std::http"
function main() -> i32 {{
    let listener: i64 = bind("127.0.0.1:{port}")
    let connection: i64 = accept(listener)
    let request: tuple<string, string> = read(connection)
    let error: string = write(connection, 200, "ok")
    close(connection)
    close(listener)
    return len(error)
}}
"#);
    let main = directory.path().join("main.xen");
    fs::write(&main, source).unwrap();
    let (program, _) = xenith::native::project::load_program(&main).unwrap();
    let typed = xenith::native::check::check(&program).unwrap();
    let object = directory.path().join("main.o");
    let executable = directory.path().join("main");
    xenith::native::codegen::emit_object(&typed, &object).unwrap();
    xenith::native::codegen::link_host(&object, &executable).unwrap();
    let mut child = Command::new(&executable).spawn().unwrap();
    thread::sleep(Duration::from_millis(100));
    let mut client = TcpStream::connect(("127.0.0.1", port)).unwrap();
    client.write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
    let mut response = Vec::new();
    client.read_to_end(&mut response).unwrap();
    assert!(response.ends_with(b"ok"));
    assert_eq!(child.wait().unwrap().code(), Some(0));
}

#[test]
fn xenith_http_router_dispatches_a_registered_route() {
    let directory = tempfile::tempdir().unwrap();
    let source = r#"
grab { router, get, dispatch, router_close } from "std::http"
function hello(method: string, path: string, body: string) -> tuple<i32, string> {
    return (200, "hello")
}
function main() -> i32 {
    let routes: i64 = router()
    let registration: string = get(routes, "/hello", hello)
    let response: tuple<i32, string> = dispatch(routes, "GET", "/hello", "")
    router_close(routes)
    return response[0] + len(response[1]) + len(registration)
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
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(205));
}

#[test]
fn xenith_http_router_matches_parameters_and_rejects_bad_routes() {
    let directory = tempfile::tempdir().unwrap();
    let source = r#"
grab { router, get, dispatch, router_close } from "std::http"
function show(method: string, path: string, body: string) -> tuple<i32, string> {
    return (201, path)
}
function main() -> i32 {
    let routes: i64 = router()
    let registration: string = get(routes, "/users/:id", show)
    let malformed: string = get(routes, "/users/:", show)
    let response: tuple<i32, string> = dispatch(routes, "GET", "/users/42", "")
    let missing: tuple<i32, string> = dispatch(routes, "POST", "/users/42", "")
    router_close(routes)
    if (len(registration) == 0 && len(malformed) > 0 && response[0] == 201 && missing[0] == 404) {
        return len(response[1])
    }
    return 0
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
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(9));
}

#[test]
fn xenith_http_router_passes_post_body_to_handler() {
    let directory = tempfile::tempdir().unwrap();
    let source = r#"
grab { router, post, dispatch, router_close } from "std::http"
function create(method: string, path: string, body: string) -> tuple<i32, string> {
    return (201, body)
}

function main() -> i32 {
    let routes: i64 = router()
    post(routes, "/users", create)
    let response: tuple<i32, string> = dispatch(routes, "POST", "/users", "payload")
    router_close(routes)
    return response[0] + len(response[1])
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
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(208));
}

#[test]
fn xenith_http_serve_routes_a_real_request() {
    let reservation = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = reservation.local_addr().unwrap().port();
    drop(reservation);
    let directory = tempfile::tempdir().unwrap();
    let source = format!(r#"
grab {{ router, get, serve }} from "std::http"
function health(method: string, path: string, body: string) -> tuple<i32, string> {{
    return (200, "served")
}}
function main() -> i32 {{
    let routes: i64 = router()
    get(routes, "/health", health)
    let error: string = serve(routes, "127.0.0.1:{port}")
    return len(error)
}}
"#);
    let main = directory.path().join("main.xen");
    fs::write(&main, source).unwrap();
    let (program, _) = xenith::native::project::load_program(&main).unwrap();
    let typed = xenith::native::check::check(&program).unwrap();
    let object = directory.path().join("main.o");
    let executable = directory.path().join("main");
    xenith::native::codegen::emit_object(&typed, &object).unwrap();
    xenith::native::codegen::link_host(&object, &executable).unwrap();
    let mut child = Command::new(&executable).spawn().unwrap();
    thread::sleep(Duration::from_millis(100));
    let mut client = TcpStream::connect(("127.0.0.1", port)).unwrap();
    client.write_all(b"GET /health HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
    let mut response = Vec::new();
    client.read_to_end(&mut response).unwrap();
    assert!(response.ends_with(b"served"));
    child.kill().unwrap();
    let _ = child.wait();
}

#[test]
fn xenith_http_query_and_json_helpers_are_typed() {
    let directory = tempfile::tempdir().unwrap();
    let source = r#"
grab { query, json } from "std::http"
function main() -> i32 {
    let value: string = query("/search?q=hello&limit=2", "q")
    let response: tuple<i32, string> = json(200, "ok")
    return response[0] + len(value)
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
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(205));
}

#[test]
fn xenith_http_headers_and_parameters_are_typed() {
    let directory = tempfile::tempdir().unwrap();
    let source = r#"
grab { header, param } from "std::http"
function main() -> i32 {
    let request: string = "Host: abc"
    let host: string = header(request, "Host")
    let id: string = param("/users/42", "/users/:id", "id")
    return len(host) + len(id)
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
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(5));
}

#[test]
fn xenith_http_server_shutdown_is_safe_and_recoverable() {
    let directory = tempfile::tempdir().unwrap();
    let source = r#"
grab { server, shutdown, run } from "std::http"
function main() -> i32 {
    let handle: i64 = server("not-an-address")
    shutdown(handle)
    let error: string = run(handle, 0i64)
    if (len(error) > 0) { return 1 }
    return 0
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
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(1));
}

#[test]
fn xenith_http_json_body_validates_and_escapes_json() {
    let directory = tempfile::tempdir().unwrap();
    let source = r#"
grab { json_body, json_validate, json_escape } from "std::http"
function main() -> i32 {
    let request: string = "POST / HTTP/1.1\r\nContent-Length: 4\r\n\r\ntrue"
    let parsed: tuple<string, string> = json_body(request)
    let valid: string = json_validate("[1, true, null]")
    let invalid: string = json_validate("oops")
    let encoded: string = json_escape("hello")
    return len(parsed[0]) + len(parsed[1]) + len(valid) + len(invalid) + len(encoded)
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
    // `true` is four bytes, the invalid-json diagnostic and escaped string are non-empty.
    assert!(Command::new(executable).status().unwrap().code().unwrap() > 8);
}
