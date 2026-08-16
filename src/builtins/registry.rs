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
    // The string primitives. Everything else about strings is meant to be
    // written in Xenith on top of these, so think hard before adding a fourth.
    BuiltinFn {
        name: "substring",
        signature: "substring(text, start, end) -> string",
        doc: "The characters from `start` up to but not including `end`. Clamped to the ends of the string, so it never fails.",
    },
    BuiltinFn {
        name: "code_at",
        signature: "code_at(text, index) -> int",
        doc: "The Unicode code point of the character at an index, for classifying and converting case.",
    },
    BuiltinFn {
        name: "from_code",
        signature: "from_code(code) -> string",
        doc: "The one character string for a Unicode code point. The inverse of `code_at`.",
    },
    // The bytes primitives.
    //
    // Deliberately few, for the same reason as the string ones: `len`, `+`,
    // indexing and `as` already cover most of what byte handling needs, and
    // everything above these is written in Xenith in `std::bytes`. Converting
    // to a string is here rather than only as `as string` because `as` stops
    // the program on invalid UTF-8, and reading bytes off a socket or a file is
    // exactly where a caller wants to handle that rather than die.
    BuiltinFn {
        name: "bytes_slice",
        signature: "bytes_slice(raw, start, end) -> bytes",
        doc: "The bytes from `start` up to but not including `end`. Clamped to the ends, so it never fails.",
    },
    BuiltinFn {
        name: "bytes_to_string",
        signature: "bytes_to_string(raw) -> (string, string)",
        doc: "The bytes decoded as UTF-8 text, and an error which is empty on success. `raw as string` is the form that stops the program instead.",
    },
    BuiltinFn {
        name: "bytes_to_list",
        signature: "bytes_to_list(raw) -> list<int>",
        doc: "Every byte as an int in 0 to 255.",
    },
    BuiltinFn {
        name: "bytes_from_list",
        signature: "bytes_from_list(codes) -> (bytes, string)",
        doc: "Bytes built from a list of ints, and an error which is empty on success. A value outside 0 to 255 is an error.",
    },
    // The float primitives. `sqrt` is not among them because `x ^ 0.5` already
    // is one, and these are here because a series expansion written in Xenith
    // would give wrong answers away from zero, not because Rust is faster.
    // Each takes a float and returns a float.
    BuiltinFn {
        name: "sin",
        signature: "sin(x) -> float",
        doc: "Sine of an angle in radians.",
    },
    BuiltinFn {
        name: "cos",
        signature: "cos(x) -> float",
        doc: "Cosine of an angle in radians.",
    },
    BuiltinFn {
        name: "tan",
        signature: "tan(x) -> float",
        doc: "Tangent of an angle in radians.",
    },
    BuiltinFn {
        name: "atan2",
        signature: "atan2(y, x) -> float",
        doc: "The angle in radians from the x axis to the point (x, y), correct in all four quadrants.",
    },
    BuiltinFn {
        name: "log",
        signature: "log(x) -> float",
        doc: "Natural logarithm. Negative input gives a not-a-number result, and zero gives negative infinity.",
    },
    BuiltinFn {
        name: "log10",
        signature: "log10(x) -> float",
        doc: "Logarithm base ten. A primitive rather than `log(x) / log(10.0)`, which loses precision: that identity gives 2.9999999999999996 for 1000.",
    },
    BuiltinFn {
        name: "exp",
        signature: "exp(x) -> float",
        doc: "e raised to the power of x, the inverse of `log`.",
    },
    // The filesystem primitives.
    //
    // These carry an `fs_` prefix and are wrapped by `std::fs`, unlike
    // `substring` or `sin` which are global under their own names. The rule:
    // an operation on a built in type is global, a service is prefixed and
    // imported. Reading a file is something a program asks the world to do,
    // and it should be visible in the imports that it does.
    //
    // Anything that can fail reports it in a string that is empty when nothing
    // went wrong. A bool would say only that something failed, and for the
    // filesystem the reason is most of the information.
    BuiltinFn {
        name: "fs_read",
        signature: "fs_read(path) -> (string, string)",
        doc: "The whole file as text, and an error which is empty on success. Fails on a file that is not valid UTF-8.",
    },
    BuiltinFn {
        name: "fs_write",
        signature: "fs_write(path, contents) -> string",
        doc: "Writes text to a file, creating it or replacing what was there. Returns an error, empty on success.",
    },
    BuiltinFn {
        name: "fs_append",
        signature: "fs_append(path, contents) -> string",
        doc: "Adds text to the end of a file, creating it if needed. Returns an error, empty on success.",
    },
    BuiltinFn {
        name: "fs_read_bytes",
        signature: "fs_read_bytes(path) -> (bytes, string)",
        doc: "The whole file as raw bytes, and an error which is empty on success. Unlike `fs_read`, works on a file that is not text.",
    },
    BuiltinFn {
        name: "fs_write_bytes",
        signature: "fs_write_bytes(path, raw) -> string",
        doc: "Writes raw bytes to a file, creating it or replacing what was there. Returns an error, empty on success.",
    },
    BuiltinFn {
        name: "fs_append_bytes",
        signature: "fs_append_bytes(path, raw) -> string",
        doc: "Adds raw bytes to the end of a file, creating it if needed. Returns an error, empty on success.",
    },
    BuiltinFn {
        name: "fs_remove",
        signature: "fs_remove(path) -> string",
        doc: "Deletes a file. Returns an error, empty on success.",
    },
    BuiltinFn {
        name: "fs_exists",
        signature: "fs_exists(path) -> bool",
        doc: "Whether anything exists at that path, file or directory.",
    },
    BuiltinFn {
        name: "fs_is_file",
        signature: "fs_is_file(path) -> bool",
        doc: "Whether the path is a file.",
    },
    BuiltinFn {
        name: "fs_is_dir",
        signature: "fs_is_dir(path) -> bool",
        doc: "Whether the path is a directory.",
    },
    BuiltinFn {
        name: "fs_size",
        signature: "fs_size(path) -> (int, string)",
        doc: "The size of a file in bytes, and an error which is empty on success.",
    },
    BuiltinFn {
        name: "fs_list",
        signature: "fs_list(path) -> (list, string)",
        doc: "The names in a directory, sorted, without their leading path. And an error, empty on success.",
    },
    BuiltinFn {
        name: "fs_create_dir",
        signature: "fs_create_dir(path) -> string",
        doc: "Creates a directory and any missing parents. Succeeds if it is already there.",
    },
    BuiltinFn {
        name: "fs_remove_dir",
        signature: "fs_remove_dir(path) -> string",
        doc: "Deletes an empty directory. There is deliberately no recursive form: deleting a tree by accident from a script is not a mistake worth making convenient.",
    },
    // The process environment primitives, wrapped by `std::env`. Prefixed and
    // imported for the same reason as the filesystem: reading the environment
    // is something a program asks the world to do, and it should be visible in
    // the imports that it does.
    BuiltinFn {
        name: "env_get",
        signature: "env_get(name) -> (string, bool)",
        doc: "The value of an environment variable, and whether it was set. An unset variable and one set to the empty string are different things, which is why this is a pair.",
    },
    BuiltinFn {
        name: "env_set",
        signature: "env_set(name, value) -> null",
        doc: "Sets an environment variable for this process and anything it starts.",
    },
    BuiltinFn {
        name: "env_unset",
        signature: "env_unset(name) -> null",
        doc: "Removes an environment variable from this process.",
    },
    BuiltinFn {
        name: "env_vars",
        signature: "env_vars() -> map<string, string>",
        doc: "Every environment variable. A name whose value is not valid UTF-8 is left out.",
    },
    BuiltinFn {
        name: "env_args",
        signature: "env_args() -> list<string>",
        doc: "The command line, starting with the path of the program being run, then whatever followed it.",
    },
    BuiltinFn {
        name: "env_cwd",
        signature: "env_cwd() -> (string, string)",
        doc: "The working directory, and an error which is empty on success.",
    },
    BuiltinFn {
        name: "env_exit",
        signature: "env_exit(code) -> null",
        doc: "Stops the program immediately with an exit status. Nothing after it runs.",
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
pub const KEYWORDS_DECLARATION: &[&str] =
    &["let", "const", "method", "struct", "enum", "type"];

/// Control flow. `when`/`or when`/`otherwise` are Xenith's if/else-if/else.
pub const KEYWORDS_CONTROL: &[&str] = &[
    "when",
    "or",
    "otherwise",
    "match",
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
    "int", "float", "string", "bytes", "bool", "null", "list", "map",
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
        "enum" => "Defines a type that is one of several named variants, each able to carry values. `enum Shape { Circle(float), Empty }`",
        "match" => "Chooses a branch by pattern. An expression, and checked for completeness when the cases are known.",
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
