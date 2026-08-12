# The Static Checker

`src/checker.rs`. Walks the tree between parsing and execution and reports type
errors before any code runs.

Two things it buys over the checks scattered through the interpreter: an error in
a branch that never executes is still reported, and every error in the file is
reported at once rather than the first one.

## Interface

```rust
pub fn check(ast: &Node, aliases: &HashMap<String, Type>) -> Vec<Error>
```

Called from four places:

- `lib.rs::run`, which returns the first error, so embedding `run` stays a
  simple `Result`.
- `lib.rs::check_source`, which returns all of them. This is what `main.rs` uses
  so the CLI can print every error and a count.
- `bin/xenith-lsp.rs`, so the editor reports exactly what the command line will.
- `ModuleRegistry::load_module`, so an imported file is checked before it runs.

The module case matters more than it sounds. Without it an exported method could
return the wrong type and every importing program would use the result without a
word, which is exactly where the guarantee is worth most. A module is checked
when it is loaded rather than when the importing file is checked, so the error
arrives at the `grab` rather than alongside the importing file's own errors.

## The rule it lives by

**A reported error must be a real one.** Missing a mistake is acceptable;
inventing one is not, because the first false positive makes the whole pass
something people want turned off.

That is enforced through `Type::Unknown`. Anything the checker cannot work out
becomes `Unknown`, and `compatible` returns true whenever either side is
`Unknown`:

```rust
fn compatible(&self, expected: &Type, actual: &Type) -> bool {
    if matches!(expected, Type::Unknown) || matches!(actual, Type::Unknown) {
        return true;
    }
    ...
}
```

So an unresolved name poisons its expression into silence rather than into a
complaint.

## Why it still does not report unresolved names

Name resolution is lexical, so the checker's view now matches the interpreter's,
and it could in principle report a name that is not in scope.

It does not yet, for one reason: a capture is live, so a method may legitimately
use a name declared later in the file.

```xenith
method report() -> null {
    echo("later is {later}")   # not declared yet, and that is fine
    release null
}

let later: int = 7
report()
```

Reporting undefined names correctly means knowing whether the declaration will
have run by the time the call does, which is a reachability question rather than
a scoping one. Until that is worked out the checker resolves what it can see and
stays quiet about the rest, which is why an unresolved name still yields
`Type::Unknown` rather than an error.

The interpreter reports those at run time as XEN002, so nothing is missed; it is
reported later than it could be.

## What it checks

| Check | Notes |
| --- | --- |
| Declaration against its annotation | `let n: int = "five"` |
| Reassignment against the declared type | needs the binding to be visible |
| Assignment to a `const let` | XEN018 |
| Arithmetic operands | `1 + 2.0`, `"a" - 1` |
| Comparison operands | both sides must be the same type |
| Call arity | XEN015 and XEN016 |
| Call argument types | for methods declared in the file |
| Return type against every `release` | including the `=>` short form |
| Struct literals | missing, unknown and wrongly typed fields |
| Field access | a field the struct does not declare |
| Index types | `list[string]`, `map[int]` |
| Interpolated expressions | everything above, inside a `{}` |

## What it does not check

- **Builtins.** `len`, `append` and the rest accept several argument types in
  ways `Type` cannot describe, so they are `Unknown` and their calls pass.
- **Names not yet declared at the point the checker reaches them**, as above.
- **Method values.** Calling a method held in a variable is not checked against
  its `method(A) -> B` type.

## Keeping it in step with the interpreter

`infer_arithmetic` must stay a **subset** of what `Value`'s operators accept in
`src/values.rs`. Rejecting something the interpreter would have run is the one
failure this pass must not have.

That is not hypothetical. The first version rejected `"=" * 50`, not knowing that
`Value::multiply` implements string repetition, and it broke a working sample the
moment it was switched on. If you add an operand pair to `values.rs`, add it here
too.

The reverse direction is safe: the checker may accept things the interpreter
rejects, because the interpreter still runs its own checks.

## Interpolated strings

The checker can see inside `"{...}"` only because those expressions are parsed at
parse time. `InterpolationPart` carries an optional parsed `Node`:

```rust
pub struct InterpolationPart {
    pub is_expression: bool,
    pub content: String,
    pub parsed: Option<Box<Node>>,
}
```

They used to be kept as source text and re-lexed and re-parsed on every
evaluation, which hid them from the checker and cost a parse per loop iteration.
Parsing them once made a 300,000 iteration loop that prints one about five times
faster.

`Lexer::new_at` numbers those tokens from the enclosing string's position, so an
error inside a `{}` points at the right line instead of line 0 of a source called
`<interpolated>`. Columns still drift for an expression on a later line of a
multi-line string, because the extracted text has lost the original layout.

`parsed` is `None` when the text does not parse, and the interpreter falls back
to its old path so the error surfaces the way it always did.

## Adding a check

1. Find or add the arm in `visit` (statements) or `infer` (expressions).
2. Ask whether every type involved is known. If any is `Unknown`, return without
   reporting.
3. Use a constructor from `src/error.rs` so the error gets a code and a help
   line, and matches the wording the interpreter uses for the same mistake.
4. Add a fixture under `tests/errors/` with the expected code.
5. Run the whole corpus and confirm nothing that used to work now fails:

```sh
cargo test
for f in testies/*.xen tests/cases/*.xen; do xenith "$f" > /dev/null || echo "BROKE $f"; done
```

Step 5 is not optional. A false positive is the failure mode that matters.

Next: [Values](07-values.md)
