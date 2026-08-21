# Values

`src/values.rs`, 1218 lines. What a Xenith expression evaluates to, plus the
arithmetic and the built in functions.

## The enum

```rust
pub enum Value {
    Int(i64),
    Float(f64),
    String(Rc<XenithString>),
    Bytes(Rc<Bytes>),
    Bool(bool),
    Null,
    List(List),
    Map(Box<Map>),
    Tuple(Rc<Vec<Value>>),
    Struct(Box<Struct>),
    Enum(Box<EnumValue>),
    Function(Box<Function>),
    BuiltInFunction(BuiltInFunction),
}
```

16 bytes, and `tests/layout.rs` keeps it there. Every payload wider than a word
is behind a pointer, which leaves `i64`/`f64` as the widest thing stored inline;
the discriminant then fits in a niche and costs nothing. See
`docs/internals/10-performance.md`.

## Number is a sum type

A `Value` does not store one. It holds an `i64` or an `f64` directly -- wrapping
them in a `Number` cost a second tag and made the payload 16 bytes to carry 8
bytes of data. `Number` remains the type the arithmetic helpers speak, built on
demand by `Value::as_number`, which is free because it is `Copy`.

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Number {
    Int(i64),
    Float(f64),
}
```

This is the centre of the language's number semantics. `Number` was once a struct
wrapping an `f64`, with `int` and `float` as annotations that did not change how
a value was stored. That is why `9007199254740993` used to print as
`...992`.

Helpers on it: `is_int`, `is_zero`, `to_f64`, `to_i64`, `as_index`,
`type_name`, and `math_pi`.

The `Display` impl prints a float with `{:.1}` when its fractional part is zero,
so `1.0` prints as `1.0` rather than `1` and stays distinguishable from an int.

## Arithmetic

Every operation matches on the pair and refuses mixed operands:

```rust
pub fn add(&self, other: &Value) -> Result<Value, Error> {
    match (self, other) {
        (Value::Int(x), Value::Int(y)) => x
            .checked_add(*y)
            .map(Value::int)
            .ok_or_else(|| Self::overflow_err("addition")),
        (Value::Float(x), Value::Float(y)) => Ok(Value::float(x + y)),
        (Value::Int(_), Value::Float(_)) | (Value::Float(_), Value::Int(_)) => Err(
            Self::mixed_err("add", &self.as_number().unwrap(), &other.as_number().unwrap()),
        ),
        (Value::String(a), Value::String(b)) => { /* concatenate */ }
        (Value::List(a), Value::List(b)) => { /* concatenate */ }
        _ => Err(Self::arith_err(...)),
    }
}
```

Two rules hold throughout:

- Integer operations use `checked_*` and turn `None` into XEN017. Nothing wraps
  silently.
- `Int` with `Float` is an error, never a promotion.

The int and float cases are separate top-level arms rather than a nested match,
which is the shape the typed opcodes want: one arm per operand pair is one
opcode each.

Comparisons return `Value::Bool` through the `eq_value` and `compare` helpers.
They once returned `Number(1.0)` and `Number(0.0)`.

## Values are copied

Assignment, argument passing and reading from a symbol table all clone. There is
no reference or pointer type.

That is what forces the write back pattern: a method like `.append()` cannot
mutate the caller's list, so `call_method` returns the new list and `visit_call`
stores it back into the variable the receiver came from. It is also why
`assign_into` unwinds through nested containers on `grid[1][2] = 9`.

### Copied, but not deep copied

That is the semantics. The representation is copy-on-write, because taking it
literally made every collection quadratic to build:

```rust
String(Rc<XenithString>),
List  -> elements: Rc<Vec<Value>>
Map   -> pairs:    Rc<HashMap<String, Value>>
```

Cloning any of them is a refcount bump. A write goes through `Rc::make_mut`
(`List::elements_mut`, `Map::pairs_mut`), which copies only when somebody else
is holding the same data, so no holder ever sees another's change. Strings need
no `make_mut` at all: nothing modifies one in place, `a + b` builds a new one.

Cheap clones are not enough on their own. `xs.append(x)` reads `xs`, changes it
and writes it back, and while it was reading it the symbol table still held the
same elements — two holders, so `make_mut` copied, and appending stayed O(n).
`SymbolTable::take` fixes that: for `append`, `pop` and `remove` on a plain
variable, `visit_call` lifts the value out of the binding and leaves `Null`, so
the mutation has the only reference. `assign_into` does the same for
`m[key] = value`. Both evaluate the arguments and the index *before* lifting,
because `xs.append(xs.len())` has to see `xs` rather than the hole.

Anything deeper than a plain variable, such as `record.items.append(v)`, still
takes the copying path. It is correct either way and the fast path stays legible.

Measured on the release build, filling a collection an element at a time:

| | before | after |
| --- | --- | --- |
| 8000 × `xs.append(i)` | 3.99s | 0.02s |
| 8000 × `m[key] = i` | 8.70s | 0.03s |

### Strings know their own length

`XenithString` carries a character count and an all-ASCII flag, both settled at
construction. Xenith counts and indexes strings by character while a `String`
holds UTF-8, so `len()` walked the whole string and `text[i]` walked as far as
`i`. Every scanner in the standard library is written `while i < text.len()`,
which made all of them quadratic before they had done anything. On ASCII, which
is the case that matters, indexing and `substring` are now byte ranges.

## Function

```rust
pub struct Function {
    pub name: Option<String>,
    pub body_node: Rc<Node>,
    pub arg_names: Rc<Vec<String>>,
    pub param_types: Rc<Vec<Type>>,
    pub should_auto_return: bool,
    /// The scope this method was written in.
    pub closure: Rc<Context>,
}
```

Every field that could be large is behind an `Rc`. `body_node` used to be a
`Box<Node>`, and since `SymbolTable::get` returns values by clone, every
reference to a function deep copied its entire body tree. That single `Box`
dominated runtime on any recursive program; most of the time was in `malloc` and
`free`.

If you add a field here, ask whether cloning it is cheap. This struct is cloned
on every function lookup.

`Function::execute`:

1. Compares argument count, raising XEN015 or XEN016.
2. Checks each argument against its declared type.
3. Checks `context.depth_exceeded()` and raises XEN019.
4. Creates a child of `self.closure` and binds the parameters, taking `depth`
   from the caller so the recursion guard still counts calls.
5. Visits the body.

Step 4 is where lexical scoping comes from; see
[The interpreter](05-interpreter.md).

## Collections

```rust
pub struct Bytes { pub data: Vec<u8> }
pub struct List { pub elements: Vec<Value> }
pub struct Map { pub pairs: HashMap<String, Value> }
pub struct Struct { pub name: String, pub fields: HashMap<String, Value> }
pub struct EnumValue {
    pub enum_name: String,
    pub variant: String,
    pub payload: Vec<Value>,
}
```

`EnumValue` carries its own enum's name for the same reason `Struct` does: it is
what `value_matches_type` compares against a `Type::Struct(name, _)`, with no
table to consult. Enums have no `Type` variant of their own -- `Type::Struct` is
the named user type and covers both, because a type annotation is written the
same way for either and the parser cannot tell an imported name apart anyway.

`Bytes` is held inline rather than boxed. A `Vec<u8>` is 24 bytes, the same as
the `String` inside `XenithString` and the `Vec<Value>` inside `List`, so it
does not widen `Value` the way `Map` and `Struct` did.

`Map` uses a plain `HashMap`, so it has no order of its own. `keys()`, `values()`
and `items()` all sort by key before returning, and `visit_for` sorts before
iterating, so every way of walking a map agrees and gives the same answer on
every run. Iterating the `HashMap` directly used to shuffle a program's output
between runs.

If maps ever need insertion order, an order preserving map type is the change,
and all four call sites can then drop their sorts.

## Built in functions

```rust
pub struct BuiltInFunction { pub name: String }
```

Just a name. `execute` dispatches on it:

```rust
match self.name.as_str() {
    "echo" => self.echo(args, call_pos),
    "len" => self.len(args, call_pos),
    // ...
}
```

The list of names lives in `src/builtins/registry.rs`, which the interpreter
reads when building the global scope and the language server reads for
completion and hover. One list means the editor cannot offer a builtin that no
longer exists.

Adding a builtin is two edits: an entry in `BUILTIN_FUNCTIONS` and an arm in
`BuiltInFunction::execute`. The registry entry alone will register the name and
then fail at the call.

## Type checking helpers

`Value::value_matches_type(value, expected)` answers whether a runtime value
inhabits a declared type. For `list<T>` it checks every element, which makes it
O(n) and worth remembering before calling it in a loop.

`Value::get_type_name(value)` gives the name for a diagnostic. Use it rather than
`{:?}` on a `Type`, which prints `Int` where a reader wants `int`.

Next: [Errors and diagnostics](08-errors.md)
