# Security

- Read-only tools are always available.
- Write and debug-control tools are marked as writes; your MCP client governs
  approval.
- `erase_flash` and `program_flash` are destructive and disabled unless the
  server is started with `--allow-destructive`. Calling them without the flag
  returns `DestructiveDisabled`.

Flash erasing, option-byte changes, read-protection and debug unlock can
permanently damage a device or make it unrecoverable. Only enable destructive
mode when you explicitly intend to reprogram the target.

Logs are written to stderr (or `--log-file`) only, never to stdout, so they
cannot corrupt the MCP protocol stream.
