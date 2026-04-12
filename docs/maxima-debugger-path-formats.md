# Maxima Debugger Path Formats

How the Maxima debugger (`src/mdebug.lisp`) stores and displays file paths. This documents every context where paths appear, and whether a debugging client can obtain a full canonical path.

## How Paths Are Stored Internally

**Lineinfo** (mdebug.lisp:14) stores `(line file)` where `file` is a **canonical absolute path** produced by `probe-file` at parse time (nparse.lisp:1789-1792). `probe-file` resolves symlinks and produces a truename.

**Breakpoint records** (bkpt struct: `form file file-line function`) copy the file from lineinfo — also **canonical absolute**.

**Pending (deferred) breakpoints** store the **user-provided path string** as-is. Normalization via `probe-file` happens only at resolution time.

## Automatic Displays (debugger entry)

These are what the debugger prints automatically when it stops — the most critical outputs for a client.

### Breakpoint hit

When a breakpoint fires, `*break-points*` (line 564) prints a prefix, then `break-dbm-loop` calls `set-env` (line 443):

```
Bkpt 0: (myfile.mac line 4, in function foo)
/Users/me/maxima/myfile.mac:4::
(dbm:1)
```

- Line 1: `Bkpt N: ` prefix + `(BASENAME line N, in function FNAME)` — **filename only** via `short-name`
- Line 2: `CANONICAL_PATH:LINE::` — **full canonical absolute path**
- Line 3: the debugger prompt

The `file:line::` line (from `set-env`, mdebug.lisp:449) is the **primary reliable source** of the canonical path. Regex: `^(/[^:]+):(\d+)::$`

### Step/next stop

When `:step` or `:next` lands on a new line, `maybe-break` (line 207) calls `break-dbm-loop` with a synthetic breakpoint, which also calls `set-env`. The output is the same format:

```
(myfile.mac line 5, in function foo)
/Users/me/maxima/myfile.mac:5::
(dbm:1)
```

Same two-line format — basename summary, then full canonical path.

### Error entry (no breakpoint)

When the debugger enters via an error (with `debugmode(true)`), `break-dbm-loop` is called with `at = nil`. It does NOT call `set-env`. Instead it calls `break-frame 0 nil` (line 467), which calls `print-one-frame` then prints:

```
foo(x=3) (myfile.mac line 4)
/Users/me/maxima/myfile.mac:4::
(dbm:1)
```

- Line 1: `FNAME(params) (BASENAME line N)` — **filename only**
- Line 2: `CANONICAL_PATH:LINE::` — **full canonical absolute path** (from `break-frame`, line 838)

The `file:line::` line is present here too. Same regex works.

**Note:** `print-one-frame` is called with `print-frame-number = nil` in this case, so there's no `#N:` prefix on the frame line.

## `:*` Command Outputs

### `:bt` (backtrace)

`$backtrace` (line 123) calls `print-one-frame` for each frame:

```
#0: foo(x=3) (myfile.mac line 4)
#1: bar(y=5) (myfile.mac line 12)
#2: baz(z=1) (otherfile.mac line 8)
```

**Filename only** — uses `short-name`. **No full path available.** No `file:line::` detail line.

A client must issue `:frame N` individually for each frame to get the full path. This means N round-trips per stop event.

### `:frame N`

`break-frame` (line 830) calls `print-one-frame` then appends the detail line:

```
#0: foo(x=3) (myfile.mac line 4)
/Users/me/maxima/myfile.mac:4::
```

**Full canonical absolute path** on the second line. This is the only way to get a full path for a specific stack frame via text output.

### `:info :bkpt`

`iterate-over-bkpts` calls `show-break-point` (line 665) for each breakpoint:

```
Bkpt 0: (myfile.mac line 4) (line 1 of foo)
Bkpt 1: (myfile.mac line 7) (disabled) (line 4 of bar)
```

**Filename only** — uses `short-name`. The `(line M of FNAME)` suffix shows the relative offset within the function.

**No full path available** from this output. A client would need `:lisp (bkpt-file (aref *break-points* N))` to get the canonical path.

### `:pending`

```
Pending 0: /path/as/user/typed/it.mac line 5
```

Displays the **user-provided path** string — whatever was passed to `:break`. Not normalized. Could be relative, absolute, or anything.

### `:up` / `:down`

`dbm-up` (line 641) calls `break-frame`, so the output includes the full `file:line::` detail line — same format as `:frame N`.

## Summary: Where Can a Client Get a Full Path?

| Context | Full path available? | How |
|---------|---------------------|-----|
| Breakpoint hit (automatic) | **Yes** | Parse `file:line::` from `set-env` output |
| Step/next stop (automatic) | **Yes** | Parse `file:line::` from `set-env` output |
| Error entry (automatic) | **Yes** | Parse `file:line::` from `break-frame` output |
| `:frame N` | **Yes** | Parse `file:line::` detail line |
| `:up` / `:down` | **Yes** | Parse `file:line::` detail line |
| `:bt` | **No** | Filename only; must `:frame N` each frame |
| `:info :bkpt` | **No** | Filename only; must use `:lisp` escape |
| `:pending` | **Partially** | User-provided string (may not be canonical) |

## `short-name` Details

`short-name` (line 661) simply finds the last `/` and returns everything after it:

```lisp
(defun short-name (name)
  (let ((pos (position #\/ name :from-end t)))
    (if pos (subseq name (f + 1 pos)) name)))
```

If the path has no `/` (e.g. it's already just a filename), it returns the whole string.

## Path Matching

When setting or resolving breakpoints, both the user-provided path and the stored lineinfo path are normalized via `probe-file` before comparison with `equal`. Matching always happens on canonical absolute paths, even if the user typed a relative path.

## Programmatic API

Maxima-callable functions (mdebug.lisp:844-900):

| Function | Returns |
|----------|---------|
| `set_breakpoint(fun, line)` | Breakpoint index, or -1 on failure/deferral |
| `breakpoint_line(n)` | Absolute file line of breakpoint `n`, or -1 |
| `breakpoint_count()` | Number of active breakpoints |
| `valid_breakpoint_count()` | Number of breakpoints still valid after redefinition |
| `pending_breakpoints()` | Maxima list of `[file, line]` pairs (user-provided paths) |
| `clear_breakpoints()` | Deletes all breakpoints and pending breakpoints |

### Missing from the API

**No `breakpoint_file(n)`.** The bkpt struct has `bkpt-file` (canonical absolute), but no Maxima-level accessor. A client must parse text output or use `:lisp`.

**No programmatic frame accessor.** `frame-info` (line 29) returns `(fname vals params backtr lineinfo bdlist)` where `lineinfo` has the canonical path, but it's Lisp-internal only.

### Suggested Upstream Additions

```lisp
;; Return canonical file path for breakpoint N
(defmfun $breakpoint_file (n)
  (if (and *break-points* (< n (length *break-points*)))
      (let ((bpt (aref *break-points* n)))
        (if bpt
            (let ((raw-bpt (if (null (car bpt)) (cdr bpt) bpt)))
              (or (bkpt-file raw-bpt) ""))
            ""))
      ""))

;; Return frame info as [function, file, line] or false
(defmfun $frame_info (n)
  (multiple-value-bind (fname vals params backtr lineinfo bdlist)
      (frame-info n)
    (declare (ignore vals params backtr bdlist))
    (if fname
        (list '(mlist) fname
              (if lineinfo (cadr lineinfo) "")
              (if lineinfo (car lineinfo) -1))
        nil)))
```
