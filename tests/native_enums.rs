use std::path::Path;
use xenith::native::syntax::parse;

#[test]
fn parses_native_enum_declarations_and_unit_variants() {
    let program = parse(
        Path::new("src/main.xen"),
        "enum Color { Red, Green } fn main() -> i32 { return 0 }",
    )
    .expect("enum source should parse");
    assert_eq!(program.enums.len(), 1);
    assert_eq!(program.enums[0].name, "Color");
    assert_eq!(program.enums[0].variants.len(), 2);
}
