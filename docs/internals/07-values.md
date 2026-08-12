# Values

`src/values.rs`, 1218 lines. What a Xenith expression evaluates to, plus the
arithmetic and the built in functions.

## The enum

```rust
pub enum Value {
    Number(Number),
    String(XenithString),
    Bool(bool),
    Null,
    List(List),
    Map(Map),
    Tuple(Vec<Value>),
    Struct(Struct),
    Function(Function),
    BuiltInFunction(BuiltInFunction),
}
```

## Number is a sum type

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
        (Value::Number(a), Value::Number(b)) => match (a, b) {
            (Number::Int(x), Number::Int(y)) => x
                .checked_add(*y)
                .map(Value::int)
                .ok_or_else(|| Self::overflow_err("addition")),
            (Number::Float(x), Number::Float(y)) => Ok(Value::float(x + y)),
            _ => Err(Self::mixed_err("add", a, b)),
        },
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

Comparisons return `Value::Bool` through the `eq_value` and `compare` helpers.
They once returned `Number(1.0)` and `Number(0.0)`.

## Values are copied

Assignment, argument passing and reading from a symbol table all clone. There is
no reference or pointer type.

That is what forces the write back pattern: a method like `.append()` cannot
mutate the caller's list, so `call_method` returns the new list and `visit_call`
stores it back into the variable the receiver came from. It is also why
`assign_into` unwinds through nested containers on `grid[1][2] = 9`.

## Function

```rust
pub struct Function {
    pub name: Option<String>,
    pub body_node: Rc<Node>,
    pub arg_names: Rc<Vec<String>>,
    pub param_types: Rc<Vec<Type>>,
    pub should_auto_return: bool,
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
4. Creates a child context of the caller's and binds the parameters.
5. Visits the body.

Step 4 is where dynamic scoping comes from; see
[The interpreter](05-interpreter.md).

## Collections

```rust
pub struct List { pub elements: Vec<Value> }
pub struct Map { pub pairs: HashMap<String, Value> }
pub struct Struct { pub name: String, pub fields: HashMap<String, Value> }
```

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
