# CMSIS-DAP MCP

An MCP (Model Context Protocol) server that lets AI assistants operate
CMSIS-DAP debug probes and access Cortex-M chip resources over **SWD** or
**JTAG**.

- Generic Cortex-M support: standard cores work without chip-specific
  adaptation.
- Named peripheral access: load any CMSIS-SVD file at runtime; chip files are
  never bundled.
- Flash programming: requires a target description with a CMSIS-Pack flash
  algorithm.
- Zero runtime dependencies for end users: one native binary, or install via
  npm.
- Cross-platform: Windows / Linux / macOS.

This site covers setup, AI client configuration, the full tool reference,
SWD/JTAG selection, SVD/Flash workflows and security guidance.

Chinese documentation: <https://guohj2021.github.io/CMSIS-DAP-MCP/zh/>
