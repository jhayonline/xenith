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
    entries: Rc<RefCell<Vec<Entry>>>,
    index: Rc<RefCell<FxHashMap<Rc<str>, u32>>>,
    parent: Option<Rc<SymbolTable>>,
}

pub struct Entry {
    pub name: Rc<str>,
    pub value: Value,
    pub declared_type: Option<Type>,
    pub is_constant: bool,
}
```

Bindings sit in a `Vec` with a map beside it from name to position. A binding's
value, declared type and constness live together in one entry, so an assignment
touches one structure rather than three separate maps.

The important methods:

| Method | Does |
| --- | --- |
| `get(name)` | walk out through parents, return a clone |
| `locate(name)` | the same, and say how many hops and which position |
| `get_slot(hops, slot, name)` | go straight there, verifying the name |
| `assign_slot(...)` | the same for a write, handing the value back on a miss |
| `set(name, v)` | write to this scope only, used by declarations |
| `assign_checked(name, v, ..)` | find, check constness and type, and store, in one walk |
| `clear_local()` | drop everything in this scope, keep parents |

Nothing is ever removed from a table, only overwritten or cleared wholesale.
That is what makes a position stable, and it is what the slot cache on each
variable reference depends on; see [Performance](10-performance.md).

`get` returns a clone of the value. That is the single most important fact about
this file, and the reason `Function` holds its body in an `Rc`; see
[Values](07-values.md).

`FxHashMap` is a `HashMap` with the hasher from `src/fxhash.rs`, a small
non cryptographic hash lifted from rustc's design. Variable lookup was hot enough
that SipHash showed up in profiles, before the slot cache took most lookups off
the hashed path entirely.

## Name resolution is lexical

A `Function` captures the context it was defined in, and a call runs the body
against a child of *that*, not of whoever called it:

```rust
pub struct Function {
    ...
    /// The scope this method was written in.
    pub closure: Rc<Context>,
}
```

```rust
let mut func_context = self.closure.create_child(name, call_position);

// Depth counts calls, not lexical nesting, so it comes from the caller.
func_context.depth = context.depth + 1;
```

That `depth` line matters. The recursion guard compares `depth` against
`MAX_CALL_DEPTH`, and a top level method's closure has a constant depth, so
taking it from the closure would leave the guard never firing and turn runaway
recursion back into a process abort.

The capture is an `Rc<Context>` rather than a copy, and `Context::clone` is O(1)
with the symbol tables shared behind it. So the capture is live: a method sees a
name declared after it, and a named method can see itself for recursion.

This resolution used to be dynamic, with the body running against the caller's
context. That meant no closures, and a module's exported method could not see
that module's private helpers.

### The cycle

A named method is stored in the scope it captured, so `Function` holds an `Rc` to
a context whose symbol table holds the function. `Rc` never frees a cycle, so
those contexts live until the process exits.

A `Weak` here would break the case closures exist for, where the method outlives
the scope that produced it. For a program that runs and exits this costs nothing;
for a long lived REPL session it accumulates slowly.

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
