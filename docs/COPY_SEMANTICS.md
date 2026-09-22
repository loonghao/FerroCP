# FerroCP copy semantics

This document is the normative contract for what FerroCP does when it copies a
file or a directory tree. Every behaviour below is enforced by code and covered
by tests; anything not listed here is **undefined** and may change.

Two rules apply to the whole contract:

1. **No silent data loss.** FerroCP never drops an entry without recording it in
   [`CopyStats`](../crates/ferrocp-types/src/types.rs). "Silently skipped" is
   treated as a bug, not as a default.
2. **No declared-but-ineffective options.** An option that cannot be honoured is
   rejected with an error, never ignored.

## 1. Overwrite policy

What happens when the destination path already exists.

| Policy         | Behaviour                                                    | `CopyStats` effect                     |
| -------------- | ------------------------------------------------------------ | -------------------------------------- |
| `always`       | Truncate and rewrite the destination. **This is the default** | `files_copied += 1`                    |
| `never`        | Keep the destination, do not read the source                 | `files_skipped += 1`                   |
| `if_newer`     | Copy only when the source mtime is strictly newer            | `files_copied` or `files_skipped += 1` |
| `if_different` | Copy only when size or mtime differ                          | `files_copied` or `files_skipped += 1` |
| `fail`         | Abort the copy with an error                                 | `errors += 1`                          |
| `prompt`       | Ask the registered handler                                   | depends on the answer                  |

Accepted spellings are case-insensitive; `OverwritePolicy::parse` also accepts
`overwrite`/`auto`/`all`, `skip`, `newer`, `different`, `error` and `ask`.
Unknown values are rejected.

Notes:

- A missing destination always proceeds, under every policy.
- Replacing an existing **directory** with a file is always an error.
- `prompt` **requires** a handler. Without one the copy fails with a
  configuration error, because an unanswered prompt must never be treated as
  "yes". See §4.
- Equal timestamps are not "newer": `if_newer` skips unchanged sources.

### Before this contract

`CopyOptions.overwrite` defaulted to `"prompt"` but was never read, so the
behaviour was always "truncate and overwrite". The default is now `"always"`,
which is the behaviour that actually happened. Runtime behaviour is unchanged;
only the advertised (and previously false) default was corrected.

## 2. Copy mode

`CopyMode` is a coarse preset that is translated into an overwrite policy before
any I/O happens, so the two can never disagree.

| `CopyMode`  | Overwrite policy |
| ----------- | ---------------- |
| `all`       | `always`         |
| `newer`     | `if_newer`       |
| `different` | `if_different`   |
| `mirror`    | **rejected**     |

`mirror` (delete destination entries that are absent from the source) is
declared but not implemented. It now fails with a clear error instead of
silently behaving like `all`, because deleting files is destructive and must not
happen behind the user's back.

## 3. Symbolic links

| Mode       | Behaviour                                                                 |
| ---------- | ------------------------------------------------------------------------- |
| `preserve` | Recreate the link itself. **This is the default.** Dangling links stay dangling. |
| `follow`   | Copy the content the link points at. A link to a directory is copied as a directory. |
| `fail`     | Abort that entry with an error.                                            |

`preserve` is the default because it never loses data and reproduces the source
tree faithfully - which matters for VFX pipelines, where symlinks carry version
and frame-sequence information.

Notes:

- The mode applies to **single-file copies as well as directory trees**. Copying
  `link.txt` by name honours the same contract as encountering it inside a walk;
  this is enforced in `ferrocp_io::policy::apply_copy_contract`, which every
  engine (buffered, micro-file, parallel) calls before it opens the destination.
- In `follow` mode a **dangling link is an error**, not a skip.
- In `follow` mode a link cycle (`sub/back -> .`) is detected via canonicalised
  directories on the current recursion stack. The cyclic entry is skipped and
  recorded in `files_skipped`; the copy terminates.
- Recreated links are counted in `CopyStats.symlinks_created`. Links copied as
  regular files (`follow`) are counted in `files_copied`.
- A link is recreated with its **raw target**, so relative links stay relative.
- `preserve` refuses to replace an existing **directory** with a link.

### Before this contract

Directory copies skipped symlinks silently: `executor.rs` counted them in
`files_skipped` without ever creating anything, and the `follow_symlinks` option
was never read.

## 4. Asking the user (`prompt`)

The handler is a callback, so `prompt` works the same way everywhere:

- **Rust**: `ferrocp_io::OverwritePrompt::new(|source, destination| ...)`.
- **CLI**: `--overwrite prompt` asks on the terminal. When stdin is not a
  terminal it warns and skips rather than guessing.
- **Python**: `CopyOptions(overwrite="prompt", overwrite_callback=fn)`, where
  `fn(source, destination) -> bool`. Returning `True` overwrites.

## 5. Platform matrix

The matrix below holds for **every engine** (buffered, micro-file, parallel):
all three share the same metadata-preservation helper. It is not a property of
the buffered engine alone.

| Behaviour                       | Linux / macOS                     | Windows                                                   |
| ------------------------------- | --------------------------------- | --------------------------------------------------------- |
| Timestamps (mtime/atime)        | Preserved                         | Preserved                                                 |
| Permission bits                 | Full mode preserved               | Only the **read-only attribute** is preserved             |
| ACLs / ownership                | Not preserved                     | Not preserved                                             |
| Symbolic links (`preserve`)     | Recreated (file, dir, dangling)   | Recreated via `symlink_file` / `symlink_dir`              |
| Creating symlinks requires      | Nothing                           | Developer Mode **or** an elevated process                 |
| Dangling link classification    | N/A (`symlink` handles all cases) | Recreated as a file link on Windows                       |
| Overwrite policies              | Identical                         | Identical                                                 |
| Special files (socket, FIFO)    | Skipped, counted in `files_skipped` | N/A                                                     |

Windows notes:

- Creating a symlink without the required privilege is reported as an error and
  counted in `errors`; the entry is never silently dropped.
- ACLs and ownership are **not** copied on any platform. If you need them, copy
  them explicitly.
- A read-only destination is cleared to writable when the source is writable, so
  a stale read-only attribute does not survive a re-copy.

## 6. Where this is enforced

| Concern             | Code                                                        |
| ------------------- | ----------------------------------------------------------- |
| Policy enum + parse | `crates/ferrocp-types/src/types.rs` (`OverwritePolicy`, `SymlinkMode`) |
| Decision            | `crates/ferrocp-io/src/policy.rs` (`decide_overwrite`)       |
| Symlink handling    | `crates/ferrocp-io/src/symlink.rs`                           |
| Contract entry      | `crates/ferrocp-io/src/policy.rs` (`apply_copy_contract`)    |
| Metadata/permission | `crates/ferrocp-io/src/metadata.rs` (`preserve_metadata`)    |
| Directory traversal | `crates/ferrocp-engine/src/executor.rs` (`copy_directory_recursive`) |
| CLI surface         | `crates/ferrocp-cli/src/main.rs` (`--overwrite`, `--symlinks`) |
| Python surface      | `crates/ferrocp-python/src/config.rs`                        |

Every engine (buffered, micro-file and parallel) calls `apply_copy_contract` -
and therefore `decide_overwrite` - before it opens the destination for writing,
so no engine can bypass the policy or the symlink mode. Metadata preservation is
likewise one shared helper (`metadata.rs`) used by all three engines, so it does
not depend on which engine the size heuristic picked.

## 7. Explicitly out of scope

These are **not** defined yet and may change. Do not rely on them:

- Hard links (a link is copied as a regular file).
- Sparse files (sparseness is not preserved).
- Cross-device / cross-filesystem behaviour beyond what the OS provides.
- Atomicity and rollback: a failed copy leaves partially written destinations.
- Ownership, ACLs and extended attributes.
- `CopyMode::mirror`.
