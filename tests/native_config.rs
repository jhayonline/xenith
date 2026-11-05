use std::{fs, process::Command};
use xenith::native::{check::check, codegen::{emit_object, link_host}};

#[test]
fn std_config_loads_typed_values_with_environment_overrides() {
    let directory = tempfile::tempdir().unwrap();
    let config_path = directory.path().join("app.env");
    fs::write(&config_path, "XENITH_NATIVE_CONFIG_HOST=\"file-host\"\nPORT = 8080\nENABLED=false\nREQUIRED=yes\n").unwrap();
    unsafe { std::env::set_var("XENITH_NATIVE_CONFIG_HOST", "env-host"); }
    let source = format!(r#"
grab {{ load_config, config_string, config_i32, config_bool, required_config, close_config }} from "std::config"
function main() -> i32 {{
    let config: i64 = load_config("{}")
    let host: string = config_string(config, "XENITH_NATIVE_CONFIG_HOST", "localhost")
    let port: i32 = config_i32(config, "PORT", 80)
    let enabled: bool = config_bool(config, "ENABLED", true)
    let required_value: string = required_config(config, "REQUIRED")
    close_config(config)
    if (host == "env-host" && port == 8080 && !enabled && required_value == "yes") {{ return 0 }}
    return 1
}}
"#, config_path.display());
    let main = directory.path().join("main.xen");
    fs::write(&main, source).unwrap();
    let program = xenith::native::project::load_program(&main).unwrap().0;
    let typed = check(&program).unwrap();
    let object = directory.path().join("main.o");
    let executable = directory.path().join("main");
    emit_object(&typed, &object).unwrap();
    link_host(&object, &executable).unwrap();
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(0));
    unsafe { std::env::remove_var("XENITH_NATIVE_CONFIG_HOST"); }
}

#[test]
fn std_config_required_missing_value_is_recoverable() {
    let directory = tempfile::tempdir().unwrap();
    let config_path = directory.path().join("app.env");
    fs::write(&config_path, "PORT=8080\n").unwrap();
    let source = format!(r#"
grab {{ load_config, required_config, close_config }} from "std::config"
function main() -> i32 {{
    let config: i64 = load_config("{}")
    try {{
        let value: string = required_config(config, "MISSING")
        close_config(config)
        return 1
    }} catch (error) {{
        close_config(config)
        if (error.code == "XEN300" && len(error.message) > 0) {{ return 0 }}
        return 1
    }}
}}
"#, config_path.display());
    let main = directory.path().join("main.xen");
    fs::write(&main, source).unwrap();
    let program = xenith::native::project::load_program(&main).unwrap().0;
    let typed = check(&program).unwrap();
    let object = directory.path().join("main.o");
    let executable = directory.path().join("main");
    emit_object(&typed, &object).unwrap();
    link_host(&object, &executable).unwrap();
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(0));
}

#[test]
fn std_config_invalid_integer_is_recoverable() {
    let directory = tempfile::tempdir().unwrap();
    let config_path = directory.path().join("app.env");
    fs::write(&config_path, "PORT=not-a-number\n").unwrap();
    let source = format!(r#"
grab {{ load_config, config_i32, close_config }} from "std::config"
function main() -> i32 {{
    let config: i64 = load_config("{}")
    try {{
        let port: i32 = config_i32(config, "PORT", 80)
        close_config(config)
        return 1
    }} catch (error) {{
        close_config(config)
        if (error.code == "XEN300") {{ return 0 }}
        return 1
    }}
}}
"#, config_path.display());
    let main = directory.path().join("main.xen");
    fs::write(&main, source).unwrap();
    let program = xenith::native::project::load_program(&main).unwrap().0;
    let typed = check(&program).unwrap();
    let object = directory.path().join("main.o");
    let executable = directory.path().join("main");
    emit_object(&typed, &object).unwrap();
    link_host(&object, &executable).unwrap();
    assert_eq!(Command::new(executable).status().unwrap().code(), Some(0));
}
