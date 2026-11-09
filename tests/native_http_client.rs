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

#[test]
fn std_http_module_exposes_bounded_client_pool() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = [0u8; 512];
        let _ = stream.read(&mut request).unwrap();
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: keep-alive\r\n\r\nok")
            .unwrap();
        let _ = stream.read(&mut request).unwrap();
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok")
            .unwrap();
        drop(stream);
        let (mut replacement, _) = listener.accept().unwrap();
        let _ = replacement.read(&mut request).unwrap();
        replacement
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok")
            .unwrap();
    });
    let (_directory, executable) = build_project_source(
        &format!(r#"
grab {{ client_pool, pool_request, pool_close }} from "std::http"
function main() -> i32 {{
    let pool: i64 = client_pool(1i64, 1000i64)
    let result: tuple<i64, string, string> = pool_request(pool, "{address}", "GET", "/", "")
    let again: tuple<i64, string, string> = pool_request(pool, "{address}", "GET", "/again", "")
    let recovered: tuple<i64, string, string> = pool_request(pool, "{address}", "GET", "/recovered", "")
    pool_close(pool)
    if (result[0] == 200i64 && result[1] == "ok" && result[2] == "" && again[0] == 200i64 && again[1] == "ok" && again[2] == "" && recovered[0] == 200i64 && recovered[1] == "ok" && recovered[2] == "") {{ return 42 }}
    return 10
}}
"#),
    );
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(42));
    server.join().unwrap();
}

#[test]
fn std_http_module_streams_response_body_in_chunks() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = [0u8; 512];
        let _ = stream.read(&mut request).unwrap();
        stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 6\r\n\r\nabc").unwrap();
        thread::sleep(std::time::Duration::from_millis(20));
        stream.write_all(b"def").unwrap();
    });
    let (_directory, executable) = build_project_source(&format!(r#"
grab {{ stream_open, stream_read, stream_close }} from "std::http"
function main() -> i32 {{
    let stream: i64 = stream_open("{address}", "GET", "/", "", 1000i64)
    let first: tuple<bytes, string, bool> = stream_read(stream, 3i64)
    let second: tuple<bytes, string, bool> = stream_read(stream, 3i64)
    stream_close(stream)
    if (len(first[0]) == 3 && len(second[0]) == 3 && first[1] == "" && second[1] == "") {{ return 42 }}
    return 10
}}
"#));
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(42));
    server.join().unwrap();
}

#[test]
fn std_http_module_decodes_chunked_streams() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = [0u8; 512];
        let _ = stream.read(&mut request).unwrap();
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n4\r\nWiki\r\n5\r\npedia\r\n0\r\n\r\n")
            .unwrap();
    });
    let (_directory, executable) = build_project_source(&format!(r#"
grab {{ stream_open, stream_read, stream_close }} from "std::http"
function main() -> i32 {{
    let stream: i64 = stream_open("{address}", "GET", "/", "", 1000i64)
    let first: tuple<bytes, string, bool> = stream_read(stream, 4i64)
    let second: tuple<bytes, string, bool> = stream_read(stream, 4i64)
    let third: tuple<bytes, string, bool> = stream_read(stream, 4i64)
    stream_close(stream)
    if (len(first[0]) == 4 && len(second[0]) == 4 && len(third[0]) == 1 && third[2] && first[1] == "" && second[1] == "" && third[1] == "") {{ return 42 }}
    return 10
}}
"#));
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(42));
    server.join().unwrap();
}
