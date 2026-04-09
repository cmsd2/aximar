# Maxima Debugger Internals

Reference documentation on Maxima's built-in debugging, tracing, and state inspection facilities, as discovered from the Maxima source code (`src/mdebug.lisp`, `src/mtrace.lisp`, `src/suprv1.lisp`, `src/server.lisp`, `src/macsys.lisp`) and mailing list discussions. This documents what is available *without modifying Maxima's source*.

## Source-Level Debugger

Implemented in `src/mdebug.lisp`.

### Enabling

```maxima
debugmode(true);    /* Enter Maxima debugger on errors */
debugmode(lisp);    /* Enter Lisp debugger on errors */
debugmode(false);   /* Default: just print the error */
```

The internal variable `*mdebug*` controls this (set in `src/suprv1.lisp`).

### Debugger Commands

When a breakpoint fires or an error occurs with `debugmode(true)`, Maxima switches to a `dbm:` prompt. These colon-prefixed commands are available:

| Command | Description |
|---------|-------------|
| `:break F` | Set breakpoint at entry to function `F` |
| `:break F N` | Set breakpoint at line offset `N` in function `F` |
| `:break "file.mac" N` | Set breakpoint at line `N` in file (**unreliable — see below**) |
| `:bt` | Print backtrace of all stack frames |
| `:bt N` | Print top `N` stack frames |
| `:frame N` | Print details of stack frame `N` |
| `:step` | Step into: advance to next source line, entering subroutines |
| `:next` | Step over: advance to next line, skipping subroutine internals |
| `:resume` | Resume execution (continue) |
| `:continue` | Alias for `:resume` |
| `:quit` | Quit current debugger level |
| `:top` | Return to top-level Maxima prompt |
| `:info :bkpt` | List all breakpoints |
| `:delete N` | Delete breakpoint number `N` |
| `:delete` | Delete all breakpoints |
| `:enable N` | Enable breakpoint `N` |
| `:disable N` | Disable breakpoint `N` |
| `:lisp FORM` | Evaluate a Common Lisp form |
| `:lisp-quiet FORM` | Evaluate a Lisp form without printing prompt |
| `:help` | Show debugger command help |

At the `dbm:` prompt, arbitrary Maxima expressions can also be evaluated to inspect variables.

### How It Works Internally

- The main debugger loop is `break-dbm-loop` (mdebug.lisp:376)
- Stack inspection uses `*mlambda-call-stack*` array
- `frame-info` extracts function name, parameter names, parameter values, and line info from the stack
- `print-one-frame` formats a single frame for display
- `$backtrace` iterates over frames and prints them
- Breakpoints stored in `*break-point-vector*`
- `break-function` (line 494) sets a breakpoint by parsing function name and line offset
- All I/O goes through `*debug-io*` — plain text, no structured protocol

### Breakpoint Reliability

**Function-name breakpoints work:**
```maxima
:break myfunction       /* break at entry */
:break myfunction 3     /* break at 3rd line of function body */
```

**File-path breakpoints are unreliable:**
```maxima
:break "test.mac" 5    /* registers but often doesn't fire */
```

This was confirmed by a 2025 mailing list thread where someone building a VS Code debug adapter hit exactly this issue. The function+offset form is the only reliable mechanism.

### Debugger Prompt Detection

When a breakpoint fires, Maxima outputs something like:

```
Bkpt 0:(test.mac 3)
dbm:1>
```

The prompt format is `dbm:N>` where `N` is the debugger nesting level. After executing a debugger command, the prompt reappears. When `:resume` is issued, normal Maxima output continues.

### Known Issues

- `:bt` and `:frame` produce no output under GCL. Use SBCL.
- `errcatch` swallows errors and also suppresses breakpoints.
- Breakpoints set on functions in some packages never fire.
- Redefining a function clears its breakpoints.
- Single-stepping with `:step` requires SBCL compiled with `(declaim (optimize (debug 3)))` to get source-level info.

## Trace System

Implemented in `src/mtrace.lisp`.

### Basic Usage

```maxima
trace(f1, f2, f3);     /* Start tracing functions */
trace();                /* List all traced functions */
untrace(f1);            /* Stop tracing */
untrace();              /* Stop tracing all */
```

Traced functions print entry/exit with arguments and return values:

```
 1 Enter f [x = 5]
 1 Exit  f 25
```

### Trace Options

```maxima
trace_options(f, option1, option2, ...);
```

| Option | Effect |
|--------|--------|
| `noprint` | Suppress default trace output |
| `break` | Break (enter debugger) on entry/exit |
| `lisp_print` | Use Lisp printer instead of Maxima display |
| `info(expr)` | Evaluate and print `expr` at trace point |
| `errorcatch` | Wrap call in `errcatch` |

Each option can take a predicate: `break(pred)` where `pred` is a function of `(level, direction, function, item)`. This enables **conditional breakpoints** on function entry/exit.

### Modifying Arguments and Return Values

During a trace breakpoint, `trace_break_arg` is bound to:
- The argument list (on function entry)
- The return value (on function exit)

Modifying this variable changes what the function receives or returns. This is useful for debugging and testing.

### Timer

```maxima
timer(f);               /* Start timing function f */
untimer(f);              /* Stop timing */
timer_info(f);           /* Show accumulated time and call count */
timer_info();            /* Show all timed functions */
```

## State Inspection

### Variable and Function Lists

```maxima
values;                 /* All user-assigned variables */
functions;              /* All user-defined functions */
macros;                 /* All user-defined macros */
arrays;                 /* All user-defined arrays */
rules;                  /* All user-defined rules */
aliases;                /* All user-defined aliases */
dependencies;           /* All declared dependencies */
gradefs;                /* All gradient definitions */
props;                  /* All properties */
labels;                 /* All input/output labels */
```

These are all accessible via the master list `infolists`.

### Function Inspection

```maxima
fundef(f);              /* Show the definition of function f */
dispfun(f);             /* Display function definition */
grind(f);               /* Print in re-readable form */
properties(f);          /* Show all properties of symbol f */
propvars(prop);         /* List all symbols with property prop */
```

### Variable Monitoring

```maxima
setcheck: [x, y, z]$           /* Monitor these variables */
setcheckbreak: true$            /* Break when any setcheck var is assigned */
setval;                         /* During a setcheckbreak, holds the pending value */

refcheck: true$                 /* Print message when a bound variable is first referenced */
```

`setcheckbreak` is particularly useful for IDE watchpoints — it creates a debugger break whenever a monitored variable is assigned.

### Expression Inspection

```maxima
listofvars(expr);       /* List all variables in an expression */
args(expr);             /* List arguments of the top-level operator */
op(expr);               /* The top-level operator */
part(expr, n);          /* The nth part of an expression */
```

## Process Communication

### Pipe-based (default)

Maxima reads from `*standard-input*` and writes to `*standard-output*`. The main REPL loop is in `src/macsys.lisp` function `continue` (line 163). Input is read via `dbm-read` which handles:
- Maxima expressions (terminated by `;` or `$`)
- Debugger commands (`:` prefix)
- Help queries (`?` prefix)

### Socket mode

```bash
maxima --server PORT
# or
maxima -s PORT
```

Maxima connects as a **client** to a TCP socket on the given port. Implemented in `src/server.lisp`:
- `setup-client` opens the socket and redirects all I/O streams
- Sends `pid=<pid>` as the first message
- All subsequent I/O goes through the socket

This is how xmaxima connects. wxMaxima uses a similar mechanism with its XML layer on top.

### Batch/string modes

```bash
maxima --batch file.mac              # Process file, show input+output
maxima --batch-string "expr1; expr2" # Process string
maxima --run-string "expr"           # Process in interactive mode
maxima --batch-lisp file.lisp        # Load and execute Lisp file
```

### Quiet modes

| Flag | Effect |
|------|--------|
| `-q` / `--quiet` | Suppress startup banner |
| `--very-quiet` | Suppress banner and labels |
| `--very-very-quiet` | Suppress banner, labels, and most output (`$ttyoff: true`) |

### Custom prompt delimiters

For reliable output parsing, inject custom delimiters via a preloaded Lisp file:

```lisp
;; preload.lisp — load with: maxima --preload-lisp preload.lisp
(setq *prompt-prefix* "<<<PROMPT:")
(setq *prompt-suffix* ">>>")
(setq *general-display-prefix* "<<<OUTPUT:")
```

These wrap every prompt and output block in detectable delimiters. Used by TeXmacs and imaxima for protocol framing.

### Alternative display hooks

```lisp
;; Replace the default display function
(setq *alt-display1d*
  (lambda (form)
    (format t "<<<RESULT:~A>>>~%" (mgrind-output form))))

(setq *alt-display2d*
  (lambda (form)
    ;; custom 2D display handler
    ))
```

The `set_alt_display` function from `share/contrib/alt-display/alt-display.lisp` provides a safe Maxima-level wrapper:

```maxima
load("alt-display.lisp")$
set_alt_display(1, my_display_function)$
```

This is the most powerful hook for building structured output without modifying Maxima. A preloaded function could wrap all output in JSON, XML, or any other format.

## Programmatic Evaluation

```maxima
load("stringproc")$
eval_string("diff(x^3, x)");       /* Parse and evaluate a string */
parse_string("diff(x^3, x)");      /* Parse without evaluating */
```

From Lisp:
```lisp
:lisp (eval-string-lisp "some lisp form")
```

## How Existing Frontends Communicate

### Emacs maxima-mode

- Spawns Maxima as a `comint` subprocess (pipe-based)
- Sends expressions via the pipe
- Parses output by matching prompts (`(%i1)`, `(%o1)`) with regexps
- No structured protocol

### Emacs imaxima

- Sets `$display2d` to `'$imaxima` to trigger LaTeX output
- Overrides `displa` to call `latex` for formatted output
- Wraps prompts in control characters (char codes 3/4)
- Wraps LaTeX output in control characters (char codes 2/5)
- The closest thing to a protocol — control chars delimit output regions

### Emacs dbl.el

- A shell mode for source-level debugging
- Parses debugger output for `filename:line` patterns
- Highlights current source location in a buffer
- `M-s` steps, `C-x space` sets breakpoints by computing function+offset from cursor
- Pure text-parsing approach, no protocol

### wxMaxima

- Maxima connects via TCP socket (`setup-client`)
- wxMaxima injects `wxmathml.lisp` which converts output to custom XML
- An "XML monitor" sidebar shows raw protocol traffic
- **Debugging does not work through wxMaxima** — the debugger's `dbm:` prompt hijacks I/O at a low level that wxMaxima can't route

### aximar (this project)

- Spawns Maxima with `--very-quiet` via pipes (or Docker/WSL)
- Sentinel-based protocol: expressions wrapped with `__AXIMAR_EVAL_END__`
- `display2d:false`, captures LaTeX via `tex(%)`
- No debugger support currently
- See `docs/maxima-protocol.md` for full details
