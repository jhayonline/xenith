use std::{collections::BTreeMap, fmt, io::{self, Read, Write}, net::{SocketAddr, TcpListener, TcpStream}};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub method: String,
    pub target: String,
    pub headers: BTreeMap<String, String>,
    pub body: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HttpError {
    Incomplete,
    Malformed(String),
    Io(String),
}

impl fmt::Display for HttpError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Incomplete => write!(formatter, "incomplete HTTP request"),
            Self::Malformed(message) => write!(formatter, "malformed HTTP request: {message}"),
            Self::Io(message) => write!(formatter, "HTTP I/O error: {message}"),
        }
    }
}

impl std::error::Error for HttpError {}

impl From<io::Error> for HttpError {
    fn from(error: io::Error) -> Self { Self::Io(error.to_string()) }
}

pub fn parse_request(bytes: &[u8]) -> Result<Request, HttpError> {
    const MAX_HEADERS: usize = 64 * 1024;
    if bytes.len() > MAX_HEADERS { return Err(HttpError::Malformed("request headers are too large".into())); }
    let separator = bytes.windows(4).position(|window| window == b"\r\n\r\n").ok_or(HttpError::Incomplete)?;
    let head = std::str::from_utf8(&bytes[..separator]).map_err(|_| HttpError::Malformed("headers are not valid UTF-8".into()))?;
    let mut lines = head.split("\r\n");
    let request_line = lines.next().ok_or_else(|| HttpError::Malformed("missing request line".into()))?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next().ok_or_else(|| HttpError::Malformed("missing method".into()))?;
    let target = parts.next().ok_or_else(|| HttpError::Malformed("missing target".into()))?;
    let version = parts.next().ok_or_else(|| HttpError::Malformed("missing HTTP version".into()))?;
    if parts.next().is_some() || version != "HTTP/1.1" { return Err(HttpError::Malformed("expected an HTTP/1.1 request line".into())); }
    let mut headers = BTreeMap::new();
    for line in lines {
        let (name, value) = line.split_once(':').ok_or_else(|| HttpError::Malformed("header is missing `:`".into()))?;
        let name = name.trim();
        if name.is_empty() || !name.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-') { return Err(HttpError::Malformed("invalid header name".into())); }
        headers.insert(name.to_ascii_lowercase(), value.trim().to_owned());
    }
    let body_start = separator + 4;
    let content_length = match headers.get("content-length") {
        Some(value) => value.parse::<usize>().map_err(|_| HttpError::Malformed("invalid Content-Length".into()))?,
        None => 0,
    };
    let available = bytes.len().saturating_sub(body_start);
    if available < content_length { return Err(HttpError::Incomplete); }
    Ok(Request { method: method.to_owned(), target: target.to_owned(), headers, body: bytes[body_start..body_start + content_length].to_vec() })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    pub status: u16,
    pub headers: BTreeMap<String, String>,
    pub body: Vec<u8>,
}

impl Response {
    pub fn new(status: u16, body: Vec<u8>) -> Self { Self { status, headers: BTreeMap::new(), body } }

    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(name.into(), value.into());
        self
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut output = format!("HTTP/1.1 {} {}\r\n", self.status, reason(self.status));
        for (name, value) in &self.headers { output.push_str(&format!("{name}: {value}\r\n")); }
        output.push_str(&format!("Content-Length: {}\r\nConnection: close\r\n\r\n", self.body.len()));
        output.into_bytes().into_iter().chain(self.body.iter().copied()).collect()
    }
}

pub struct Server {
    listener: TcpListener,
}

impl Server {
    pub fn bind(address: &str) -> Result<Self, HttpError> {
        Ok(Self { listener: TcpListener::bind(address)? })
    }

    pub fn local_addr(&self) -> SocketAddr { self.listener.local_addr().expect("HTTP listener address is available") }

    pub fn accept_once<F>(&self, handler: F) -> Result<(), HttpError>
    where
        F: FnOnce(Request) -> Response,
    {
        let (mut stream, _) = self.listener.accept()?;
        let request = read_request(&mut stream)?;
        let response = handler(request);
        stream.write_all(&response.to_bytes())?;
        stream.shutdown(std::net::Shutdown::Both)?;
        Ok(())
    }
}

fn read_request(stream: &mut TcpStream) -> Result<Request, HttpError> {
    const MAX_REQUEST: usize = 1024 * 1024;
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 4096];
    loop {
        match parse_request(&bytes) {
            Ok(request) => return Ok(request),
            Err(HttpError::Malformed(message)) => return Err(HttpError::Malformed(message)),
            Err(HttpError::Io(message)) => return Err(HttpError::Io(message)),
            Err(HttpError::Incomplete) => {}
        }
        if bytes.len() >= MAX_REQUEST { return Err(HttpError::Malformed("request is too large".into())); }
        let count = stream.read(&mut buffer)?;
        if count == 0 { return Err(HttpError::Incomplete); }
        bytes.extend_from_slice(&buffer[..count]);
    }
}

fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        400 => "Bad Request",
        404 => "Not Found",
        500 => "Internal Server Error",
        _ => "Unknown",
    }
}
