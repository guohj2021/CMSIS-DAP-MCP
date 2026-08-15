# Security

- Read-only tools are always available.
- Write and debug-control tools are marked as writes; the MCP client governs approval.
- `erase_flash` and `program_flash` are destructive and disabled unless the server was started with `--allow-destructive`.

Flash erasing, option-byte changes, read-protection, and debug unlock can permanently damage a device or make it unrecoverable. Only enable destructive mode when you explicitly intend to reprogram the target.