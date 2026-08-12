# The Lexer

`src/lexer.rs`, 773 lines. Turns a string of characters into a `Vec<Token>`.

## Interface

```rust
let mut lexer = Lexer::new(file_name, source);
let tokens = lexer.make_tokens()?;
```

`make_tokens` returns `Result<Vec<Token>, IllegalCharError>`. It stops at the
first character it cannot use; there is no error recovery.

## State

```rust
pub struct Lexer {
    pub file_name: String,
    pub text: String,
    pub position: Position,
    pub current_character: Option<char>,
}
```

`advance()` moves one character forward and updates the position. Both use
`chars().nth(index)`, so `Position::index` counts characters rather than bytes.
This matters when converting to LSP positions, which count UTF-16 code units.

## Tokens

`TokenType` in `src/tokens.rs` lists every kind: literals, operators,
delimiters, the type keywords, and a catch all `Keyword`.

```rust
pub struct Token {
    pub kind: TokenType,
    pub value: Option<String>,
    pub position_start: Position,
    pub position_end: Position,
}
```

The `value` carries the text for identifiers, numbers, strings and keywords, and
is `None` for punctuation whose kind already says everything.

## Identifiers and keywords

`make_identifier` reads letters, digits and underscores, then decides what the
result is:

1. The built in type names get their own kinds: `int` becomes `TypeInt`, `list`
   becomes `TypeList`, and so on.
2. `true` and `false` become `BoolTrue` and `BoolFalse`.
3. Anything in the `KEYWORDS` list becomes `TokenType::Keyword` with the word as
   its value.
4. Everything else is `TokenType::Identifier`.

Note that `echo` is in `KEYWORDS`, because it has a form without parentheses
that the parser handles specially. Every other builtin is an ordinary
identifier, which is what lets it be called, assigned and passed around like any
other value.

`format` used to be in that list too, with no parser handling to justify it, and
the only effect was that it could not be used as an expression: the parser saw a
`Keyword` where it wanted a callable identifier. Do not add a name here unless
the parser genuinely treats it specially.

### The `or when` special case

`or` and `when` lex as one token. When `make_identifier` reads `or`, it looks
ahead past whitespace for `when`, and if it finds it, emits a single
`Keyword("or when")`.

```rust
if id_str == "or" {
    // peek ahead for `when`, and combine if found
}
```

This is why `or when` has to be on the same line as the closing brace before it:
the lookahead skips spaces, not newlines.

## Numbers

`make_number` reads digits and at most one dot. No dot gives `TokenType::Int`,
one dot gives `TokenType::Float`. The text is kept as a string and parsed later
by the interpreter, which is where the range check for `int` lives.

There is no support for hex, binary, underscores as digit separators, or
exponents, so `1.0e10` does not lex.

The sign is not part of the token. `factor()` in the parser folds a minus
directly in front of a number literal into it, which is what makes the most
negative int writable at all: `-9223372036854775808` as an operator applied to a
literal would overflow on the literal before the minus was ever reached.

## Strings

`make_string` handles three things at once: escapes, interpolation, and the
closing quote.

Escapes are looked up in a small map:

```rust
let escape_map: HashMap<char, char> = HashMap::from([
    ('n', '\n'), ('t', '\t'), ('r', '\r'),
    ('\\', '\\'), ('"', '"'), ('\'', '\''),
    ('{', '{'), ('}', '}'),
]);
```

An escape that is not in the map yields the character itself, so `\q` is `q`.

`{{` and `}}` produce a single literal brace.

### How interpolation is encoded

This is the least obvious part of the lexer.

A string containing `{...}` becomes a `TokenType::InterpolatedString` whose value
is a single packed string:

```
__INTERPOLATED__|text:Hello, |expr:name|text:!
```

Parts are separated by `|` and each part is `kind:content`, where kind is `text`
or `expr`. `InterpolatedStringNode::new` in `src/nodes.rs` unpacks it.

Because `|` is the separator, a `|` inside a part has to be escaped or it would
be read as a boundary. `escape_interpolation_part` replaces `\` with `\\` and `|`
with `\p` on the way in, and `unescape_interpolation_part` reverses it on the way
out. Without that, `"{a || b}"` was silently truncated to `a`, since the encoded
form split at the first pipe.

Packing the parts into a string is not a good design, and it is worth replacing
with a proper list on the token. What the encoding no longer costs is a parse per
evaluation: `InterpolatedStringNode::new` unpacks this and parses each expression
once, at parse time. See [The AST](04-ast.md).

## Operators

Most are a single `match` arm. The multi character ones look ahead one character:

- `+` then `+` gives `PlusPlus`, `+` then `=` gives `PlusEqual`
- `-` then `-` gives `MinusMinus`, `-` then `>` gives `Arrow`
- `=` then `=` gives `Ee`, `=` then `>` gives `FatArrow`
- `&&` and `||` become `Keyword` tokens rather than their own kinds, which is a
  historical wart the interpreter has to work around when dispatching operators

## Newlines and semicolons

`\n` produces `TokenType::Newline` and `;` produces `TokenType::Semicolon`. They
mean the same thing to the parser, which tests both through
`Parser::is_line_break`.

They were once the same token. Giving `;` its own kind is what made the C style
`for` loop possible, since its three parts are separated by semicolons and the
parser needs to tell them from line ends.

## Comments

`#` skips to the end of the line. There is no block comment.

## What the lexer does not do

- No error recovery. The first bad character ends lexing.
- No interning. Every identifier allocates a `String`.
- No lookahead beyond one character, except in the `or when` case.

Next: [The parser](03-parser.md)
