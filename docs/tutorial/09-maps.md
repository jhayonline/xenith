# Maps

A map associates string keys with values. The type is `map<K, V>`. Keys are
always strings today; the key type is written out for symmetry and for when other
key types arrive.

```xenith
let ages: map<string, int> = {"ada": 36, "alan": 41}
let empty: map<string, int> = {}

echo("{ret(ages.items())}")
echo("{empty.len()}")
```

```
[[ada, 36], [alan, 41]]
0
```

Keys must be quoted. A literal can spread across lines, with an optional trailing
comma:

```xenith
let config: map<string, string> = {
    "host": "localhost",
    "port": "8080",
}

echo("{config["host"]}:{config["port"]}")
```

```
localhost:8080
```

## Reading

Index with the key:

```xenith
let ages: map<string, int> = {"ada": 36}

echo("{ages["ada"]}")
```

```
36
```

Reading a key that is not there is an error, not a null:

```xenith
let ages: map<string, int> = {"ada": 36}

echo("{ages["nobody"]}")
```

```
error XEN200: Runtime Error
  Key 'nobody' not found in map
```

So check first when you are not certain:

```xenith
let ages: map<string, int> = {"ada": 36}

when ages.has_key("ada") {
    echo("ada is {ages["ada"]}")
}

when !ages.has_key("nobody") {
    echo("nobody is not in the map")
}
```

```
ada is 36
nobody is not in the map
```

## Writing

Assigning to a key updates it if it is there and inserts it if it is not:

```xenith
let ages: map<string, int> = {"ada": 36}

ages["ada"] = 37
ages["grace"] = 45

echo("{ret(ages.items())}")
```

```
[[ada, 37], [grace, 45]]
```

This is how a map gets built up. Starting from an empty literal and filling it in
is the normal pattern:

```xenith
let counts: map<string, int> = {}

for word in ["red", "blue", "red"] {
    when counts.has_key(word) {
        counts[word] = counts[word] + 1
    } otherwise {
        counts[word] = 1
    }
}

echo("{ret(counts.items())}")
```

```
[[blue, 1], [red, 2]]
```

## Removing

`.remove(key)` takes a key out and hands back what was under it:

```xenith
let ages: map<string, int> = {"ada": 36, "alan": 41}

let was: int = ages.remove("ada")

echo("removed {was}, {ages.len()} left")
```

```
removed 36, 1 left
```

Removing a key that is not there is an error, the same as reading one:

```xenith
let ages: map<string, int> = {"ada": 36}

ages.remove("nobody")
```

```
error XEN200: Runtime Error
  Key 'nobody' not found in map
  💡 check with `has_key` before removing
```

So guard it when you are not certain, exactly as you would a read:

```xenith
let ages: map<string, int> = {"ada": 36}

when ages.has_key("nobody") {
    ages.remove("nobody")
}

echo("{ages.len()}")
```

```
1
```

## The map methods

| Method | Gives you |
| --- | --- |
| `.len()` | how many pairs |
| `.has_key(k)` | whether a key is present |
| `.remove(k)` | removes a key and gives back its value |
| `.keys()` | a list of the keys |
| `.values()` | a list of the values |
| `.items()` | a list of two element lists |

```xenith
let scores: map<string, int> = {"ada": 90, "alan": 85, "grace": 95}

echo("size   {scores.len()}")
echo("keys   {ret(scores.keys())}")
echo("values {ret(scores.values())}")
echo("items  {ret(scores.items())}")
```

```
size   3
keys   [ada, alan, grace]
values [90, 85, 95]
items  [[ada, 90], [alan, 85], [grace, 95]]
```

## Order

Maps are ordered by key, not by insertion. `.keys()`, `.values()`, `.items()` and
`for k, v in map` all walk the same sorted order, so a program that prints a map
produces the same bytes on every run.

```xenith
let m: map<string, int> = {"zebra": 1, "apple": 2, "mango": 3}

for key, value in m {
    echo("{key} = {value}")
}
```

```
apple = 2
mango = 3
zebra = 1
```

If you need insertion order, keep a separate `list<string>` of keys alongside the
map.

## Iterating

Two names gives you key and value, one name gives you just the key:

```xenith
let stock: map<string, int> = {"apples": 3, "pears": 0}

for item, count in stock {
    when count == 0 {
        echo("{item}: out of stock")
    } otherwise {
        echo("{item}: {count}")
    }
}
```

```
apples: 3
pears: out of stock
```

## Nesting

Values can be any type, including lists and other maps:

```xenith
let teams: map<string, list<string>> = {
    "red": ["ada", "grace"],
    "blue": ["alan"]
}

echo("{ret(teams["red"])}")
echo("{teams["red"][0]}")
```

```
[ada, grace]
ada
```

Next: [Tuples](10-tuples.md)
