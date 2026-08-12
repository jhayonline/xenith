# Loops

There are three loop keywords and four loop shapes.

## The counting loop

The C form: initialiser, condition, step, separated by semicolons.

```xenith
for (let i: int = 0; i < 5; i++) {
    echo("{i}")
}
```

```
0
1
2
3
4
```

Each of the three parts is optional. Leaving all of them out loops forever,
which is only useful with a `stop` inside:

```xenith
let n: int = 0

for (;;) {
    n = n + 1
    when n > 3 {
        stop
    }
}

echo("{n}")
```

```
4
```

Counting down works the way you expect:

```xenith
for (let n: int = 3; n > 0; n--) {
    echo("{n}")
}
echo("liftoff")
```

```
3
2
1
liftoff
```

The step can be anything, not just `++`:

```xenith
for (let i: int = 0; i < 20; i += 5) {
    echo("{i}")
}
```

```
0
5
10
15
```

## Iterating a list

`for item in collection` when you want the elements and not the positions:

```xenith
let names: list<string> = ["Ada", "Alan", "Grace"]

for name in names {
    echo("hello {name}")
}
```

```
hello Ada
hello Alan
hello Grace
```

If you need the index too, use the counting form:

```xenith
let names: list<string> = ["Ada", "Alan", "Grace"]

for (let i: int = 0; i < names.len(); i++) {
    echo("{i}: {names[i]}")
}
```

```
0: Ada
1: Alan
2: Grace
```

## Iterating a map

Name two variables and you get the key and the value:

```xenith
let ages: map<string, int> = {"ada": 36, "alan": 41, "grace": 45}

for name, age in ages {
    echo("{name} is {age}")
}
```

```
ada is 36
alan is 41
grace is 45
```

Maps iterate in sorted key order, so the output of a program that walks a map is
the same on every run. Name one variable instead of two and you get the keys:

```xenith
let ages: map<string, int> = {"ada": 36, "alan": 41}

for name in ages {
    echo("{name}")
}
```

```
ada
alan
```

## while

When the end condition is not a count:

```xenith
let remaining: int = 100
let halvings: int = 0

while remaining > 1 {
    remaining = remaining / 2
    halvings = halvings + 1
}

echo("{halvings} halvings, {remaining} left")
```

```
6 halvings, 1 left
```

## skip and stop

`skip` jumps to the next iteration. `stop` leaves the loop entirely. They work in
all four shapes.

```xenith
for (let n: int = 1; n <= 10; n++) {
    when n % 2 == 0 {
        skip
    }
    when n > 7 {
        stop
    }
    echo("{n}")
}
```

```
1
3
5
7
```

In a counting loop, `skip` still runs the step, so it cannot cause an accidental
infinite loop the way a hand written `continue` before an increment can.

## Nesting

Loops nest with no special syntax. `skip` and `stop` apply to the nearest
enclosing loop.

```xenith
let grid: list<list<int>> = [[1, 2], [3, 4], [5, 6]]

for row in grid {
    for cell in row {
        echo("{cell}")
    }
}
```

```
1
2
3
4
5
6
```

## Building a list in a loop

`.append()` changes the list in place, including from inside a loop body:

```xenith
let squares: list<int> = []

for (let i: int = 1; i <= 5; i++) {
    squares.append(i * i)
}

echo("{ret(squares)}")
```

```
[1, 4, 9, 16, 25]
```

## Loop bodies have their own scope

A `let` inside a loop body is local to the iteration:

```xenith
for (let i: int = 0; i < 3; i++) {
    let doubled: int = i * 2
    echo("{doubled}")
}
```

```
0
2
4
```

`doubled` does not exist after the loop, and each iteration starts with a clean
body scope. The loop variable `i` in the counting form lives in the loop's own
scope, so it is not visible afterwards either.

Assignment to something declared outside still reaches out, which is what makes
accumulators work:

```xenith
let total: int = 0

for n in [1, 2, 3, 4] {
    total = total + n
}

echo("{total}")
```

```
10
```

Next: [Lists](08-lists.md)
