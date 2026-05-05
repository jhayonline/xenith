//! HTTP built-in functions }
//! These are called by the std::http wrapper module

use crate::error::Error;
use crate::json::Json;
use crate::position::Position;
use crate::runtime_result::RuntimeResult;
use crate::values::{List, Map, Number, Struct, Value, XenithString};
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

// Helper to convert Xenith Value to JSON string for HTTP body
fn value_to_json_string(value: &Value) -> Result<String, Error> {
    match value {
        Value::Json(j) => Ok(j.to_string()),
        Value::String(s) => Ok(s.value.clone()),
        Value::Map(m) => {
            let mut obj = serde_json::Map::new();
            for (k, v) in &m.pairs {
                let json_val = xenith_to_serde_json(v)?;
                obj.insert(k.clone(), json_val);
            }
            Ok(serde_json::to_string(&obj).unwrap_or_else(|_| "{}".to_string()))
        }
        Value::List(l) => {
            let mut arr = Vec::new();
            for elem in &l.elements {
                arr.push(xenith_to_serde_json(elem)?);
            }
            Ok(serde_json::to_string(&arr).unwrap_or_else(|_| "[]".to_string()))
        }
        Value::Number(n) => {
            if n.value.fract() == 0.0 {
                Ok(format!("{}", n.value as i64))
            } else {
                Ok(n.value.to_string())
            }
        }
        Value::Bool(b) => Ok(b.to_string()),
        // Remove Value::Null - use Number check instead
        _ => {
            // Check if it's null (Number with value 0.0)
            if let Value::Number(n) = value {
                if n.value == 0.0 {
                    return Ok("null".to_string());
                }
            }
            Err(Error::invalid_conversion(
                "value",
                "json string",
                crate::position::Position::new(0, 0, 0, "", ""),
                crate::position::Position::new(0, 0, 0, "", ""),
            ))
        }
    }
}

// Helper to convert Xenith Value to serde_json::Value
fn xenith_to_serde_json(value: &Value) -> Result<serde_json::Value, Error> {
    match value {
        Value::Number(n) => {
            if n.value == 0.0 {
                // This represents null
                Ok(serde_json::Value::Null)
            } else if n.value.fract() == 0.0 {
                Ok(serde_json::Value::Number(serde_json::Number::from(
                    n.value as i64,
                )))
            } else {
                serde_json::Number::from_f64(n.value)
                    .map(serde_json::Value::Number)
                    .ok_or_else(|| {
                        Error::invalid_conversion(
                            "number",
                            "json",
                            crate::position::Position::new(0, 0, 0, "", ""),
                            crate::position::Position::new(0, 0, 0, "", ""),
                        )
                    })
            }
        }
        Value::String(s) => Ok(serde_json::Value::String(s.value.clone())),
        Value::Bool(b) => Ok(serde_json::Value::Bool(*b)),
        Value::List(l) => {
            let mut arr = Vec::new();
            for elem in &l.elements {
                arr.push(xenith_to_serde_json(elem)?);
            }
            Ok(serde_json::Value::Array(arr))
        }
        Value::Map(m) => {
            let mut obj = serde_json::Map::new();
            for (k, v) in &m.pairs {
                obj.insert(k.clone(), xenith_to_serde_json(v)?);
            }
            Ok(serde_json::Value::Object(obj))
        }
        Value::Json(j) => {
            // Convert our Json enum to serde_json::Value
            // We need to implement Serialize for Json
            match j {
                Json::Null => Ok(serde_json::Value::Null),
                Json::Bool(b) => Ok(serde_json::Value::Bool(*b)),
                Json::Number(n) => serde_json::Number::from_f64(*n)
                    .map(serde_json::Value::Number)
                    .ok_or_else(|| {
                        Error::invalid_conversion(
                            "number",
                            "json",
                            crate::position::Position::new(0, 0, 0, "", ""),
                            crate::position::Position::new(0, 0, 0, "", ""),
                        )
                    }),
                Json::String(s) => Ok(serde_json::Value::String(s.clone())),
                Json::Array(arr) => {
                    let mut new_arr = Vec::new();
                    for item in arr {
                        new_arr.push(xenith_to_serde_json(&json_to_value(item))?);
                    }
                    Ok(serde_json::Value::Array(new_arr))
                }
                Json::Object(obj) => {
                    let mut new_obj = serde_json::Map::new();
                    for (k, v) in obj {
                        new_obj.insert(k.clone(), xenith_to_serde_json(&json_to_value(v))?);
                    }
                    Ok(serde_json::Value::Object(new_obj))
                }
            }
        }
        _ => Ok(serde_json::Value::Null),
    }
}

pub fn get(args: Vec<Value>, call_pos: Position) -> RuntimeResult {
    if args.len() < 1 || args.len() > 2 {
        return RuntimeResult::new().failure(
            Error::new(
                call_pos.clone(),
                call_pos,
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
                call_pos.clone(),
                call_pos,
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
                        call_pos.clone(),
                        call_pos,
                        "HTTP Error",
                        &format!("Failed to read response body: {}", e),
                    )
                    .with_code("XEN200"),
                ),
            }
        }
        Err(e) => RuntimeResult::new().failure(
            Error::new(
                call_pos.clone(),
                call_pos,
                "HTTP Request Failed",
                &format!("{}", e),
            )
            .with_code("XEN200")
            .with_note(&format!("Failed to request URL: {}", url))
            .with_help("Check your network connection and URL"),
        ),
    }
}

pub fn post(args: Vec<Value>, call_pos: Position) -> RuntimeResult {
    if args.len() < 2 || args.len() > 3 {
        return RuntimeResult::new().failure(
            Error::new(
                call_pos.clone(),
                call_pos,
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
                call_pos.clone(),
                call_pos,
            ));
        }
    };

    // Convert body to JSON string
    let body_string = match value_to_json_string(&args[1]) {
        Ok(s) => s,
        Err(e) => return RuntimeResult::new().failure(e),
    };

    // Get headers or create default
    let mut headers = get_headers_map(&args, 2).unwrap_or_default();

    // Auto-add Content-Type header if not present and body looks like JSON
    if !headers.contains_key("Content-Type") {
        // Check if body is JSON or map
        match &args[1] {
            Value::Json(_) | Value::Map(_) => {
                headers.insert("Content-Type".to_string(), "application/json".to_string());
            }
            _ => {
                headers.insert("Content-Type".to_string(), "text/plain".to_string());
            }
        }
    }

    let client = HTTP_CLIENT.lock().unwrap();
    let user_agent = USER_AGENT.lock().unwrap();

    let mut request = client.post(url).body(body_string);
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
                        call_pos.clone(),
                        call_pos,
                        "HTTP Error",
                        &format!("Failed to read response body: {}", e),
                    )
                    .with_code("XEN200"),
                ),
            }
        }
        Err(e) => RuntimeResult::new().failure(
            Error::new(
                call_pos.clone(),
                call_pos,
                "HTTP Request Failed",
                &format!("{}", e),
            )
            .with_code("XEN200")
            .with_note(&format!("Failed to POST to URL: {}", url))
            .with_help("Check your network connection and URL"),
        ),
    }
}

pub fn put(args: Vec<Value>, call_pos: Position) -> RuntimeResult {
    if args.len() < 2 || args.len() > 3 {
        return RuntimeResult::new().failure(
            Error::new(
                call_pos.clone(),
                call_pos,
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
                call_pos.clone(),
                call_pos,
            ));
        }
    };

    let body_string = match value_to_json_string(&args[1]) {
        Ok(s) => s,
        Err(e) => return RuntimeResult::new().failure(e),
    };

    let mut headers = get_headers_map(&args, 2).unwrap_or_default();

    if !headers.contains_key("Content-Type") && matches!(&args[1], Value::Json(_) | Value::Map(_)) {
        headers.insert("Content-Type".to_string(), "application/json".to_string());
    }

    let client = HTTP_CLIENT.lock().unwrap();
    let user_agent = USER_AGENT.lock().unwrap();

    let mut request = client.put(url).body(body_string);
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
                        call_pos.clone(),
                        call_pos,
                        "HTTP Error",
                        &format!("Failed to read response body: {}", e),
                    )
                    .with_code("XEN200"),
                ),
            }
        }
        Err(e) => RuntimeResult::new().failure(
            Error::new(
                call_pos.clone(),
                call_pos,
                "HTTP Request Failed",
                &format!("{}", e),
            )
            .with_code("XEN200")
            .with_note(&format!("Failed to PUT to URL: {}", url))
            .with_help("Check your network connection and URL"),
        ),
    }
}

pub fn delete(args: Vec<Value>, call_pos: Position) -> RuntimeResult {
    if args.len() < 1 || args.len() > 2 {
        return RuntimeResult::new().failure(
            Error::new(
                call_pos.clone(),
                call_pos,
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
                call_pos.clone(),
                call_pos,
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
                        call_pos.clone(),
                        call_pos,
                        "HTTP Error",
                        &format!("Failed to read response body: {}", e),
                    )
                    .with_code("XEN200"),
                ),
            }
        }
        Err(e) => RuntimeResult::new().failure(
            Error::new(
                call_pos.clone(),
                call_pos,
                "HTTP Request Failed",
                &format!("{}", e),
            )
            .with_code("XEN200")
            .with_note(&format!("Failed to DELETE URL: {}", url))
            .with_help("Check your network connection and URL"),
        ),
    }
}

pub fn patch(args: Vec<Value>, call_pos: Position) -> RuntimeResult {
    if args.len() < 2 || args.len() > 3 {
        return RuntimeResult::new().failure(
            Error::new(
                call_pos.clone(),
                call_pos,
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
                call_pos.clone(),
                call_pos,
            ));
        }
    };

    let body_string = match value_to_json_string(&args[1]) {
        Ok(s) => s,
        Err(e) => return RuntimeResult::new().failure(e),
    };

    let mut headers = get_headers_map(&args, 2).unwrap_or_default();

    if !headers.contains_key("Content-Type") && matches!(&args[1], Value::Json(_) | Value::Map(_)) {
        headers.insert("Content-Type".to_string(), "application/json".to_string());
    }

    let client = HTTP_CLIENT.lock().unwrap();
    let user_agent = USER_AGENT.lock().unwrap();

    let mut request = client.patch(url).body(body_string);
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
                        call_pos.clone(),
                        call_pos,
                        "HTTP Error",
                        &format!("Failed to read response body: {}", e),
                    )
                    .with_code("XEN200"),
                ),
            }
        }
        Err(e) => RuntimeResult::new().failure(
            Error::new(
                call_pos.clone(),
                call_pos,
                "HTTP Request Failed",
                &format!("{}", e),
            )
            .with_code("XEN200")
            .with_note(&format!("Failed to PATCH URL: {}", url))
            .with_help("Check your network connection and URL"),
        ),
    }
}

pub fn set_timeout(args: Vec<Value>, call_pos: Position) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            Error::new(
                call_pos.clone(),
                call_pos,
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
                call_pos.clone(),
                call_pos,
            ));
        }
    };

    let new_client = Client::builder()
        .timeout(std::time::Duration::from_secs(seconds))
        .build()
        .unwrap();

    let mut client = HTTP_CLIENT.lock().unwrap();
    *client = new_client;

    RuntimeResult::new().success(Value::Null)
}

pub fn set_user_agent(args: Vec<Value>, call_pos: Position) -> RuntimeResult {
    if args.len() != 1 {
        return RuntimeResult::new().failure(
            Error::new(
                call_pos.clone(),
                call_pos,
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
                call_pos.clone(),
                call_pos,
            ));
        }
    };

    let mut user_agent = USER_AGENT.lock().unwrap();
    *user_agent = agent;

    RuntimeResult::new().success(Value::Null)
}

// Helper to convert Json to Value (needed for xenith_to_serde_json)
fn json_to_value(json: &Json) -> Value {
    match json {
        Json::Null => Value::Null,
        Json::Bool(b) => Value::Bool(*b),
        Json::Number(n) => Value::Number(Number::new(*n)),
        Json::String(s) => Value::String(XenithString::new(s.clone())),
        Json::Array(arr) => {
            let elements: Vec<Value> = arr.iter().map(json_to_value).collect();
            Value::List(List::new(elements))
        }
        Json::Object(obj) => {
            let mut map = Map::new();
            for (k, v) in obj {
                map.set(k.clone(), json_to_value(v));
            }
            Value::Map(map)
        }
    }
}
