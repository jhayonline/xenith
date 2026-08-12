# Errors

Xenith has no exceptions and no `try` or `catch`. An error stops the program,
prints where it happened and why, and exits with a non zero status.

That is a design choice, not a gap waiting to be filled. Recoverable failure is
expressed by returning a value that says what happened; unrecoverable failure
stops the program.

## Reading a diagnostic

Every error looks like this:

```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
error XEN001: Type Mismatch
  expected `int`, found `string`
  → program.xen:3:1

     3 │ let count: int = "many"
         ^^^^^^^^^^^^^^^^^^^^^^^
  note: cannot assign `string` to variable of type `int`
  💡 use type conversion: `value as int`
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

The parts, in order: the code and category, what went wrong, the file with line
and column, the offending source with the span underlined, a note explaining the
rule, and a line suggesting a fix. The last two appear where they are useful,
which is most of the time.

## Error codes

### Type and value errors

| Code | Name | When |
| --- | --- | --- |
| XEN001 | Type Mismatch | a value does not match the type expected of it |
| XEN002 | Undefined Variable | a name was used or assigned before being declared |
| XEN003 | Division by Zero | the right side of `/` was zero |
| XEN004 | Index Out of Bounds | a list index was past the end, or negative |
| XEN009 | Field Not Found | a struct has no such field, or a literal named an unknown one |
| XEN009 | Missing Field | a struct literal left a declared field out |
| XEN011 | Invalid Type Conversion | `as` could not convert the value |
| XEN015 | Too Many Arguments | a call passed more arguments than the method takes |
| XEN016 | Too Few Arguments | a call passed fewer |
| XEN017 | Integer Overflow | an int operation or literal fell outside the 64 bit range |
| XEN018 | Constant Reassignment | something declared `const let` was assigned to |
| XEN019 | Recursion Limit | calls nested more than 10000 deep |
| XEN020 | Destructuring Mismatch | the pattern and the tuple have different sizes |

### Module errors

| Code | Name | When |
| --- | --- | --- |
| XEN012 | Module Not Found | the file could not be found, or did not export that name |
| XEN021 | Circular Import | two modules import each other |

### Syntax errors

| Code | Name | When |
| --- | --- | --- |
| XEN013 | Unexpected Token | the parser found something it could not use here |
| XEN100 | Illegal Character | a character that is not part of the language |
| XEN101 | Expected Character | something was left unclosed |
| XEN102 | Invalid Syntax | the shape of a construct is wrong |

### Everything else

| Code | Name | When |
| --- | --- | --- |
| XEN200 | Runtime Error | an error with no more specific code |
| XEN300 | Panic | the program called `panic` |

## panic

`panic` stops the program with a message. Use it for a condition that means the
program cannot sensibly carry on.

```xenith
let config_loaded: bool = false

when !config_loaded {
    panic("configuration missing, cannot continue")
}

echo("never reached")
```

```
error XEN300: Panic
  configuration missing, cannot continue
```

Nothing catches a panic. If a caller should be able to handle the situation, do
not panic; return something it can act on.

## Handling failure without exceptions

Return a value that carries the outcome. A tuple of the result and a flag is the
usual shape:

```xenith
method safe_divide(a: int, b: int) -> (int, bool) {
    when b == 0 {
        release (0, false)
    }
    release (a / b, true)
}

let (result, ok) = safe_divide(10, 0)

when ok {
    echo("{result}")
} otherwise {
    echo("cannot divide by zero")
}
```

```
cannot divide by zero
```

The caller cannot ignore the flag, because destructuring makes them name it. That
is most of what exception handling buys you, without the invisible control flow.

## Check before you act

Several operations are errors rather than returning a null, so guard them:

```xenith
let ages: map<string, int> = {"ada": 36}
let names: list<string> = ["Ada"]

when ages.has_key("alan") {
    echo("{ages["alan"]}")
} otherwise {
    echo("alan is not in the map")
}

when names.len() > 3 {
    echo(names[3])
} otherwise {
    echo("no fourth name")
}
```

```
alan is not in the map
no fourth name
```

## When errors are reported

Syntax and type errors are reported before the program starts, all of them at
once:

```
error XEN001: Type Mismatch
...
error XEN015: Too Many Arguments
...
3 errors found, nothing was run
```

An error in a branch that never executes is still reported.

Imported modules are checked too, when they are loaded. An error inside one is
reported at its own line in its own file, with a note saying which module it came
from:

```
error XEN001: Type Mismatch
  expected `int`, found `string`
  → textstats.xen:4:5
  note: from module 'textstats'
```

What the checker cannot prove ahead of time is caught as it runs, which stops the
program at that point. [Known limitations](18-limitations.md) has the detail.

The language server runs the same checks, so the editor shows what the command
line will.

## Exit status

A program that finishes exits 0. A program stopped by any error exits 1, so shell
scripts and CI can test for it:

```sh
xenith program.xen && echo "worked"
```

Next: [Editor setup](17-editor-setup.md)
