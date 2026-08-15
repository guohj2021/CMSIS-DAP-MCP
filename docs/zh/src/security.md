# 安全

- 只读工具始终可用。
- 写与调试控制工具标记为写操作，由你的 MCP 客户端审批策略决定。
- `erase_flash` 与 `program_flash` 为破坏性工具，默认禁用，仅当以
  `--allow-destructive` 启动时可用；未启用时调用返回 `DestructiveDisabled`。

Flash 擦除、Option 字节修改、读保护与调试解锁可能导致设备永久损坏或不可恢复。
只有明确要重新编程目标时才启用破坏性模式。

日志只写入 stderr（或 `--log-file`），绝不写入 stdout，因此不会污染 MCP
协议流。
