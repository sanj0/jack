# The Jack Language

Jack is stack-based, which means that it only works with a global stack. For
every operation, all operands are pushed onto the stack. The operation itself
then pops them off and pushed the result (if any). This means that math
expression work in the reverse polish notation. For example, to calculate
`40 + 2`, one would employ the folling Jack-snippet: `40 2 +`.

Examples can be found in /examples.

## Using the compiler

To compile a source file at `Hello.jack` into `Hello.class`, with the Jasmin
assembler jar being at `../jasmin/jasmin.jar`, consider the
following command:

```bash
cargo r -- -j ../jasmin/jasmin.jar Hello.jack
```

Now you can run the code using `java Hello`.

## Pushing onto the stack

Currently, two types of values are pushable: `Int` and `String`. Booleans are
equal to `Int`s, with `0` being `false` and every other value being `true`
Writing an expression, that pushes a literal is as simple as stating that
literal:

```forth
40 2
"hello"
```

After executing the above code, the stack contains `[40, 2, "hello"]`

## Operators

Most standard math and boolean operators exist in Jack. `==` also works on two
`String`, where it employs `equals()` instead of referential equality.

```forth
40 2 +
20 20 2 + +
== // result in a stack of [1] (= [true])
```

## Stack Manipulation

1. `dup` duplicates the top of the stack
2. `drop` drops the top value from the stack
3. `swap` swaps the two top values on the stack

## Control flow

`if` / `else` and `while` control flow is currently implemented:

```ebnf
if-else = if <head>? <body> (else <body>)?;
while   = while <head>? <body>;
head    = \(<expression>*\);
body    = (<expression>) | ({ <expression>* })
```

The »head« of `if` and `while` is optional, meaning the following two ways to
write an infinite loop are semantically equal:

```
true while {
// ...
true
}

while (true) {
// ...
}
```

## Intrinsic Functions

### `print`

`print` pops any value from the top of the stack and prints it to standard out.

```forth
"hello, world!" print
42 print
```

### `readln`

`readln` waits for user input at standard in and pushes it (as a String) onto
the stack.

```forth
// echoes user input:
readln print
```

