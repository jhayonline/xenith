use std::{io::{Read, Write}, net::TcpStream, thread};
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
