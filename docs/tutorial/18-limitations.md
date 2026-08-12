# Known Limitations

Xenith is young. This page is the honest list of what does not work yet, so you
find out here rather than halfway through writing something.

Each entry says what happens, why, and what to do instead.

## Type checking happens at runtime

Types are checked when an operation is reached, not before the program starts. So
a type error in a branch that never executes is never reported, and a program can
produce output and then stop on a bad line further down.

```xenith
echo("this prints")
let broken: int = "not an int"
echo("this does not")
```

```
this prints
error XEN001: Type Mismatch
```

**What to do:** run your code to type check it. A static pass that walks the
whole program before executing is the main piece of work outstanding on the
language.

## Return types are not checked

A method declares a result type, and nothing verifies that what it releases
matches:

```xenith
method wrong() -> int {
    release "not an int"
}

echo(wrong())
```

```
not an int
```

**What to do:** treat the return type as documentation until the static checker
lands.

## Methods cannot capture their surroundings

Names inside a method body are resolved against the scope of whoever called it,
not the scope where the method was written. There are no closures:

```xenith
type IntFn = method(int) -> int

method make_adder(n: int) -> IntFn {
    release method(x: int) -> int => x + n
}

let add_ten: IntFn = make_adder(10)
echo("{add_ten(5)}")
```

```
error XEN002: Undefined Variable
  `n` is not defined
```

The same rule means a module's exported method cannot see that module's private
helpers once an importing file calls it.

**What to do:** pass everything a method needs as an argument, and write module
exports so each one stands alone.

## format cannot be used as an expression

`format` is a keyword to the lexer rather than an identifier, so it cannot appear
inside a larger expression. Both of these fail to parse:

```xenith
let s: string = format("{} and {}", 1, 2)
```

```
error XEN013: Unexpected Token
```

**What to do:** use string interpolation, which does everything `format` would:

```xenith
echo("{1} and {2}")
```

```
1 and 2
```

## export struct is not supported

Only methods and `let` bindings can be exported from a module. A struct needed in
two files has to be declared in both.

## Chain keywords must follow the closing brace

`or when` and `otherwise` have to be on the same line as the `}` before them.
Starting one on a new line does not parse.

```xenith
when false {
    echo("a")
} otherwise {
    echo("b")
}
```

```
b
```

## No way to remove a map key

Maps can be read, updated and added to, but there is no delete.

**What to do:** build a new map without the key.

## Conditions accept non-booleans

`when` and `while` treat `0`, `""`, an empty list, an empty map and `null` as
false. This is looser than the rest of the language, where an `int` where a
`bool` belongs is a type error.

**What to do:** write the comparison out. `when count > 0` rather than
`when count`.

## ret drops the brackets on a one element list

```xenith
echo(ret([5]))
echo(ret([5, 6]))
```

```
5
[5, 6]
```

## Strings are minimal

No indexing, no slicing, no `split`, `trim`, `upper`, `replace` or `contains`.
The only operations are `+`, `.len()` and comparison.

**What to do:** these belong in a standard library, which has not been written
yet. The old one was removed because it was a thin wrapper over Rust rather than
a designed library.

## No standard library

There is no file system, no networking, no JSON, no time, no random, no maths
beyond the operators and `MATH_PI`. The [builtins](15-builtins.md) are the whole
surface.

This is deliberate for now. The language is being settled first.

## The language server does not report type errors

It reports syntax errors as you type. Type errors wait until you run the program,
because the parser does not type check and there is no separate checking pass
yet.

Its symbol handling is also by name and file local, so rename will rewrite every
`i` in the file, and definitions in other files are not followed.

## Recursion is bounded by the Rust stack

The interpreter recurses as it evaluates, so Xenith recursion depth is limited by
the host stack. The limit is 10000 calls and hitting it gives a clean XEN019
rather than a crash, but a deeply recursive algorithm may need rewriting as a
loop.

## Things that are not planned

Some absences are decisions rather than gaps:

- **No exceptions.** Errors stop the program; recoverable failure is a returned
  value. See [Errors](16-errors.md).
- **No pattern matching.** A chain of `or when` covers it, and leaving `match`
  out keeps the language small.
- **No methods on structs.** Structs are data; behaviour is free methods that
  take them. See [Structs](12-structs.md).
- **No generics.** Not currently planned, though a container type that is not
  `list` or `map` would force the question.
- **No inheritance or interfaces.**

Back to [the tutorial index](README.md)
