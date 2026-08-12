# Known Limitations

Xenith is young. This page is the honest list of what does not work yet, so you
find out here rather than halfway through writing something.

Each entry says what happens, why, and what to do instead.

## The checker cannot see through the caller's scope

A static pass runs before your program and reports the type errors it can prove.
The one thing it cannot reason about is a name that a method reads from whoever
called it, rather than from a parameter, because that value has no type until the
call happens:

```xenith
method show() -> null {
    echo("{count + 1}")
    release null
}

let count: string = "not a number"
show()
```

```
error XEN001: Type Mismatch
  cannot add string and int
```

The error still arrives, but at run time rather than before it, and only if that
line executes.

**What to do:** pass what a method needs as an argument. Parameters have declared
types, so everything computed from them is checked ahead of time.

## Built in functions are not type checked

`len`, `append`, `is_num` and the rest accept more than one type of argument in
ways the type system cannot yet describe, so calls to them are not checked.
Calls to methods you write are checked, by both count and type.

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

## The language server's symbols are file local

It reports syntax and type errors as you type. Its symbol handling, though, is by
name and within one file, so rename will rewrite every `i` in the file, and
definitions in other files are not followed.

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
