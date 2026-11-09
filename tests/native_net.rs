use std::{fs, io::Write, net::TcpListener, path::Path, process::Command, thread};
use xenith::native::{
    check::check,
    codegen::{emit_object, link_host},
    syntax::parse,
};

/// Builds a single-file program that uses only builtins.
fn build_builtin_source(source: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    let program = parse(Path::new("src/main.xen"), source).unwrap();
    let typed = check(&program).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let object = directory.path().join("main.o");
    let executable = directory.path().join("main");
    emit_object(&typed, &object).unwrap();
    link_host(&object, &executable).unwrap();
    (directory, executable)
}

/// Builds a program through the project loader so `grab ... from "std::net"`
/// resolves.
fn build_project_source(source: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    let directory = tempfile::tempdir().unwrap();
    let main = directory.path().join("main.xen");
    fs::write(&main, source).unwrap();
    let (program, _) = xenith::native::project::load_program(&main).unwrap();
    let typed = check(&program).unwrap();
    let object = directory.path().join("main.o");
    let executable = directory.path().join("main");
    emit_object(&typed, &object).unwrap();
    link_host(&object, &executable).unwrap();
    (directory, executable)
}

fn assert_error(source: &str, fragment: &str) {
    let program = parse(Path::new("src/main.xen"), source).expect("source should parse");
    let errors = check(&program).expect_err("source should not type-check");
    assert!(
        errors.iter().any(|error| error.message.contains(fragment)),
        "{errors:?}"
    );
}

/// Binding to port 0 and reading the address back is what makes the round trip
/// tests independent of whichever ports happen to be free on the machine.
#[test]
fn native_tcp_local_address_reports_the_bound_ephemeral_port() {
    let (_directory, executable) = build_builtin_source(
        r#"
fn main() -> i32 {
    let listener: i64 = tcp_bind("127.0.0.1:0", 8i64)
    let address: string = tcp_local(listener)
    tcp_close(listener)
    if (len(address) < 12) { return 10 }
    return 42
}
"#,
    );
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(42));
}

/// Connect, accept, write and read in one process. The client connects before
/// the server accepts, so the kernel backlog holds the connection and no
/// threads are needed.
#[test]
fn native_tcp_client_and_server_exchange_bytes() {
    let (_directory, executable) = build_builtin_source(
        r#"
fn main() -> i32 {
    let listener: i64 = tcp_bind("127.0.0.1:0", 8i64)
    let address: string = tcp_local(listener)
    let client: i64 = tcp_connect(address)
    let server: i64 = tcp_accept(listener)
    let sent: string = tcp_write(client, bytes("ping"))
    if (len(sent) != 0) { return 10 }
    let received: tuple<bytes, string> = tcp_read(server, 64i64)
    if (len(received[1]) != 0) { return 20 }
    let data: bytes = received[0]
    if (len(data) != 4) { return 30 }
    if (data[0] != 112) { return 40 }
    if (data[3] != 103) { return 50 }
    tcp_close(client)
    tcp_close(server)
    tcp_close(listener)
    return 42
}
"#,
    );
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(42));
}

#[test]
fn native_tcp_peer_address_reports_the_connected_client() {
    let (_directory, executable) = build_builtin_source(
        r#"
fn main() -> i32 {
    let listener: i64 = tcp_bind("127.0.0.1:0", 8i64)
    let address: string = tcp_local(listener)
    let client: i64 = tcp_connect(address)
    let server: i64 = tcp_accept(listener)
    let peer: string = tcp_peer(server)
    let local: string = tcp_local(server)
    tcp_close(client)
    tcp_close(server)
    tcp_close(listener)
    if (len(peer) < 12) { return 10 }
    if (local != address) { return 20 }
    return 42
}
"#,
    );
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(42));
}

/// A read timeout must make a quiet connection fail instead of hanging the
/// process forever.
#[test]
fn native_tcp_read_timeout_fails_instead_of_blocking() {
    let (_directory, executable) = build_builtin_source(
        r#"
fn main() -> i32 {
    let listener: i64 = tcp_bind("127.0.0.1:0", 8i64)
    let address: string = tcp_local(listener)
    let client: i64 = tcp_connect(address)
    let server: i64 = tcp_accept(listener)
    let applied: string = tcp_timeout(server, 50i64, 50i64)
    if (len(applied) != 0) { return 10 }
    let received: tuple<bytes, string> = tcp_read(server, 64i64)
    tcp_close(client)
    tcp_close(server)
    tcp_close(listener)
    if (len(received[1]) == 0) { return 20 }
    return 42
}
"#,
    );
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(42));
}

#[test]
fn native_tcp_timeout_rejects_negative_values_recoverably() {
    let (_directory, executable) = build_builtin_source(
        r#"
fn main() -> i32 {
    let listener: i64 = tcp_bind("127.0.0.1:0", 8i64)
    let applied: string = tcp_timeout(listener, 0i64 - 1i64, 0i64)
    tcp_close(listener)
    if (len(applied) == 0) { return 10 }
    return 42
}
"#,
    );
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(42));
}

/// The whole point of the std::net wrappers is that programs never name the
/// builtins. Nothing exercised that path before.
#[test]
fn std_net_module_round_trips_a_connection() {
    let (_directory, executable) = build_project_source(
        r#"
grab { bind, accept, connect, read, write, close, local_address, peer_address } from "std::net"

function main() -> i32 {
    let listener: i64 = bind("127.0.0.1:0", 8i64)
    let address: string = local_address(listener)
    let client: i64 = connect(address)
    let server: i64 = accept(listener)
    let sent: string = write(client, bytes("hello"))
    if (len(sent) != 0) { return 10 }
    let received: tuple<bytes, string> = read(server, 64i64)
    if (len(received[1]) != 0) { return 20 }
    let data: bytes = received[0]
    if (len(data) != 5) { return 30 }
    if (data[0] != 104) { return 40 }
    let peer: string = peer_address(server)
    if (len(peer) < 12) { return 50 }
    close(client)
    close(server)
    close(listener)
    return 42
}
"#,
    );
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(42));
}

#[test]
fn std_net_module_applies_timeouts() {
    let (_directory, executable) = build_project_source(
        r#"
grab { bind, accept, connect, read, close, local_address, timeout } from "std::net"

function main() -> i32 {
    let listener: i64 = bind("127.0.0.1:0", 8i64)
    let address: string = local_address(listener)
    let client: i64 = connect(address)
    let server: i64 = accept(listener)
    let applied: string = timeout(server, 50i64, 50i64)
    if (len(applied) != 0) { return 10 }
    let received: tuple<bytes, string> = read(server, 64i64)
    close(client)
    close(server)
    close(listener)
    if (len(received[1]) == 0) { return 20 }
    return 42
}

"#,
    );
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(42));
}

#[test]
fn native_tcp_connect_resolves_hostnames() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream.write_all(b"ok").unwrap();
    });
    let (_directory, executable) = build_builtin_source(&format!(
        "fn main() -> i32 {{ let connection: i64 = tcp_connect(\"localhost:{}\") tcp_close(connection) return 42 }}",
        port
    ));
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(42));
    server.join().unwrap();
}

#[test]
fn native_tcp_connect_supports_ipv6_loopback() {
    let listener = TcpListener::bind("[::1]:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream.write_all(b"ok").unwrap();
    });
    let (_directory, executable) = build_builtin_source(&format!(
        "fn main() -> i32 {{ let connection: i64 = tcp_connect(\"[::1]:{}\") let peer: string = tcp_peer(connection) tcp_close(connection) if (len(peer) < 5) {{ return 10 }} return 42 }}",
        port
    ));
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(42));
    server.join().unwrap();
}

#[test]
fn native_tcp_bind_reports_ipv6_addresses_in_brackets() {
    let (_directory, executable) = build_builtin_source(
        r#"
fn main() -> i32 {
    let listener: i64 = tcp_bind("[::1]:0", 8i64)
    let address: string = tcp_local(listener)
    tcp_close(listener)
    if (len(address) < 8) { return 10 }
    if (len(address) < 8) { return 20 }
    return 42
}
"#,
    );
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(42));
}

#[test]
fn rejects_malformed_tcp_calls_statically() {
    assert_error(
        "fn main() -> i32 { let socket: i64 = tcp_connect(8080) return 0 }",
        "argument type does not match `tcp_connect`",
    );
    assert_error(
        "fn main() -> i32 { let address: string = tcp_peer(\"127.0.0.1:80\") return 0 }",
        "argument type does not match `tcp_peer`",
    );
    assert_error(
        "fn main() -> i32 { let applied: string = tcp_timeout(1i64, 0i64) return 0 }",
        "arguments",
    );
}
