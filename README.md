# Jack

Jack is a **J**VM-targeting st**ack**-based language. This works quite
naturally, as the JVM is, indeed, stack-based itself.

# The Language

Jack is a simple stack-based language. Find its documentation in JACK.md.

# How It Works

The `jack` binary requires the following arguments:

1. `-j` or `--jasmin`: the path to the [Jasmin](https://github.com/davidar/jasmin) assembler jar
2. The path to the `.jack` source file

It then reads the source file and goes through the following steps.

## 1. Tokenizing

Jack uses [Klex](https://github.com/sanj0/klex) to tokenize the source code.

## 2. Parsing

The tokens are then being parsed into an abstract syntax tree. At this stage,
comments are being skipped.

## 3. Analysis

Next, the abstract syntax tree ist being traversed. This effectively simulates
a simplified exectution of the program on a type level. The following checks are run:

1. Type checking: Can every node work with what's on the stack? For example,
   `print` just requires anything on the stack, while `+` requires two numbers of the
   same type
2. Branach checking
    1. `if` either has to leave the stack alone or have an `else` that effects
       it in the same way
    2. `while` is not allowed to alter what types are on the stack (but has to
       leave the conditional `Int` on stack after every iteration)
3. The program has to leave with an empty stack

The analyzer also injects stack information into the tree, so that for the next
step, every node knows what types it works with. Additionally, it figures out
the maximum stack and local variable size, which the JVM requires.

## 4. Code generation

The abstract syntax tree is traversed for a final type, during which every node
emits its Jasmin assembly. `Print` nodes (that print the top element of the
stack to standard out) for example need the stack information:

```jasmin
getstatic java/lang/System/out Ljava/io/PrintStream;
swap
; in case of a String on the stack,
; the following line is emitted:
invokevirtual java/io/PrintStream/print(Ljava/lang/String;)V
; while in case of an int,
; the following has to be emitted:
; (notice the type difference between the parantheses)
invokevirtual java/io/PrintStream/print(I)V
```

At this stage, line number directives are also injected into the assembly,
meaning that when debugging or upon runtime errors, Java can actually show you
the correct line in the `.jack` source file.
As everything in Java needs to be a class, Jack has to generate one for the
program. It has the name of the source file (sans .jack) and the default
constructor.

## 4. Assembler

Lastly, `jack` writes the generated assembly into a file and calls the Jasmin
assembler at the given path.

Jasmin creates a `.class` File with the same name as the source file. This
`class` can be executed with `java` or debugged with `jdb` etc.
