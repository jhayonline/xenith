# The AST

`src/nodes.rs`, 541 lines. One enum with 31 variants, each wrapping a struct that
holds that construct's parts.

## The enum

```rust
pub enum Node {
    Number(NumberNode),
    String(StringNode),
    List(ListNode),
    Ternary(Box<TernaryNode>),
    VarAccess(VarAccessNode),
    VarAssign(Box<VarAssignNode>),
    BinaryOperator(Box<BinaryOperatorNode>),
    UnaryOp(Box<UnaryOpNode>),
    If(Box<IfNode>),
    For(Box<ForNode>),
    ForClassic(Box<ForClassicNode>),
    While(Box<WhileNode>),
    FuncDef(Box<FuncDefNode>),
    Call(Box<CallNode>),
    Return(Box<ReturnNode>),
    Continue(ContinueNode),
    Break(BreakNode),
    InterpolatedString(InterpolatedStringNode),
    MethodAccess(MethodAccessNode),
    Map(MapNode),
    Panic(Box<PanicNode>),
    Grab(Box<GrabNode>),
    Export(Box<ExportNode>),
    StructDef(Box<StructDefNode>),
    TypeAlias(Box<TypeAliasNode>),
    BoolLiteral(BoolLiteralNode),
    NullLiteral(NullLiteralNode),
    StructInstantiation(Box<StructInstantiationNode>),
    TupleLiteral(TupleLiteralNode),
    Destructure(DestructureNode),
    DestructurePattern(DestructurePatternNode),
}
```

Larger variants are boxed to keep `Node` itself small, since it is moved and
cloned constantly.

## Positions

Every node struct ends with the same two fields:

```rust
pub position_start: Position,
pub position_end: Position,
```

`Node::position_start()` and `Node::position_end()` are big matches over every
variant returning a reference to them. Adding a variant means adding an arm to
both, which the compiler will insist on.

Positions are what make diagnostics point at the right code. When an error is
raised deep in evaluation without a position, the interpreter fills in the
current node's span before returning it.

## Nodes worth a closer look

### VarAssignNode

```rust
pub struct VarAssignNode {
    pub variable_name_token: Token,
    pub var_type: Option<Type>,
    pub value_node: Box<Node>,
    pub is_constant: bool,
    pub is_declaration: bool,
    ...
}
```

`is_declaration` separates `let x = v` from `x = v`. A declaration writes to the
current scope; an assignment walks out to find the scope that declared the name.
The two were once indistinguishable, which meant every assignment inside a block
created a shadow copy and left the outer variable untouched.

`is_constant` records `const let` so the interpreter can refuse a later
assignment.

### The two for nodes

```rust
pub struct ForNode {
    pub variable_name_token: Token,
    pub iterable_node: Box<Node>,
    pub body_node: Box<Node>,
    ...
}

pub struct ForClassicNode {
    pub init_node: Option<Box<Node>>,
    pub condition_node: Option<Box<Node>>,
    pub step_node: Option<Box<Node>>,
    pub body_node: Box<Node>,
    ...
}
```

Every part of the classic loop is optional, so `for (;;)` is a valid infinite
loop.

`ForNode` encodes a two variable pattern by putting the literal text `(k,v)` in
`variable_name_token`, which the interpreter splits on the comma. That is a
shortcut worth replacing with a proper field if the node is touched again.

### MethodAccessNode

```rust
pub struct MethodAccessNode {
    pub object: Box<Node>,
    pub method_name: Token,
    ...
}
```

Used for both `value.field` and `value.method()`. Which one it is depends on
whether a `Call` wraps it, so the interpreter checks for
`Node::MethodAccess` inside `visit_call` before evaluating the callee normally.

### InterpolatedStringNode

```rust
pub struct InterpolatedStringNode {
    pub parts: Vec<InterpolationPart>,
    ...
}

pub struct InterpolationPart {
    pub is_expression: bool,
    pub content: String,
}
```

The content of an expression part is *source text*, not a parsed node. The
interpreter lexes and parses it every time the string is evaluated. That is both
slow and the reason interpolated expressions have their own quirks. Parsing them
at parse time, into `Vec<Node>`, is a worthwhile change.

`InterpolatedStringNode::new` unpacks the delimited string the lexer produced.
`escape_interpolation_part` and `unescape_interpolation_part` live here too.

### DestructurePattern

```rust
pub enum DestructurePattern {
    Variable(Token),
    Ignore,
    Tuple(Vec<DestructurePattern>),
}
```

Recursive, so `let ((a, b), c) = ...` is a `Tuple` containing a `Tuple` and a
`Variable`.

## Adding a node

1. Add the variant to `Node` and the struct beside it.
2. Add arms to `position_start()` and `position_end()`.
3. Parse it in `src/parser.rs`.
4. Add a `visit_*` method in `src/interpreter.rs` and an arm to `visit`.
5. If it defines a name, handle it in the language server's `Collector::walk` so
   the editor can see it.

The compiler will find steps 2 and 4 for you. It will not find step 5.

Next: [The interpreter](05-interpreter.md)
