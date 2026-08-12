# Tuples

A tuple groups a fixed number of values that do not have to share a type. Where a
list is "many of the same thing", a tuple is "these specific things, in this
order".

```xenith
let point = (10, 20)
let person = ("Ada", 36)

echo("{ret(point)}")
echo("{ret(person)}")
```

```
(10, 20)
(Ada, 36)
```

The type is written as the element types in parentheses: `(int, int)`,
`(string, int)`.

## Destructuring

Pulling a tuple apart by position is the point of having them:

```xenith
let person = ("Ada", 36)

let (name, age) = person

echo("{name} is {age}")
```

```
Ada is 36
```

The number of names has to match the number of elements:

```xenith
let person = ("Ada", 36)
let (name, age, city) = person
```

```
error XEN020: Destructuring Mismatch
  Expected 3 elements, got 2
```

## Ignoring parts

`_` throws a value away:

```xenith
let person = ("Ada", 36, "London")

let (_, age, _) = person

echo("{age}")
```

```
36
```

Use it so a reader can see at a glance which parts you actually care about.

## Returning several values

This is where tuples earn their place. A method that has two answers returns
both:

```xenith
method divmod(a: int, b: int) -> (int, int) {
    release (a / b, a % b)
}

let (quotient, remainder) = divmod(17, 5)

echo("17 / 5 = {quotient} remainder {remainder}")
```

```
17 / 5 = 3 remainder 2
```

Naming the parts at the call site keeps the meaning where it is used, rather than
in a comment next to the return type.

A common shape is a value plus whether it was found:

```xenith
method first_even(values: list<int>) -> (int, bool) {
    for value in values {
        when value % 2 == 0 {
            release (value, true)
        }
    }
    release (0, false)
}

let (found, ok) = first_even([1, 3, 6, 7])

when ok {
    echo("found {found}")
} otherwise {
    echo("none")
}
```

```
found 6
```

## Nesting

Tuples hold tuples, and destructuring follows the same shape:

```xenith
let corners = ((0, 0), (4, 3))

let ((x1, y1), (x2, y2)) = corners

echo("({x1},{y1}) to ({x2},{y2})")
```

```
(0,0) to (4,3)
```

## Tuples or structs

A tuple is right when the grouping is local and obvious: a pair of coordinates, a
result and a flag, two things a method returns together.

Reach for a [struct](12-structs.md) once the parts need names to be understood,
or the same grouping shows up in several places. `Point { x: 1, y: 2 }` survives
someone reading it six months later in a way `(1, 2)` does not.

Next: [Methods](11-methods.md)
