# The Xenith Programming Language

Xenith is a lightweight, expressive programming language designed for simplicity and readability. It supports variables, arithmetic, control flow (if, loops), functions, lists, and built-in utilities—all in a clean syntax.

Its goal is to make writing and experimenting with code fun and intuitive, while allowing developers to focus on logic rather than boilerplate.

## About Xenith (Rust Edition)

Xenith was originally written in Python, which made developing and experimenting with language features easy—but it came with significant downsides. Interpreted languages like Python can be frustratingly slow for building another interpreted language: executing loops, parsing code, and managing runtime values all added noticeable overhead. Debugging performance issues and handling memory-intensive operations became cumbersome, and the lack of strict typing made certain runtime errors harder to catch early.

At the time I wrote the Python version, I was still very new to Rust and not comfortable enough with its ecosystem and ownership model to implement a full interpreter. Over the past 14 months, and especially during the last 8 months of consistent Rust development, I’ve gained the experience and confidence to take on this rewrite.

This Rust edition of Xenith aims to retain the language’s original simplicity and expressiveness while drastically improving performance, safety, and maintainability—thanks to Rust’s speed, type system, and memory guarantees.

## Xenith's Features

- ✅ Basic output and variables
- ✅ Arithmetic operations
- ✅ Comparison and logical operators
- ✅ Control flow (when/or when/otherwise, ternary)
- ✅ Lists with modern syntax (append, pop, indexing)
- ✅ Loops (for, while, stop, skip)
- ✅ Methods (including arrow methods and closures)
- ✅ Built-in methods
- ✅ Fibonacci sequence
- ✅ Data processing with lists
- ✅ Increment/decrement operators (++, --, +=, -=)
- ✅ Maps with iteration (keys, values, items)
- ✅ Pattern matching (match expressions)
- ✅ String interpolation
- ✅ Proper variable scoping (methods can now modify outer variables!)
- ✅ Error handling (try/catch/panic)
- ✅ Import module system
- ✅ Type System
