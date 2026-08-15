# CMSIS-DAP MCP

An MCP server that lets AI assistants operate CMSIS-DAP debug probes to access Cortex-M chip resources over SWD/JTAG.

- Generic Cortex-M support: standard cores work without chip-specific adaptation.
- Named peripheral access: load any CMSIS-SVD file at runtime; chip files are never bundled.
- Flash programming: requires a target description with a CMSIS-Pack flash algorithm.
- Zero runtime dependencies for end users: one native binary, or install via npm.

This site covers setup, the tool reference, and security guidance.