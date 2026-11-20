//! Helpers shared by the test binaries that `mod common;` this file.

use std::{
    io::ErrorKind,
    net::TcpStream,
    thread,
    time::{Duration, Instant},
};

/// How long a freshly spawned server gets to start listening. macOS assesses a
/// newly linked executable on its first launch, which takes 300-550 ms, so the
/// fixed 100 ms sleeps these tests used to rely on were not enough there.
pub const STARTUP_DEADLINE: Duration = Duration::from_secs(10);

/// Connect to a server the test has just spawned, retrying while it is still
/// starting. A refused connection never reaches the server, so retrying cannot
/// use up one of the requests a bounded server is waiting for.
pub fn connect_when_ready(port: u16) -> TcpStream {
    let deadline = Instant::now() + STARTUP_DEADLINE;
    loop {
        match TcpStream::connect(("127.0.0.1", port)) {
            Ok(stream) => return stream,
            Err(error) if error.kind() == ErrorKind::ConnectionRefused && Instant::now() < deadline => {
                thread::sleep(Duration::from_millis(10));
            }
            Err(error) => panic!("server on port {port} never accepted a connection: {error}"),
        }
    }
}
