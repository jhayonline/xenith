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

#[test]
fn xenith_http_json_object_returns_a_typed_map_or_error() {
    let directory = tempfile::tempdir().unwrap();
    let source = r#"
grab { json_object } from "std::http"
function main() -> i32 {
    let decoded: tuple<map<string, string>, string> = json_object("true")
    return len(decoded[1])
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
    assert!(Command::new(executable).status().unwrap().code().unwrap() > 0);
}

#[test]
fn xenith_http_json_object_decodes_post_fields() {
    let reservation = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = reservation.local_addr().unwrap().port();
    drop(reservation);
    let directory = tempfile::tempdir().unwrap();
    let source = format!(r#"
grab {{ router, post, serve, json_object }} from "std::http"
function create(method: string, path: string, body: string) -> tuple<i32, string> {{
    let decoded: tuple<map<string, string>, string> = json_object(body)
    if (len(decoded[1]) > 0) {{ return (400, decoded[1]) }}
    return (200, decoded[0]["name"])
}}
function main() -> i32 {{
    let routes: i64 = router()
    post(routes, "/users", create)
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
    client.write_all(b"POST /users HTTP/1.1\r\nContent-Length: 22\r\n\r\n{\"name\":\"Ada\",\"age\":42}").unwrap();
    let mut response = Vec::new();
    client.read_to_end(&mut response).unwrap();
    assert!(response.ends_with(b"\"Ada\""));
    child.kill().unwrap();
    let _ = child.wait();
}

#[test]
fn xenith_http_json_values_convert_to_typed_scalars() {
    let directory = tempfile::tempdir().unwrap();
    let source = r#"
grab { json_string, json_i64, json_bool, json_escape } from "std::http"
function main() -> i32 {
    let encoded: string = json_escape("Ada")
    let text: tuple<string, string> = json_string(encoded)
    let number: tuple<i64, string> = json_i64("42")
    let flag: tuple<bool, string> = json_bool("true")
    let bad: tuple<i64, string> = json_i64("oops")
    if (text[0] == "Ada" && number[0] == 42i64 && flag[0] && len(text[1]) == 0 && len(number[1]) == 0 && len(bad[1]) > 0) {
        return 1
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
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(1));
}

#[test]
fn xenith_http_server_configuration_rejects_invalid_settings_recoverably() {
    let directory = tempfile::tempdir().unwrap();
    let source = r#"
grab { server_with_config, shutdown, run, router } from "std::http"
function main() -> i32 {
    let handle: i64 = server_with_config("not-an-address", 1024i64, 1000i64, 1000i64, 16i64)
    shutdown(handle)
    let error: string = run(handle, router())
    return len(error)
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
    assert!(Command::new(executable).status().unwrap().code().unwrap() > 0);
}

#[test]
fn xenith_http_router_middleware_can_short_circuit_or_continue() {
    let directory = tempfile::tempdir().unwrap();
    let source = r#"
grab { router, get, use, dispatch, router_close } from "std::http"
function auth(method: string, path: string, body: string) -> tuple<i32, string> {
    if (path == "/private") { return (401, "unauthorized") }
    return (0, "")
}
function hello(method: string, path: string, body: string) -> tuple<i32, string> {
    return (200, "ok")
}
function main() -> i32 {
    let routes: i64 = router()
    use(routes, auth)
    get(routes, "/private", hello)
    get(routes, "/public", hello)
    let denied: tuple<i32, string> = dispatch(routes, "GET", "/private", "")
    let allowed: tuple<i32, string> = dispatch(routes, "GET", "/public", "")
    router_close(routes)
    if (denied[0] == 401 && allowed[0] == 200 && allowed[1] == "ok") { return 1 }
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
fn xenith_http_run_for_processes_a_bounded_number_of_requests() {
    let reservation = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = reservation.local_addr().unwrap().port();
    drop(reservation);
    let directory = tempfile::tempdir().unwrap();
    let source = format!(r#"
grab {{ server_with_config, router, get, run_for }} from "std::http"
function health(method: string, path: string, body: string) -> tuple<i32, string> {{
    return (200, "ok")
}}
function main() -> i32 {{
    let routes: i64 = router()
    get(routes, "/health", health)
    let server: i64 = server_with_config("127.0.0.1:{port}", 65536i64, 1000i64, 1000i64, 16i64)
    let error: string = run_for(server, routes, 2i64)
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
    for _ in 0..2 {
        let mut client = TcpStream::connect(("127.0.0.1", port)).unwrap();
        client.write_all(b"GET /health HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
        let mut response = Vec::new();
        client.read_to_end(&mut response).unwrap();
        assert!(response.ends_with(b"ok"));
    }
    assert_eq!(child.wait().unwrap().code(), Some(0));
}

#[test]
fn xenith_http_typed_response_route_serves_headers_over_tcp() {
    let reservation = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = reservation.local_addr().unwrap().port();
    drop(reservation);
    let directory = tempfile::tempdir().unwrap();
    let source = format!(r#"
grab {{ server_with_config, router, get_response, run_for, response_new, response_header }} from "std::http"
function health(request: i64) -> i64 {{
    let result: i64 = response_new(201, "created")
    response_header(result, "X-Test", "yes")
    return result
}}
function main() -> i32 {{
    let routes: i64 = router()
    get_response(routes, "/health", health)
    let server: i64 = server_with_config("127.0.0.1:{port}", 65536i64, 1000i64, 1000i64, 16i64)
    let error: string = run_for(server, routes, 1i64)
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
    assert!(response.starts_with(b"HTTP/1.1 201"));
    assert!(response.windows(b"X-Test: yes".len()).any(|window| window == b"X-Test: yes"));
    assert!(response.ends_with(b"created"));
    assert_eq!(child.wait().unwrap().code(), Some(0));
}

#[test]
fn xenith_http_worker_pool_serves_requests_concurrently() {
    let reservation = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = reservation.local_addr().unwrap().port();
    drop(reservation);
    let directory = tempfile::tempdir().unwrap();
    let source = format!(r#"
grab {{ server_with_config, router, get, run_workers }} from "std::http"
function health(method: string, path: string, body: string) -> tuple<i32, string> {{
    return (200, "worker")
}}
function main() -> i32 {{
    let routes: i64 = router()
    get(routes, "/health", health)
    let server: i64 = server_with_config("127.0.0.1:{port}", 65536i64, 1000i64, 1000i64, 16i64)
    let error: string = run_workers(server, routes, 2i64)
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
    for _ in 0..2 {
        let mut client = TcpStream::connect(("127.0.0.1", port)).unwrap();
        client.write_all(b"GET /health HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
        let mut response = Vec::new();
        client.read_to_end(&mut response).unwrap();
        assert!(response.ends_with(b"worker"));
    }
    child.kill().unwrap();
    let _ = child.wait();
}

#[test]
fn xenith_http_request_handlers_extract_typed_context() {
    let directory = tempfile::tempdir().unwrap();
    let source = r#"
grab { router, get_request, dispatch_request, request_method, request_path, request_body, request_header, request_query, request_param, router_close } from "std::http"
function show(request: i64) -> tuple<i32, string> {
    let method: string = request_method(request)
    let path: string = request_path(request)
    let body: string = request_body(request)
    let host: string = request_header(request, "Host")
    let query: string = request_query(request, "page")
    let id: string = request_param(request, "/users/:id", "id")
    return (200, "{method}:{path}:{body}:{host}:{query}:{id}")
}
function main() -> i32 {
    let routes: i64 = router()
    get_request(routes, "/users/:id", show)
    let response: tuple<i32, string> = dispatch_request(routes, "GET /users/42?page=2 HTTP/1.1")
    router_close(routes)
    if (response[0] == 200 && response[1] == "GET:/users/42?page=2:::2:42") { return 1 }
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
fn xenith_http_typed_responses_and_request_middleware() {
    let directory = tempfile::tempdir().unwrap();
    let source = r#"
grab { router, get_response, use_request, dispatch_response, response_new, response_header, response_status, response_body, response_header_value, response_close, router_close } from "std::http"
function continue_request(request: i64) -> i64 { return 0i64 }
function show(request: i64) -> i64 {
    let result: i64 = response_new(201, "created")
    response_header(result, "X-Test", "yes")
    return result
}

function main() -> i32 {
    let routes: i64 = router()
    use_request(routes, continue_request)
    get_response(routes, "/users", show)
    let result: i64 = dispatch_response(routes, "GET /users HTTP/1.1")
    let status: i32 = response_status(result)
    let body: string = response_body(result)
    let header: string = response_header_value(result, "X-Test")
    response_close(result)
    router_close(routes)
    if (status != 201) { return 10 }
    if (len(body) != 7) { return 20 }
    if (len(header) != 3) { return 30 }
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
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(1));
}

#[test]
fn xenith_http_cookies_extract_and_set_secure_attributes() {
    let directory = tempfile::tempdir().unwrap();
    let source = r#"
grab { router, get_response, dispatch_response, request_cookie, response_new, response_cookie, response_header, response_header_value, response_status, response_close, router_close } from "std::http"
function show(request: i64) -> i64 {
    let absent: string = request_cookie(request, "session")
    if (len(absent) != 0) { return response_new(400, "unexpected cookie") }
    let result: i64 = response_new(200, "ok")
    let cookie_error: string = response_cookie(result, "session", "next", 3600i64, true, true)
    if (len(cookie_error) != 0) { return response_new(500, cookie_error) }
    let cookie: string = response_header_value(result, "Set-Cookie")
    if (cookie != "session=next; Max-Age=3600; Path=/; Secure; HttpOnly; SameSite=Lax") { return response_new(500, "bad cookie") }
    response_header(result, "X-Test", "yes")
    return result
}
function main() -> i32 {
    let routes: i64 = router()
    get_response(routes, "/", show)
    let result: i64 = dispatch_response(routes, "GET / HTTP/1.1\r\nCookie: session=abc; theme=dark\r\n\r\n")
    let marker: string = response_header_value(result, "X-Test")
    let status: i32 = response_status(result)
    response_close(result)
    router_close(routes)
    if (status != 200) { return 10 }
    if (len(marker) != 3) { return 20 }
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
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(0));
}
