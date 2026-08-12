//! # Built-in Registry
//!
//! The single list of names the language predefines, shared by the interpreter
//! (which installs them into the global scope) and the language server (which
//! offers them for completion and hover). Keeping one list means the editor
//! can never advertise a builtin that was renamed or deleted.

/// A predefined function: name, signature, one-line doc.
pub struct BuiltinFn {
    pub name: &'static str,
    pub signature: &'static str,
    pub doc: &'static str,
}

/// A predefined constant: name, type, one-line doc.
pub struct BuiltinConst {
    pub name: &'static str,
    pub type_name: &'static str,
    pub doc: &'static str,
}

/// Every function available without an import.
///
/// Adding an entry here is not enough to make it callable -- it must also be
/// dispatched in `BuiltInFunction::execute`.
pub const BUILTIN_FUNCTIONS: &[BuiltinFn] = &[
    BuiltinFn {
        name: "echo",
        signature: "echo(value) -> null",
        doc: "Writes a value to standard output, followed by a newline.",
    },
    BuiltinFn {
        name: "format",
        signature: "format(text) -> string",
        doc: "Applies `{}` interpolation to a string that did not get it already, which is what makes a backtick raw string useful. Evaluated in the scope it is called from.",
    },
    BuiltinFn {
        name: "ret",
        signature: "ret(value) -> string",
        doc: "Renders a value as a string without printing it.",
    },
    BuiltinFn {
        name: "input",
        signature: "input() -> string",
        doc: "Reads one line from standard input.",
    },
    BuiltinFn {
        name: "input_int",
        signature: "input_int() -> int",
        doc: "Reads one line from standard input and parses it as an int, re-prompting until it parses.",
    },
    BuiltinFn {
        name: "clear",
        signature: "clear() -> null",
        doc: "Clears the terminal screen.",
    },
    BuiltinFn {
        name: "len",
        signature: "len(value) -> int",
        doc: "Number of elements in a list or map, or characters in a string.",
    },
    BuiltinFn {
        name: "append",
        signature: "append(list, value) -> null",
        doc: "Adds a value to the end of a list, in place.",
    },
    BuiltinFn {
        name: "pop",
        signature: "pop(list, index) -> value",
        doc: "Removes and returns the element at an index.",
    },
    BuiltinFn {
        name: "extend",
        signature: "extend(list, other) -> null",
        doc: "Appends every element of `other` onto `list`, in place.",
    },
    BuiltinFn {
        name: "is_num",
        signature: "is_num(value) -> bool",
        doc: "True when the value is an int or a float.",
    },
    BuiltinFn {
        name: "is_str",
        signature: "is_str(value) -> bool",
        doc: "True when the value is a string.",
    },
    BuiltinFn {
        name: "is_list",
        signature: "is_list(value) -> bool",
        doc: "True when the value is a list.",
    },
    BuiltinFn {
        name: "is_fun",
        signature: "is_fun(value) -> bool",
        doc: "True when the value is a function.",
    },
    BuiltinFn {
        name: "run",
        signature: "run(path) -> value",
        doc: "Executes another Xenith source file.",
    },
];

/// Every constant available without an import.
pub const BUILTIN_CONSTANTS: &[BuiltinConst] = &[
    BuiltinConst {
        name: "TRUE",
        type_name: "bool",
        doc: "The boolean true. `true` is the usual spelling.",
    },
    BuiltinConst {
        name: "FALSE",
        type_name: "bool",
        doc: "The boolean false. `false` is the usual spelling.",
    },
    BuiltinConst {
        name: "NULL",
        type_name: "null",
        doc: "The null value. `null` is the usual spelling.",
    },
    BuiltinConst {
        name: "MATH_PI",
        type_name: "float",
        doc: "The ratio of a circle's circumference to its diameter.",
    },
];

/// Reserved words, grouped so editors can colour them by role.
pub const KEYWORDS_DECLARATION: &[&str] = &["let", "const", "method", "struct", "type"];

/// Control flow. `when`/`or when`/`otherwise` are Xenith's if/else-if/else.
pub const KEYWORDS_CONTROL: &[&str] = &[
    "when",
    "or",
    "otherwise",
    "for",
    "while",
    "in",
    "skip",
    "stop",
    "release",
    "panic",
];

/// Modules and conversions.
pub const KEYWORDS_MODULE: &[&str] = &["grab", "from", "export", "as"];

/// Type names usable in annotations.
pub const TYPE_NAMES: &[&str] = &[
    "int", "float", "string", "bool", "null", "list", "map",
];

/// Literals that lex as their own token kinds.
pub const LITERALS: &[&str] = &["true", "false", "null"];

/// One-line explanation for each keyword, for hover and completion detail.
pub fn keyword_doc(keyword: &str) -> Option<&'static str> {
    Some(match keyword {
        "let" => "Declares a variable in the current scope. `const let` makes it immutable.",
        "const" => "Used with `let` to declare an immutable binding.",
        "method" => "Defines a function. `method name(a: int) -> int { ... }`",
        "struct" => "Defines a plain data type with named, typed fields.",
        "type" => "Declares an alias for an existing type.",
        "when" => "Conditional branch. Pairs with `or when` and `otherwise`.",
        "or" => "Begins an `or when` branch, Xenith's else-if.",
        "otherwise" => "The fallback branch of a `when` chain.",
        "for" => "`for (init; cond; step)` counts; `for item in xs` iterates.",
        "while" => "Repeats a block while its condition holds.",
        "in" => "Separates the loop variable from the collection in a `for ... in` loop.",
        "skip" => "Advances to the next iteration of the enclosing loop.",
        "stop" => "Exits the enclosing loop.",
        "release" => "Returns a value from the enclosing method.",
        "panic" => "Aborts the program with a message.",
        "grab" => "Imports names from another module.",
        "from" => "Names the module a `grab` imports from.",
        "export" => "Makes a definition visible to modules that import this one.",
        "as" => "Converts between types (`x as float`), or renames an import.",
        _ => return None,
    })
}
