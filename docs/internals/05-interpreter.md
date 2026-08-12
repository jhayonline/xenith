# The Interpreter

`src/interpreter.rs`, 2211 lines. Walks the AST and produces values and effects.

## The dispatch

```rust
pub fn visit(&mut self, node: &Node, context: &mut Context) -> RuntimeResult {
    match node {
        Node::Number(n) => self.visit_number(n, context),
        Node::BinaryOperator(n) => self.visit_binary_op(n, context),
        // ... one arm per variant
    }
}
```

Every `visit_*` takes the node and the current context and returns a
`RuntimeResult`. Recursion on the Rust stack is what makes the tree walk work,
and what bounds Xenith's own recursion depth.

## RuntimeResult

`src/runtime_result.rs`. Carries a value, an optional error, and the loop and
return signals.

```rust
pub struct RuntimeResult {
    pub value: Option<Value>,
    pub error: Option<Box<Error>>,
    pub func_return_value: Option<Value>,
    pub loop_should_continue: bool,
    pub loop_should_break: bool,
}
```

The usual shape in a visitor is:

```rust
let value = result.register(self.visit(&node.child, context));
if result.should_return() {
    return result;
}
```

`register` folds a sub result in and copies any error or signal up.
`should_return` is true when an error happened, a `release` fired, or a `skip` or
`stop` is unwinding.

`error` is boxed. Inline it was 392 bytes, and this struct is returned by value
from every single visit.

## Context and scope

`src/context.rs`. A context is one scope plus a link to its parent.

```rust
pub struct Context {
    pub display_name: String,
    pub parent: Option<Rc<Context>>,
    pub parent_entry_position: Option<Position>,
    pub symbol_table: Rc<SymbolTable>,
    pub exports: HashMap<String, Value>,
    pub depth: usize,
}
```

`parent` is `Rc`, not `Box`. It was `Box` once, which meant entering a scope
deep copied the entire chain and made calls quadratic in depth.

`depth` counts how far down the chain we are, and `depth_exceeded()` compares it
against `MAX_CALL_DEPTH`, which is 10,000. That check is what turns runaway
recursion into an XEN019 instead of a stack overflow.

A child context is created by:

```rust
let mut child = context.create_child("<for>", position);
```

Which constructs are scoped:

| Construct | Own scope |
| --- | --- |
| method body | yes |
| `for (;;)` body | yes, one scope reused and cleared each iteration |
| `for x in xs` body | yes |
| `while` body | yes, reused and cleared |
| `when` / `otherwise` body | yes |
| program top level | the global scope |

Loop bodies reuse a single child context and call `symbol_table.clear_local()`
each iteration rather than allocating a fresh one, which keeps a hot loop from
allocating a symbol table per pass.

## The symbol table

`src/symbol_table.rs`. Names to values, with a parent link mirroring the context
chain.

```rust
pub struct SymbolTable {
    symbols: Rc<RefCell<FxHashMap<String, Value>>>,
    types: Rc<RefCell<FxHashMap<String, Type>>>,
    constants: Rc<RefCell<FxHashSet<String>>>,
    parent: Option<Rc<SymbolTable>>,
}
```

Three maps: the values, the declared types for checking reassignment, and the
set of names declared `const let`.

The important methods:

| Method | Does |
| --- | --- |
| `get(name)` | walk out through parents, return a clone |
| `set(name, v)` | write to this scope only, used by declarations |
| `assign_existing(name, v)` | walk out to the scope that declared it |
| `resolve_for_assign(name)` | one walk returning constness and declared type |
| `clear_local()` | drop everything in this scope, keep parents |

`resolve_for_assign` exists because checking "declared?", "constant?" and
"declared type?" as three separate calls walked the chain three times on every
assignment.

`get` returns a clone of the value. That is the single most important fact about
this file, and the reason `Function` holds its body in an `Rc`; see
[Values](07-values.md).

`FxHashMap` is a `HashMap` with the hasher from `src/fxhash.rs`, a small
non cryptographic hash lifted from rustc's design. Variable lookup is hot enough
that SipHash showed up in profiles.

## Name resolution is dynamic

When a method is called, its body executes in a child of the *caller's* context:

```rust
func.execute(args, context.clone(), self, call_position)
```

`context` there is the caller's. The context where the method was defined is not
recorded anywhere.

Two consequences:

1. A method can read and write its caller's variables.
2. A method cannot capture anything from where it was written, so there are no
   closures, and a module's exported method cannot see that module's private
   helpers.

Making this lexical means storing the defining context in `Function` and using it
in `execute`. It is a contained change with a wide blast radius, since it alters
what existing programs do.

## Assignment

`visit_var_assign` splits on `is_declaration`:

- **Declaration**: check the value against the annotation, then `set` in the
  current scope, and record the name in `constants` if it was `const let`.
- **Assignment**: `resolve_for_assign` to find the binding, refuse if it is
  constant, check the value against the declared type, then `assign_existing`.

Assigning to a name that resolves to nothing is XEN002 rather than a new
variable.

`assign_into` handles the compound targets, recursing through the target
expression:

```rust
fn assign_into(&mut self, target: &Node, value: Value, context: &mut Context)
    -> Option<Error>
```

For `grid[1][2] = 9` it evaluates `grid[1]`, sets index 2 in that list, then
recurses to store the rebuilt inner list back into `grid[1]`, then again to store
`grid` back into the variable. Values are copied rather than shared, so an
in place mutation is not possible and this unwinding is what makes nested
assignment work at any depth.

## Method calls

`visit_call` has two paths.

If the callee is a `MethodAccess`, it is `value.method(args)`: evaluate the
receiver, evaluate the arguments, and dispatch in `call_method`.

```rust
fn call_method(&mut self, object: Value, method_name: &str, args: Vec<Value>,
               context: &mut Context) -> (RuntimeResult, Option<Value>)
```

The second element of that tuple is the receiver's new state for the methods that
mutate, `append` and `pop`. `visit_call` writes it back with `assign_existing`,
so the change lands in the scope that declared the variable rather than the
current one. Writing to the current scope instead meant `xs.append(v)` inside a
loop body built up a shadow copy and left the original list empty.

Otherwise the callee is evaluated normally and must be a `Function` or a
`BuiltInFunction`.

## Structs

`visit_struct_def` records the declared fields in
`struct_defs: HashMap<String, Vec<(String, Type)>>`.

`visit_struct_instantiation` checks a literal against that record: every field
must be declared, every declared field must be given a value, and each value must
match its field's type. Without those checks a literal could invent fields, omit
them, or hold anything at all, and the mistake surfaced much later as a confusing
read.

## Where type checking happens

The interpreter checks as it runs, at the moment each operation is reached:

- `visit_var_assign` checks declarations and reassignments
- `Function::execute` checks argument count and types
- `visit_struct_instantiation` checks struct literals
- `Value::add` and friends refuse mixed `int` and `float`

[The static checker](06-checker.md) runs before any of this and reports what it
can prove ahead of time. It does not replace these; it is conservative and gives
up on anything it cannot type, so these are what make the guarantee complete.

Next: [The static checker](06-checker.md)
