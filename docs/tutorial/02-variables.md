# Variables

## Declaring

Every variable is introduced with `let`:

```xenith
let name: string = "Ada"
let age: int = 36
```

The part after the colon is the type. You can leave it out when the value makes
the type obvious:

```xenith
let name = "Ada"      # string
let age = 36          # int
let ratio = 1.5       # float
let active = true     # bool
```

Both forms produce exactly the same thing. Writing the type down does two jobs:
it documents the intent, and it gets checked. If the value does not match, you
find out at that line:

```xenith
let age: int = "36"
```

```
error XEN001: Type Mismatch
  expected `int`, found `string`
```

Use the annotation when the type is not obvious from the right hand side, or
when you want the checker holding you to it. Leave it off for the cases where it
would just be noise.

## Reassigning

Assignment without `let` updates an existing variable:

```xenith
let counter: int = 0
counter = 10
counter = counter + 5

echo("{counter}")
```

```
15
```

The `let` is what makes a new variable. A bare `name = value` never creates one.
If nothing by that name has been declared, it is an error rather than a new
variable appearing quietly:

```xenith
countr = 1
```

```
error XEN002: Undefined Variable
  `countr` is not declared
```

That rule exists so a typo in an assignment cannot silently become a second
variable that nobody reads.

Reassignment is type checked against the declared type:

```xenith
let count: int = 1
count = "two"
```

```
error XEN001: Type Mismatch
  expected `int`, found `string`
```

## Constants

Put `const` in front of `let` for a binding that cannot be reassigned:

```xenith
const let MAX_RETRIES: int = 3
const let APP_NAME: string = "xenith"
```

Trying to change one is an error:

```xenith
const let MAX: int = 100
MAX = 200
```

```
error XEN018: Constant Reassignment
  cannot reassign constant `MAX`
```

The convention is SCREAMING_SNAKE_CASE for constants and snake_case for
everything else. Nothing enforces that, but the editor colours them differently
if you follow it.

## Scope

A block introduces a scope. Variables declared inside it disappear at the closing
brace:

```xenith
let outer: int = 1

when true {
    let inner: int = 2
    echo("inside: {outer} {inner}")
}

echo("outside: {outer}")
```

```
inside: 1 2
outside: 1
```

Referring to `inner` after the block is an XEN002.

Assignment reaches outward. Writing to a name from inside a block updates the
variable where it was declared, it does not make a local copy:

```xenith
let total: int = 0

for n in [1, 2, 3] {
    total = total + n
}

echo("{total}")
```

```
6
```

This is the behaviour you want for accumulators, and it is why the declaration
rule above matters. `total = total + n` updates the outer `total`; if it had said
`let total = total + n` it would have made a fresh one inside the loop and thrown
it away each iteration.

## Shadowing

Declaring the same name again inside an inner scope hides the outer one for the
rest of that block:

```xenith
let value: int = 1

when true {
    let value: int = 99
    echo("inner: {value}")
}

echo("outer: {value}")
```

```
inner: 99
outer: 1
```

Use it sparingly. It is easy to write by accident when you meant to assign.

## Naming

Identifiers are letters, digits and underscores, and cannot start with a digit.
Names are case sensitive, so `total` and `Total` are different variables.

The keywords cannot be used as names. There are not many of them:

```
let const method struct type
when or otherwise for while in skip stop release panic
grab from export as
int float string bool null list map
true false echo format
```

Next: [Numbers](03-numbers.md)
