use std::{
    io::{Read, Write},
    net::TcpListener,
    path::Path,
    process::Command,
    thread,
};

use xenith::native::{
    check::check,
    codegen::{emit_object, link_host},
    syntax::parse,
};

fn build_source(source: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    let program = parse(Path::new("src/main.xen"), source).unwrap();
    let typed = check(&program).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let object = directory.path().join("main.o");
    let executable = directory.path().join("main");
    emit_object(&typed, &object).unwrap();
    link_host(&object, &executable).unwrap();
    (directory, executable)
}

fn build_project_source(source: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    let directory = tempfile::tempdir().unwrap();
    let main = directory.path().join("main.xen");
    std::fs::write(&main, source).unwrap();
    let (program, _) = xenith::native::project::load_program(&main).unwrap();
    let typed = check(&program).unwrap();
    let object = directory.path().join("main.o");
    let executable = directory.path().join("main");
    emit_object(&typed, &object).unwrap();
    link_host(&object, &executable).unwrap();
    (directory, executable)
}

#[test]
fn native_http_client_returns_status_body_and_empty_error() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = [0u8; 1024];
        let size = stream.read(&mut request).unwrap();
        assert!(std::str::from_utf8(&request[..size])
            .unwrap()
            .starts_with("GET /health HTTP/1.1"));
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok")
            .unwrap();
    });
    let (_directory, executable) = build_source(&format!(
        r#"
fn main() -> i32 {{
    let result: tuple<i64, string, string> = http_client_request("{}", "GET", "/health", "", 1000i64)
    if (result[0] != 200i64) {{ return 10 }}
    if (result[1] != "ok") {{ return 20 }}
    if (result[2] != "") {{ return 30 }}
    return 42
}}
"#,
        address,
    ));
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(42));
    server.join().unwrap();
}

#[test]
fn native_http_client_rejects_wrong_argument_types() {
    let program = parse(
        Path::new("src/main.xen"),
        "fn main() -> i32 { let value: tuple<i64, string, string> = http_client_request(1i64, \"GET\", \"/\", \"\", 1000i64) return 0 }",
    )
    .unwrap();
    let errors = check(&program).expect_err("invalid client address should fail static checking");
    assert!(errors.iter().any(|error| error
        .message
        .contains("argument type does not match `http_client_request`")));
}

#[test]
fn native_https_client_reports_recoverable_connection_errors() {
    let (_directory, executable) = build_source(
        r#"
fn main() -> i32 {
    let result: tuple<i64, string, string> = https_client_request("127.0.0.1:1", "GET", "/", "", 100i64)
    if (len(result[2]) == 0) { return 10 }
    return 42
}
"#,
    );
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(42));
}

#[test]
fn native_https_client_rejects_wrong_argument_types() {
    let program = parse(
        Path::new("src/main.xen"),
        "fn main() -> i32 { let value: tuple<i64, string, string> = https_client_request(1i64, \"GET\", \"/\", \"\", 100i64) return 0 }",
    )
    .unwrap();
    let errors = check(&program).expect_err("invalid HTTPS address should fail static checking");
    assert!(errors.iter().any(|error| error
        .message
        .contains("argument type does not match `https_client_request`")));
}

#[test]
fn std_http_module_exposes_request() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = [0u8; 1024];
        let size = stream.read(&mut request).unwrap();
        assert!(std::str::from_utf8(&request[..size])
            .unwrap()
            .contains("POST /users HTTP/1.1"));
        stream
            .write_all(
                b"HTTP/1.1 201 Created\r\nContent-Length: 7\r\nConnection: close\r\n\r\ncreated",
            )
            .unwrap();
    });
    let (_directory, executable) = build_project_source(&format!(
        r#"
grab {{ request }} from "std::http"
function main() -> i32 {{
    let result: tuple<i64, string, string> = request("{}", "POST", "/users", "Ada", 1000i64)
    if (result[0] != 201i64) {{ return 10 }}
    if (result[1] != "created") {{ return 20 }}
    if (result[2] != "") {{ return 30 }}
    return 42
}}
"#,
        address,
    ));
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(42));
    server.join().unwrap();
}

#[test]
fn std_http_module_exposes_secure_request() {
    let (_directory, executable) = build_project_source(
        r#"
grab { secure_request } from "std::http"
function main() -> i32 {
    let result: tuple<i64, string, string> = secure_request("127.0.0.1:1", "GET", "/", "", 100i64)
    if (len(result[2]) == 0) { return 10 }
    return 42
}
"#,
    );
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(42));
}
