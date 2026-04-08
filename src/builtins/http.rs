//! HTTP built-in functions
//! These are called by the std::http wrapper module

use crate::error::Error;
use crate::position::Position;
use crate::runtime_result::RuntimeResult;
use crate::values::{Map, Number, Struct, Value, XenithString};
use once_cell::sync::Lazy;
use reqwest::blocking::Client;
use std::collections::HashMap;
use std::sync::Mutex;

static HTTP_CLIENT: Lazy<Mutex<Client>> = Lazy::new(|| {
    Mutex::new(
        Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap(),
    )
});

static USER_AGENT: Lazy<Mutex<String>> = Lazy::new(|| Mutex::new("Xenith/1.0".to_string()));

fn dummy_pos() -> Position {
    Position::new(0, 0, 0, "", "")
}

fn create_response_struct(status: u16, body: String, headers: HashMap<String, String>) -> Value {
    let mut headers_map = Map::new();
    for (k, v) in headers {
        headers_map.set(k, Value::String(XenithString::new(v)));
    }

    let mut response = Struct::new("HttpResponse".to_string());
    response.set_field(
        "status".to_string(),
        Value::Number(Number::new(status as f64)),
    );
    response.set_field("body".to_string(), Value::String(XenithString::new(body)));
    response.set_field("headers".to_string(), Value::Map(headers_map));

    Value::Struct(response)
}

fn get_headers_map(args: &[Value], index: usize) -> Option<HashMap<String, String>> {
    if args.len() <= index {
        return Some(HashMap::new());
    }

    match &args[index] {
        Value::Map(m) => {
            let mut headers = HashMap::new();
            for (k, v) in &m.pairs {
                if let Value::String(s) = v {
                    headers.insert(k.clone(), s.value.clone());
                }
            }
            Some(headers)
        }
        _ => None,
    }
}

fn get_body_string(args: &[Value], index: usize) -> Option<String> {
    if args.len() <= index {
        return None;
    }

    match &args[index] {
        Value::String(s) => Some(s.value.clone()),
        Value::Json(j) => Some(j.value.to_string()),
        _ => None,
    }
}

pub fn get(args: Vec<Value>) -> RuntimeResult {
    if args.len() < 1 || args.len() > 2 {
        return RuntimeResult::new().failure(
            Error::new(
                dummy_pos(),
                dummy_pos(),
                "Argument Error",
                "__http_get expects 1-2 arguments (url, headers?)",
            )
            .with_code("XEN100"),
        );
    }

    let url = match &args[0] {
        Value::String(s) => &s.value,
        _ => {
            return RuntimeResult::new().failure(Error::type_mismatch(
                "string",
                "other",
                dummy_pos(),
                dummy_pos(),
            ));
        }
    };

    let headers = get_headers_map(&args, 1).unwrap_or_default();

    let client = HTTP_CLIENT.lock().unwrap();
    let user_agent = USER_AGENT.lock().unwrap();

    let mut request = client.get(url);
    request = request.header("User-Agent", user_agent.as_str());

    for (k, v) in headers {
        request = request.header(&k, v);
    }

    match request.send() {
        Ok(response) => {
            let status = response.status().as_u16();
            let mut headers_map = HashMap::new();
            for (k, v) in response.headers().iter() {
                let name = k.as_str();
                headers_map.insert(name.to_string(), v.to_str().unwrap_or("").to_string());
            }

            match response.text() {
                Ok(body) => {
                    RuntimeResult::new().success(create_response_struct(status, body, headers_map))
                }
                Err(e) => RuntimeResult::new().failure(
                    Error::new(
                        dummy_pos(),
                        dummy_pos(),
                        "HTTP Error",
                        &format!("Failed to read response body: {}", e),
                    )
                    .with_code("XEN200"),
                ),
            }
        }
        Err(e) => RuntimeResult::new().failure(
            Error::new(
                dummy_pos(),
                dummy_pos(),
                "HTTP Request Failed",
                &format!("{}", e),
            )
            .with_code("XEN200")
            .with_note(&format!("Failed to request URL: {}", url))
            .with_help("Check your network connection and URL"),
        ),
    }
}

pub fn post(args: Vec<Value>) -> RuntimeResult {
    if args.len() < 2 || args.len() > 3 {
        return RuntimeResult::new().failure(
            Error::new(
                dummy_pos(),
                dummy_pos(),
                "Argument Error",
                "__http_post expects 2-3 arguments (url, body, headers?)",
            )
            .with_code("XEN100"),
        );
    }

    let url = match &args[0] {
        Value::String(s) => &s.value,
        _ => {
            return RuntimeResult::new().failure(Error::type_mismatch(
                "string",
                "other",
                dummy_pos(),
                dummy_pos(),
            ));
        }
    };

    let body = match get_body_string(&args, 1) {
        Some(b) => b,
        None => {
            return RuntimeResult::new().failure(Error::type_mismatch(
                "string or json",
                "other",
                dummy_pos(),
                dummy_pos(),
            ));
        }
    };

    let headers = get_headers_map(&args, 2).unwrap_or_default();

    let client = HTTP_CLIENT.lock().unwrap();
    let user_agent = USER_AGENT.lock().unwrap();

    let mut request = client.post(url).body(body);
    request = request.header("User-Agent", user_agent.as_str());

    for (k, v) in headers {
        request = request.header(&k, v);
    }

    match request.send() {
        Ok(response) => {
            let status = response.status().as_u16();
            let mut headers_map = HashMap::new();
            for (k, v) in response.headers().iter() {
                let name = k.as_str();
                headers_map.insert(name.to_string(), v.to_str().unwrap_or("").to_string());
            }

            match response.text() {
                Ok(body) => {
                    RuntimeResult::new().success(create_response_struct(status, body, headers_map))
                }
                Err(e) => RuntimeResult::new().failure(
                    Error::new(
                        dummy_pos(),
                        dummy_pos(),
                        "HTTP Error",
                        &format!("Failed to read response body: {}", e),
                    )
                    .with_code("XEN200"),
                ),
            }
        }
        Err(e) => RuntimeResult::new().failure(
            Error::new(
                dummy_pos(),
                dummy_pos(),
                "HTTP Request Failed",
                &format!("{}", e),
            )
            .with_code("XEN200")
            .with_note(&format!("Failed to POST to URL: {}", url))
            .with_help("Check your network connection and URL"),
        ),
    }
}

pub fn put(args: Vec<Value>) -> RuntimeResult {
    if args.len() < 2 || args.len() > 3 {
        return RuntimeResult::new().failure(
            Error::new(
                dummy_pos(),
                dummy_pos(),
                "Argument Error",
                "__http_put expects 2-3 arguments (url, body, headers?)",
            )
            .with_code("XEN100"),
        );
    }

    let url = match &args[0] {
        Value::String(s) => &s.value,
        _ => {
            return RuntimeResult::new().failure(Error::type_mismatch(
                "string",
                "other",
                dummy_pos(),
                dummy_pos(),
            ));
        }
    };

    let body = match get_body_string(&args, 1) {
        Some(b) => b,
        None => {
            return RuntimeResult::new().failure(Error::type_mismatch(
                "string or json",
                "other",
                dummy_pos(),
                dummy_pos(),
            ));
        }
    };

    let headers = get_headers_map(&args, 2).unwrap_or_default();

    let client = HTTP_CLIENT.lock().unwrap();
    let user_agent = USER_AGENT.lock().unwrap();

    let mut request = client.put(url).body(body);
    request = request.header("User-Agent", user_agent.as_str());

    for (k, v) in headers {
        request = request.header(&k, v);
    }

    match request.send() {
        Ok(response) => {
            let status = response.status().as_u16();
            let mut headers_map = HashMap::new();
            for (k, v) in response.headers().iter() {
                let name = k.as_str();
                headers_map.insert(name.to_string(), v.to_str().unwrap_or("").to_string());
            }

            match response.text() {
                Ok(body) => {
                    RuntimeResult::new().success(create_response_struct(status, body, headers_map))
                }
                Err(e) => RuntimeResult::new().failure(
                    Error::new(
                        dummy_pos(),
                        dummy_pos(),
                        "HTTP Error",
                        &format!("Failed to read response body: {}", e),
                    )
                    .with_code("XEN200"),
                ),
            }
        }
        Err(e) => RuntimeResult::new().failure(
            Error::new(
                dummy_pos(),
                dummy_pos(),
                "HTTP Request Failed",
                &format!("{}", e),
            )
            .with_code("XEN200")
            .with_note(&format!("Failed to PUT to URL: {}", url))
            .with_help("Check your network connection and URL"),
        ),
    }
}

pub fn delete(args: Vec<Value>) -> RuntimeResult {
    if args.len() < 1 || args.len() > 2 {
        return RuntimeResult::new().failure(
            Error::new(
                dummy_pos(),
                dummy_pos(),
                "Argument Error",
                "__http_delete expects 1-2 arguments (url, headers?)",
            )
            .with_code("XEN100"),
        );
    }

    let url = match &args[0] {
        Value::String(s) => &s.value,
        _ => {
            return RuntimeResult::new().failure(Error::type_mismatch(
                "string",
                "other",
                dummy_pos(),
                dummy_pos(),
            ));
        }
    };

    let headers = get_headers_map(&args, 1).unwrap_or_default();

    let client = HTTP_CLIENT.lock().unwrap();
    let user_agent = USER_AGENT.lock().unwrap();

    let mut request = client.delete(url);
    request = request.header("User-Agent", user_agent.as_str());

    for (k, v) in headers {
        request = request.header(&k, v);
    }

    match request.send() {
        Ok(response) => {
            let status = response.status().as_u16();
            let mut headers_map = HashMap::new();
            for (k, v) in response.headers().iter() {
                let name = k.as_str();
                headers_map.insert(name.to_string(), v.to_str().unwrap_or("").to_string());
            }

            match response.text() {
                Ok(body) => {
                    RuntimeResult::new().success(create_response_struct(status, body, headers_map))
                }
                Err(e) => RuntimeResult::new().failure(
                    Error::new(
                        dummy_pos(),
                        dummy_pos(),
                        "HTTP Error",
                        &format!("Failed to read response body: {}", e),
                    )
                    .with_code("XEN200"),
                ),
            }
        }
        Err(e) => RuntimeResult::new().failure(
            Error::new(
                dummy_pos(),
                dummy_pos(),
                "HTTP Request Failed",
                &format!("{}", e),
            )
            .with_code("XEN200")
            .with_note(&format!("Failed to DELETE URL: {}", url))
            .with_help("Check your network connection and URL"),
        ),
    }
}

pub fn patch(args: Vec<Value>) -> RuntimeResult {
    if args.len() < 2 || args.len() > 3 {
        return RuntimeResult::new().failure(
            Error::new(
                dummy_pos(),
                dummy_pos(),
                "Argument Error",
                "__http_patch expects 2-3 arguments (url, body, headers?)",
            )
            .with_code("XEN100"),
        );
    }

    let url = match &args[0] {
        Value::String(s) => &s.value,
        _ => {
            return RuntimeResult::new().failure(Error::type_mismatch(
                "string",
                "other",
                dummy_pos(),
                dummy_pos(),
            ));
        }
    };

    let body = match get_body_string(&args, 1) {
        Some(b) => b,
        None => {
            return RuntimeResult::new().failure(Error::type_mismatch(
                "string or json",
                "other",
                dummy_pos(),
                dummy_pos(),
            ));
        }
    };

    let headers = get_headers_map(&args, 2).unwrap_or_default();

    let client = HTTP_CLIENT.lock().unwrap();
    let user_agent = USER_AGENT.lock().unwrap();

    let mut request = client.patch(url).body(body);
    request = request.header("User-Agent", user_agent.as_str());

    for (k, v) in headers {
        request = request.header(&k, v);
    }

    match request.send() {
        Ok(response) => {
            let status = response.status().as_u16();
            let mut headers_map = HashMap::new();
            for (k, v) in response.headers().iter() {
                let name = k.as_str();
                headers_map.insert(name.to_string(), v.to_str().unwrap_or("").to_string());
            }

            match response.text() {
                Ok(body) => {
                    RuntimeResult::new().success(create_response_struct(status, body, headers_map))
                }
                Err(e) => RuntimeResult::new().failure(
                    Error::new(
                        dummy_pos(),
                        dummy_pos(),
                        "HTTP Error",
                        &format!("Failed to read response body: {}", e),
                    )
                    .with_code("XEN200"),
                ),
            }
        }
        Err(e) => RuntimeResult::new().failure(
            Error::new(
                dummy_pos(),
                dummy_pos(),
                "HTTP Request Failed",
                &format!("{}", e),
            )
            .with_code("XEN200")
            .with_note(&format!("Failed to PATCH URL: {}", url))
            .with_help("Check your network connection and URL"),
        ),
    }
}

pub fn set_timeout(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            Error::new(
                dummy_pos(),
                dummy_pos(),
                "Argument Error",
                "__http_set_timeout expects 1 argument (seconds)",
            )
            .with_code("XEN100"),
        );
    }

    let seconds = match &args[0] {
        Value::Number(n) => n.value as u64,
        _ => {
            return RuntimeResult::new().failure(Error::type_mismatch(
                "number",
                "other",
                dummy_pos(),
                dummy_pos(),
            ));
        }
    };

    let new_client = Client::builder()
        .timeout(std::time::Duration::from_secs(seconds))
        .build()
        .unwrap();

    let mut client = HTTP_CLIENT.lock().unwrap();
    *client = new_client;

    RuntimeResult::new().success(Value::Number(Number::null()))
}

pub fn set_user_agent(args: Vec<Value>) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            Error::new(
                dummy_pos(),
                dummy_pos(),
                "Argument Error",
                "__http_set_user_agent expects 1 argument (agent)",
            )
            .with_code("XEN100"),
        );
    }

    let agent = match &args[0] {
        Value::String(s) => s.value.clone(),
        _ => {
            return RuntimeResult::new().failure(Error::type_mismatch(
                "string",
                "other",
                dummy_pos(),
                dummy_pos(),
            ));
        }
    };

    let mut user_agent = USER_AGENT.lock().unwrap();
    *user_agent = agent;

    RuntimeResult::new().success(Value::Number(Number::null()))
}
