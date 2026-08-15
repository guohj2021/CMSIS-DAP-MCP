# Tools

| Tool | Purpose | Level |
| --- | --- | --- |
| `list_probes` | Enumerate CMSIS-DAP probes | Read |
| `get_probe_info` | Probe firmware/serial details | Read |
| `connect` | Connect a target | Write |
| `disconnect` | Disconnect | Write |
| `get_target_info` | Core and memory info | Read |
| `read_memory` | Read memory by address/width | Read |
| `write_memory` | Write memory by address/width | Write |
| `read_core_register` | Read a core register | Read |
| `write_core_register` | Write a core register | Write |
| `halt` / `resume` / `step` | Core execution control | Write |
| `set_breakpoint` / `clear_breakpoints` / `list_breakpoints` | Hardware breakpoints | Write/Read |
| `reset` | Reset the target | Write |
| `read_dap` / `write_dap` | Raw DP/AP registers | Read/Write |
| `load_svd` / `list_peripherals` | Load SVD and list peripherals | Write/Read |
| `read_peripheral` / `write_peripheral` | Named register/field access | Read/Write |
| `erase_flash` / `program_flash` | Flash erase/program | Destructive |

Memory widths: `u8`, `u16`, `u32`, `u64`. DAP addresses use APSEL in bits 24-31 for AP access (for example `0x010000FC`).