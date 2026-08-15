# Known Limitations

Xenith is young. This page is the honest list of what does not work yet, so you
find out here rather than halfway through writing something.

Each entry says what happens, why, and what to do instead.

## Built in functions are not type checked

`len`, `append`, `is_num` and the rest accept more than one type of argument in
ways the type system cannot yet describe, so calls to them are not checked.
Calls to methods you write are checked, by both count and type.

## Imported names are checked as they run, not before

The static pass does not follow `grab`, so it knows nothing about what a module
exports. A call to an imported method is not checked for argument count or type,
and a literal of an imported struct is not checked for its fields.

They are all still checked, with the same errors and the same messages, when the
line runs. What you lose is finding out before any output appears, and finding
out about a branch that never executes.

## Patterns cover less than Rust's

`match` handles `_`, bindings, literals, tuples, `Enum::Variant(...)`,
`A | B` and `when` guards. It does not have struct patterns
(`Point { x, y }`), list patterns (`[first, ..rest]`), ranges, or `@` bindings.

**What to do:** match the variant, then read fields with `p.x`. See
[Enums and match](13-enums.md).

## No generics, so no Result or Option

An enum's payload types are concrete, so `Result<T>` and `Option<T>` cannot be
written. A concrete enum per case works: `enum Lookup { Found(int), Missing }`.

This is the largest thing still missing, and it is the same decision that
collections are waiting on.

## A struct or enum cannot be renamed on import

`grab { Point } from "shapes"` works. `grab { Point as Coordinate }` is refused,
because a struct is identified by its name and a renamed one would not match the
methods that take it. Methods and `let` bindings can still be renamed.

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

## A keyword cannot be a struct field name

`from`, `in`, `as` and the rest are reserved everywhere, including inside a
struct literal, so `Line { from: a, to: b }` does not parse.

**What to do:** name the field something else. `head` and `tail` in that case.

## No hex or binary number literals

`255` is the only way to write it; `0xff` and `0b1111_1111` do not parse. This
shows up most when working with `bytes`.

**What to do:** write it in decimal, or go through
[`std::bytes`](20-standard-library.md) `from_hex` for a run of them.

## An expression cannot span lines

A newline ends a statement, so a long condition has to stay on one line:

```xenith
# Does not parse.
release a == "1" || a == "true"
    || a == "yes"
```

**What to do:** put it on one line, or name the pieces with `let` first.

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

## The standard library is small

`std::string`, `std::math`, `std::fs`, `std::bytes` and `std::env` exist; see
[The standard library](20-standard-library.md). There is no networking, no JSON,
no time and no random.

Collections are the interesting gap. Without generics there is no way to write
one `map` or `filter` that works for a `list<int>` and a `list<string>` both, so
higher order collection functions are waiting on that decision. Everything else
is just not written yet.

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
  value. See [Errors](17-errors.md).
- **No methods on structs.** Structs are data; behaviour is free methods that
  take them. See [Structs](12-structs.md).
- **No inheritance or interfaces.**

This page used to say pattern matching was not planned, on the grounds that a
chain of `or when` covered it. That was wrong, and the reason it was wrong is
worth keeping: `match` on its own really is only a tidier `or when`. It is
[enums](13-enums.md) that make it worth having, and completeness checking is not
something a chain of conditions can give you.

Next: [The standard library](20-standard-library.md)
