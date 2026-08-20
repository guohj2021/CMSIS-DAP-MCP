# Security

- Read-only tools are always available.
- Write and debug-control tools are marked as writes; your MCP client governs
  approval.
- `erase_flash` and `program_flash` are destructive and disabled by
  default. Enable them either at startup with `--allow-destructive` **or**
  at runtime via `update_config` with `allow_destructive: true`. Calling
  them while disabled returns `DestructiveDisabled`.

Flash erasing, option-byte changes, read-protection and debug unlock can
permanently damage a device or make it unrecoverable. Only enable destructive
mode when you explicitly intend to reprogram the target.

Logs are written to stderr (or `--log-file`) only, never to stdout, so they
cannot corrupt the MCP protocol stream.

`read_memory` with a `path` argument writes an export file (bin/hex) on the
host at the path you provide; `run_script` may read and write files on the
host too. This uses the same trust model as `load_svd`: the paths come from
the user and are executed on the machine running the server.
