# Maxima `.mac` File Syntax Guide

Reference for writing and maintaining Maxima source files (`.mac`) in the Aximar project.

## Statement Terminators

Every Maxima statement must end with `;` (display result) or `$` (suppress result):

```maxima
x: 42$          /* assign, suppress output */
x^2 + 1;        /* evaluate and display */
```

Forgetting a terminator causes a syntax error on the *next* line, which can be confusing.

## Variable Assignment

Use `:` (not `=`) for assignment. `=` creates an equation (a symbolic expression):

```maxima
x: 5$           /* assigns 5 to x */
x = 5;          /* creates the equation x = 5 (does NOT assign) */
```

## Function Definition

Use `:=` to define functions:

```maxima
f(x) := x^2 + 1$
g(x, y) := x + y$
```

For multi-statement function bodies, use `block`:

```maxima
f(x) := block(
  [a, b],           /* local variables */
  a: x + 1,
  b: a^2,
  b + 1             /* last expression is the return value */
)$
```

## Block Scoping

`block([vars], body)` introduces local variables. The variable list uses `:` for initialization:

```maxima
block(
  [a: 0, b, c],    /* a initialized to 0; b, c are unbound */
  b: a + 1,
  c: b * 2,
  c                 /* return value */
)
```

**Pitfall**: Variables NOT listed in `block([...])` are global. This is a common source of bugs in loops and nested functions.

## For Loops

Maxima `for` loops have specific syntax that differs from most languages:

```maxima
for i: 1 thru 10 do (
  /* body */
)$

for i thru 10 do (     /* shorthand: starts from 1 */
  /* body */
)$

for i: 0 step 2 thru 10 do (
  /* body with step */
)$

for x in [a, b, c] do (
  /* iterate over list */
)$
```

**Pitfall**: `step` is a keyword in `for` loops. Do NOT use `step` as a variable name in `for` loops (e.g., `for step: 1 thru n do ...` will fail). Use a different name like `i`, `j`, `idx`, `iter`, etc.

**Pitfall**: The `for var: init` syntax uses `:` which looks like assignment but is special loop syntax. There must be no space issues, but the `:` after the variable name is required.

## Conditional Expressions

```maxima
if x > 0 then x else -x$

if x > 0 then (
  result: x,
  print("positive")
) else if x = 0 then (
  result: 0
) else (
  result: -x
)$
```

`if` is an expression and returns a value — the last expression in the chosen branch.

## Equality and Comparison

| Operator | Meaning |
|----------|---------|
| `=` | Equation (symbolic) |
| `#` | Not equal |
| `<`, `>`, `<=`, `>=` | Numeric comparison |

```maxima
if x # false then print("not false")$
if x = 0 then print("zero")$     /* compares value, not assigns */
```

**Pitfall**: `#` means "not equal" in Maxima, not a comment. Comments use `/* ... */`.

## Comments

Maxima only supports block comments:

```maxima
/* This is a comment */

/* Multi-line
   comment */
```

There are no line comments (`//` or `#` style).

## Lists and Iteration

```maxima
lst: [1, 2, 3, 4, 5]$
first(lst);               /* 1 */
rest(lst);                /* [2, 3, 4, 5] */
length(lst);              /* 5 */
lst[2];                   /* 2 (1-indexed) */
endcons(6, lst);          /* [1, 2, 3, 4, 5, 6] (returns new list) */
append(lst, [6, 7]);      /* [1, 2, 3, 4, 5, 6, 7] */
map(f, lst);              /* [f(1), f(2), f(3), f(4), f(5)] */
makelist(i^2, i, 1, 5);   /* [1, 4, 9, 16, 25] */
```

**Pitfall**: Lists are 1-indexed, not 0-indexed.

**Pitfall**: `endcons` and `append` return new lists — they do NOT modify in place. You must reassign: `lst: endcons(x, lst)`.

## String Operations

Requires `load("stringproc")$` first:

```maxima
load("stringproc")$
sconcat("hello", " ", "world");   /* "hello world" */
simplode(["a","b","c"], ",");     /* "a,b,c" */
string(42);                        /* "42" (number to string) */
stringp("hello");                  /* true */
```

## Error Handling

`errcatch` catches errors and returns `[]` on failure:

```maxima
result: errcatch(1/0)$
if result = [] then print("error occurred")$

/* Wrapping evaluation */
val: errcatch(float(subst(x=0, expr)))$
if val = [] then false else first(val)$
```

## Common Pitfalls

### 1. `false` is a value, not just boolean

In Maxima, `false` is a regular atom. It's distinct from `[]` (empty list). Functions that "fail" may return either depending on convention.

### 2. `return()` only works inside `for` loops

`return(val)` exits the innermost `for` loop and makes the loop expression evaluate to `val`. It does NOT exit a `block` or function. The last expression in a block is the return value.

```maxima
/* This does NOT work as expected: */
f(x) := block(
  if x < 0 then return(-1),    /* WRONG: return is for loops only here */
  x + 1
)$

/* Correct approach: */
f(x) := block(
  if x < 0 then -1
  else x + 1
)$

/* return() inside a for loop: */
find_first(lst, pred) := block([result: false],
  for x in lst do (
    if pred(x) then (result: x, return(x))
  ),
  result
)$
```

### 3. Operator precedence with `:` assignment

Assignment `:` has very low precedence. Parentheses may be needed:

```maxima
/* Careful with conditional assignments */
a: if x > 0 then 1 else -1$     /* OK: assigns result of if */
```

### 4. `numberp` and type checking

```maxima
numberp(3.14);      /* true */
numberp(3);         /* true */
numberp(%pi);       /* false — it's symbolic */
numberp(false);     /* false */
integerp(3);        /* true */
floatnump(3.14);    /* true */
```

### 5. Atoms vs. strings

Unquoted names are atoms (symbols); quoted names are strings:

```maxima
red;        /* atom */
"red";      /* string */
red = "red"; /* false — different types */
```

### 6. Pattern: style options as `key=value`

In Aximar's plotting code, `color="red"` creates an equation `color = "red"`. We check this with `op(item) = "="` and extract with `lhs(item)` / `rhs(item)`. Options accumulate and apply to subsequent draw objects — they must appear BEFORE the objects they affect.

## Reserved Words

Avoid using these as variable names: `do`, `for`, `thru`, `step`, `from`, `next`, `while`, `unless`, `in`, `then`, `else`, `if`, `and`, `or`, `not`, `true`, `false`.
