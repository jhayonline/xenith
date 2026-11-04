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
