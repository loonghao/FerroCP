# FerroCP error model

How failures are represented, classified, and surfaced to callers.

## 1. Principles

1. **Never swallow an error.** If an operation fails, the failure reaches the
   caller. Turning a failure into a "successful" result with zero files copied
   is treated as a bug.
2. **Classify with types, not text.** Recoverability and error categories come
   from [`std::io::ErrorKind`], never from searching the message for English
   substrings — those differ between platforms and locales.
3. **Carry context.** An error says what was being attempted and on which path,
   not only why the OS call failed.

## 2. The error type

`ferrocp_types::Error` is the single error type across the workspace. The
variants that matter for callers:

| Variant | Meaning | `io_kind()` |
| --- | --- | --- |
| `Io { message, kind }` | An I/O failure. `kind` is the originating `ErrorKind` when there is one. | `kind` |
| `FileNotFound { path }` | A path does not exist. | `NotFound` |
| `PermissionDenied { path }` | Access was refused. | `PermissionDenied` |
| `Config { message }` | Invalid options or configuration. | — |
| `Network { message }` | Network transfer failure. | — |
| `Compression { message }` | Compression failure. | — |
| `DeviceDetection { message }` | Device detection failure. | — |
| `ZeroCopy { message }` | Zero-copy operation failed. | — |
| `Sync { message }` | Synchronisation failure. | — |
| `Cancelled` | The operation was cancelled. | — |
| `Timeout { seconds }` | The operation timed out. | — |
| `Other { message }` | Anything else. | — |
| `WithContext { error, context }` | Any of the above plus structured context. | delegated |

### Recoverability

`is_recoverable()` is decided by the error kind:

| Kind | Recoverable |
| --- | --- |
| `Interrupted`, `WouldBlock`, `TimedOut` | yes |
| everything else with a known kind | no |
| no kind available (`kind: None`) | no |

An error with no kind is treated as **not** recoverable rather than guessed at.
Retrying a failing operation is only safe when the failure is known to be
transient.

### Before this model

`is_recoverable()` matched on substrings such as `"Interrupted"`,
`"WouldBlock"` and `"timed out"`. A non-English error message, or a platform
that words the same condition differently, silently changed the retry decision.
`From<std::io::Error>` also flattened every I/O failure into `Error::Io`,
losing the distinction between "missing" and "forbidden".

## 3. Context

Attach context with `Error::with_context` / `with_path_context`:

```rust
use ferrocp_types::Error;

let error = Error::other("disk full").with_path_context("copy_file", "/data/a.bin");
assert_eq!(error.context().unwrap().operation, "copy_file");
```

`ErrorContext` carries:

- `operation` — what was being attempted (`"copy_file"`, `"copy_directory"`).
- `path` — the path being operated on, when there is one.
- `details` — free-form key/value pairs.
- `timestamp` — when the error occurred.

Useful accessors:

| Method | Returns |
| --- | --- |
| `error.context()` | the attached `ErrorContext`, if any |
| `error.root_cause()` | the innermost error, with wrappers removed |
| `error.chain()` | `"operation: cause: cause"`, for logs |
| `error.io_kind()` | the `ErrorKind`, when one is known |

Wrapping is idempotent: `with_context` on an error that already has context
keeps the innermost context instead of building a deeper chain. `kind()`,
`severity()` and `is_recoverable()` all delegate through the wrapper, so
wrapping never changes how an error is classified.

## 4. Python exceptions

| Python exception | Base | Raised for |
| --- | --- | --- |
| `FerrocpError` | `Exception` | base class for FerroCP failures |
| `IoError` | `FerrocpError` | I/O failures without a more specific kind |
| `FileNotFoundError` | builtin `FileNotFoundError` | `NotFound` |
| `PermissionError` | builtin `PermissionError` | `PermissionDenied` |
| `ConfigError` | `FerrocpError` | invalid options |
| `NetworkError` | `FerrocpError` | network failures |
| `SyncError` | `FerrocpError` | synchronisation failures |
| `RuntimeError` | builtin `RuntimeError` | an internal panic contained at the boundary |

The file and permission exceptions derive from the **builtins**, so standard
idioms work:

```python
try:
    ferrocp.copy("missing.txt", "dest.txt")
except FileNotFoundError:
    ...
```

Because Python has single inheritance here, those two do not also derive from
`FerrocpError`. Catch `FerrocpError` for FerroCP-specific failures and `OSError`
for os-level ones.

## 5. Panic isolation

A panic inside the extension module must not surface as
`pyo3_runtime.PanicException`, which callers cannot catch with `except
FerrocpError`.

- `catch_panic(operation, f)` runs `f` and converts a panic into a
  `FerrocpError`.
- Locks recover from poisoning instead of panicking: a poisoned cache lock
  previously turned one panic into a second panic at the PyO3 boundary.
- `Default for PyCopyOptions` builds the struct directly instead of unwrapping
  a `Result`.
- The Tokio fallback runtime is created once and reused. A build failure is
  returned as an error instead of `expect`-ing.

> **Important limitation.** `catch_panic` only works when the crate is built
> with unwinding panics. The workspace release profile sets `panic = "abort"`,
> which makes any panic in a release-built wheel abort the host interpreter
> before this code runs. The Python extension must be built with
> `panic = "unwind"` (PyO3 requires this); see PIP-3212, which re-enables the
> `ferrocp-python` crate.
