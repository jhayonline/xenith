# Lists

A list holds any number of values of the same type. The type is written
`list<T>`, where `T` is the element type.

```xenith
let numbers: list<int> = [1, 2, 3]
let names: list<string> = ["Ada", "Alan"]
let empty: list<int> = []

echo("{ret(numbers)}")
echo("{ret(names)}")
echo("{ret(empty)}")
```

```
[1, 2, 3]
[Ada, Alan]
[]
```

A literal can spread over several lines, and a trailing comma is allowed:

```xenith
let languages: list<string> = [
    "Rust",
    "Go",
    "C",
]

echo("{ret(languages)}")
```

```
[Rust, Go, C]
```

## Reading an element

Index with square brackets. Indexes start at 0.

```xenith
let names: list<string> = ["Ada", "Alan", "Grace"]

echo(names[0])
echo(names[2])
```

```
Ada
Grace
```

Going past the end is an error, not a null:

```xenith
let names: list<string> = ["Ada"]
echo(names[5])
```

```
error XEN004: Index Out of Bounds
```

The last element is at `len() - 1`:

```xenith
let names: list<string> = ["Ada", "Alan", "Grace"]
echo(names[names.len() - 1])
```

```
Grace
```

## Writing an element

Assign through the index:

```xenith
let numbers: list<int> = [1, 2, 3]

numbers[0] = 99
echo("{ret(numbers)}")
```

```
[99, 2, 3]
```

Index assignment only replaces an existing position. It cannot extend the list;
use `.append()` for that.

## Length

```xenith
let numbers: list<int> = [1, 2, 3]

echo("{numbers.len()}")
echo("{len(numbers)}")
```

```
3
3
```

## Adding and removing

`.append()` puts a value on the end, changing the list in place:

```xenith
let queue: list<int> = [1, 2]

queue.append(3)
queue.append(4)

echo("{ret(queue)}")
```

```
[1, 2, 3, 4]
```

`.pop(index)` removes the element at an index and returns it:

```xenith
let queue: list<int> = [10, 20, 30]

let first: int = queue.pop(0)

echo("popped {first}")
echo("left {ret(queue)}")
```

```
popped 10
left [20, 30]
```

Both work from inside a loop, on a list declared outside it:

```xenith
let evens: list<int> = []

for (let i: int = 0; i < 10; i++) {
    when i % 2 == 0 {
        evens.append(i)
    }
}

echo("{ret(evens)}")
```

```
[0, 2, 4, 6, 8]
```

## Joining

`+` produces a new list and leaves both sides alone:

```xenith
let a: list<int> = [1, 2]
let b: list<int> = [3, 4]

let joined: list<int> = a + b

echo("{ret(joined)}")
echo("{ret(a)}")
```

```
[1, 2, 3, 4]
[1, 2]
```

## The non-mutating versions

`append`, `extend` and `pop` also exist as free functions. They return a new list
and leave the original untouched, which is the opposite of the method forms:

```xenith
let base: list<int> = [1, 2]

let grown: list<int> = append(base, 3)
let merged: list<int> = extend(base, [8, 9])

echo("grown  {ret(grown)}")
echo("merged {ret(merged)}")
echo("base   {ret(base)}")
```

```
grown  [1, 2, 3]
merged [1, 2, 8, 9]
base   [1, 2]
```

Method form changes the list. Function form makes a new one. Pick by whether you
want the original to survive.

## Nesting

A list of lists is written `list<list<T>>` and indexes chain:

```xenith
let grid: list<list<int>> = [
    [1, 2, 3],
    [4, 5, 6]
]

echo("{grid[1][2]}")

grid[0][0] = 99
echo("{ret(grid[0])}")
```

```
6
[99, 2, 3]
```

## Printing a list

`echo` on a list prints its elements. `ret` turns any value into a string, which
is what you want inside interpolation:

```xenith
let numbers: list<int> = [1, 2, 3]

echo("the list is {ret(numbers)}")
```

```
the list is [1, 2, 3]
```

One thing to watch: `ret` on a single element list drops the brackets.
`ret([5])` gives `5`, not `[5]`.

## Element types are checked

A `list<int>` holds ints:

```xenith
let numbers: list<int> = [1, 2, "three"]
```

```
error XEN001: Type Mismatch
```

Next: [Maps](09-maps.md)
