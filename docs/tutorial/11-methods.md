# Methods

A method is a named piece of code that takes typed arguments and returns a typed
result. Xenith calls them methods rather than functions; there is only the one
kind.

```xenith
method add(a: int, b: int) -> int {
    release a + b
}

echo("{add(2, 3)}")
```

```
5
```

Reading that signature left to right: the name, the parameters with their types,
`->` and the type of the result.

## release

`release` hands a value back and ends the method immediately:

```xenith
method classify(n: int) -> string {
    when n < 0 {
        release "negative"
    }
    when n == 0 {
        release "zero"
    }
    release "positive"
}

echo(classify(-5))
echo(classify(0))
echo(classify(7))
```

```
negative
zero
positive
```

Early `release` is the normal way to write a method with several cases. There is
no need to nest everything into one `otherwise` chain.

## Methods that return nothing

Give the result type as `null` and release `null`:

```xenith
method banner(text: string) -> null {
    echo("=== {text} ===")
    release null
}

banner("report")
```

```
=== report ===
```

## The short form

When the body is a single expression, `=>` replaces the braces:

```xenith
method square(n: int) -> int => n * n
method louder(text: string) -> string => text + "!"

echo("{square(9)}")
echo(louder("hey"))
```

```
81
hey!
```

Use it for one liners. Anything with a branch in it reads better as a block.

## Arguments are checked

Both the count and the types:

```xenith
method greet(name: string) -> string {
    release "hello " + name
}

echo(greet("Ada", "extra"))
```

```
error XEN015: Too Many Arguments
  expected `1` arguments, got `2`
```

```xenith
method double(n: int) -> int {
    release n * 2
}

echo("{double("five")}")
```

```
error XEN001: Type Mismatch
```

## Recursion

A method can call itself:

```xenith
method factorial(n: int) -> int {
    when n <= 1 {
        release 1
    }
    release n * factorial(n - 1)
}

echo("{factorial(10)}")
```

```
3628800
```

Recursion that never bottoms out stops with a clean error rather than crashing
the process:

```xenith
method forever(n: int) -> int {
    release forever(n + 1)
}

echo("{forever(1)}")
```

```
error XEN019: Recursion Limit
  call depth exceeded 10000 while calling `forever`
```

If you hit that with a correct algorithm, rewrite it as a loop.

## Methods are values

A method can be stored in a variable, passed to another method and returned from
one. The type of a method is written `method(A, B) -> C`, and giving that type a
name makes signatures readable:

```xenith
type IntFn = method(int) -> int

method square(n: int) -> int => n * n
method negate(n: int) -> int => -n

method apply_to_each(values: list<int>, fn: IntFn) -> list<int> {
    let out: list<int> = []
    for value in values {
        out.append(fn(value))
    }
    release out
}

echo("{ret(apply_to_each([1, 2, 3], square))}")
echo("{ret(apply_to_each([1, 2, 3], negate))}")
```

```
[1, 4, 9]
[-1, -2, -3]
```

That is how you write one routine and vary the behaviour, instead of writing the
same loop three times.

## Anonymous methods

Leave the name out to write a method inline:

```xenith
type IntFn = method(int) -> int

let triple: IntFn = method(n: int) -> int => n * 3

echo("{triple(7)}")
```

```
21
```

## Names are resolved at the call site

This is the one surprising thing about methods in Xenith, and it is worth
understanding before it surprises you.

A method body looks up names in the scope of whoever called it, not the scope
where the method was written. So a method can read and change a variable that
belongs to its caller:

```xenith
let counter: int = 0

method bump() -> null {
    counter = counter + 1
    release null
}

bump()
bump()

echo("{counter}")
```

```
2
```

The flip side is that a method cannot capture anything from where it was
defined. This does not work:

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

By the time `add_ten` runs, `make_adder` has returned and its `n` is gone. There
are no closures.

In practice: pass everything a method needs as an argument. That is good style
anyway, and here it is also the only thing that works. See
[Known limitations](18-limitations.md).

## Parameters are copies

Changing a parameter inside a method does not affect the caller's value:

```xenith
method try_to_change(n: int) -> null {
    n = 99
    release null
}

let value: int = 1
try_to_change(value)

echo("{value}")
```

```
1
```

Next: [Structs](12-structs.md)
