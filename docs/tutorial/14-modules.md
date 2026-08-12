# Modules

A module is a `.xen` file. Anything marked `export` can be used by another file;
everything else stays private to it.

## Exporting

Put `export` in front of a method or a `let`:

```xenith
# greeting.xen

export let LANGUAGE: string = "en"

export method greet(name: string) -> string {
    release "Hello, {name}"
}

export method shout(name: string) -> string {
    release "HELLO, {name}"
}
```

Definitions without `export` are not visible outside the file.

## Importing

`grab` names from a file. The path is relative to the importing file and the
`.xen` extension is left off:

```xenith
grab { greet, LANGUAGE } from "greeting"

echo(greet("Ada"))
echo("language: {LANGUAGE}")
```

```
Hello, Ada
language: en
```

Import several names in one set of braces, or write several `grab` lines.

## Renaming an import

`as` gives an imported name a different local name, which is how you deal with
two modules that both export a `parse`:

```xenith
grab { greet as hello } from "greeting"

echo(hello("Ada"))
```

```
Hello, Ada
```

## Importing everything

`grab * as name` brings the whole module in under one name. The namespace is a
map from export name to value, so you reach into it with brackets:

```xenith
grab * as g from "greeting"

echo(g["greet"]("Ada"))
echo(g["LANGUAGE"])
```

```
Hello, Ada
en
```

Named imports read better. Use the namespace form when you want it obvious where
something came from, or when a module has many exports.

## Where files are looked up

A module path is resolved relative to the directory of the file doing the
importing. Subdirectories use `::`:

```xenith
grab { greet } from "greeting"
grab { area } from "geometry::shapes"
```

The second line looks for `geometry/shapes.xen` next to the importing file.

A module that cannot be found is an error at the `grab`:

```xenith
grab { thing } from "nowhere"
```

```
error XEN012: Module Not Found
  Module 'nowhere' not found
```

So is asking for a name the module does not export:

```xenith
grab { private_helper } from "greeting"
```

```
error XEN012: Module Not Found
  'private_helper' is not exported from module 'greeting'
```

## Modules run when imported

Importing a module executes its top level. A module should therefore contain
definitions and little else. Anything at the top level that prints or does work
will happen on import, once, the first time any file imports it. Modules are
cached, so importing the same one twice does not run it twice.

## Two limitations worth knowing up front

**A module's exports cannot call its own private helpers.** Names are resolved
against the scope of the caller, not the file where the code was written. Once an
importing file calls an exported method, the module's own unexported definitions
are out of scope:

```xenith
# stats.xen

method double(n: int) -> int => n * 2

export method double_all(values: list<int>) -> list<int> {
    let out: list<int> = []
    for value in values {
        out.append(double(value))     # `double` is not visible here
    }
    release out
}
```

Calling `double_all` from another file gives `error XEN002: Undefined Variable`
for `double`. Until name resolution becomes lexical, write each export so it
stands on its own, or export the helper too.

**`export struct` is not supported.** Only methods and `let` bindings can be
exported. A struct needed in two files has to be declared in both.

## A worked example

`textstats.xen`:

```xenith
export let VERSION: string = "1.0"

export method total(values: list<int>) -> int {
    let sum: int = 0
    for value in values {
        sum = sum + value
    }
    release sum
}

export method largest(values: list<int>) -> int {
    when values.len() == 0 {
        release 0
    }
    let best: int = values[0]
    for value in values {
        when value > best {
            best = value
        }
    }
    release best
}
```

`report.xen`, in the same directory:

```xenith
grab { total, largest, VERSION } from "textstats"

let durations: list<int> = [143, 62, 28, 9]

echo("textstats v{VERSION}")
echo("total   {total(durations)}")
echo("slowest {largest(durations)}")
```

```
textstats v1.0
total   242
slowest 143
```

Next: [Built in functions](15-builtins.md)
