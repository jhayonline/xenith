use xenith::program;

#[test]
fn the_graph_lists_dependencies_before_dependents() {
    let source = "grab { trim } from \"std::string\"\n\
                  method main() -> int {\n\
                  \x20   echo(trim(\"  x  \"))\n\
                  \x20   release 0\n\
                  }\n";

    let graph = program::build("<test>", source).expect("should build");

    let paths: Vec<&str> = graph.modules.iter().map(|m| m.path.as_str()).collect();

    let std_string = paths
        .iter()
        .position(|p| *p == "std::string")
        .expect("std::string should be in the graph");
    let root = paths
        .iter()
        .position(|p| *p == "<test>")
        .expect("the root should be in the graph");

    assert!(std_string < root, "a dependency must come first: {paths:?}");
}

#[test]
fn a_module_appears_once_however_often_it_is_grabbed() {
    let source = "grab { trim } from \"std::string\"\n\
                  grab { pad_left } from \"std::string\"\n\
                  method main() -> int => 0\n";

    let graph = program::build("<test>", source).expect("should build");

    let count = graph
        .modules
        .iter()
        .filter(|m| m.path == "std::string")
        .count();
    assert_eq!(count, 1);
}

#[test]
fn a_cycle_is_reported_rather_than_looped_on() {
    // tests/errors/circular_import.xen is the end-to-end version; this is the
    // unit that proves the walk terminates rather than the interpreter
    // catching it later.
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/errors/circular_import.xen");
    let source = std::fs::read_to_string(&root).expect("fixture should exist");

    let result = program::build(root.to_str().unwrap(), &source);
    assert!(result.is_err(), "a cycle must not build");
}

#[test]
fn a_call_into_a_module_is_checked_before_anything_runs() {
    // The interpreter would catch this at the call. What matters here is that
    // the graph refuses to build, which means it was caught by the checker
    // with the imported signature in hand -- before a statement executed.
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/errors/imported_wrong_type.xen");
    let source = std::fs::read_to_string(&root).expect("fixture should exist");

    let result = program::build(root.to_str().unwrap(), &source);
    assert!(
        result.is_err(),
        "passing a string to `double(n: int)` should not check"
    );
}

#[test]
fn a_correct_call_into_a_module_still_checks() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/modules/typed_main.xen");
    let source = std::fs::read_to_string(&root).expect("fixture should exist");

    assert!(program::build(root.to_str().unwrap(), &source).is_ok());
}

#[test]
fn a_call_into_the_standard_library_is_checked_by_type() {
    // The reason the whole graph is walked first: without the module's real
    // signature in hand, `trim` is Unknown and this call is unchecked.
    let source = "grab { trim } from \"std::string\"\n\
                  method main() -> int {\n\
                  \x20   echo(trim(42))\n\
                  \x20   release 0\n\
                  }\n";

    assert!(
        program::build("<test>", source).is_err(),
        "passing an int to `trim(text: string)` should not check"
    );
}
