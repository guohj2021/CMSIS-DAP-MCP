 # Getting Started

 This guide walks you through installation, hardware setup, and your first
 debug session --- from absolute zero. Every step includes a concrete command
 and the expected output.

 ## What you get

 CMSIS-DAP MCP ships two tools built on the same engine:

 - **cmsis-dap-mcp** --- a [Model Context Protocol](https://modelcontextprotocol.io)
   server that lets AI assistants (Codex, Claude Code, etc.) drive your debug
   probe and target chip directly.
 - **cmsis-dap-cli** --- a standalone command-line tool for humans, scripts
   and automation, no AI client needed.

 Both support:

 - Enumerating debug probes, connecting over SWD or JTAG to any Cortex-M chip
 - Reading/writing memory and core registers, halt/resume/step execution
 - Loading SVD files at runtime for named peripheral access
 - Programming flash from firmware files (elf/axf/bin/hex)
 - Running J-Link / OpenOCD style debug scripts
 - Non-invasive CPU snapshots (without resetting the target)
 - Remote TCP server and GDB debug server

 The CLI additionally provides live debugging: `watch` (variable polling),
 `rtt monitor` (SEGGER RTT logs), `evr monitor` (CMSIS-View Event
 Recorder) --- all over SWD/JTAG, no UART needed.

 ---

 ## Hardware you need

 ### Required

 1. **CMSIS-DAP debug probe**
    - Supports CMSIS-DAP v1 (HID) or v2 (WinUSB) protocol
    - Most commercial CMSIS-DAP compatible probes work
    - Connects to your PC via USB

 2. **Cortex-M development board**
    - Any ARM Cortex-M board with an SWD debug port
    - Supports M0, M0+, M3, M4, M7 --- all core variants
    - Connects to the probe via SWD wires

 3. **SWD wires**
    - Minimum 3 wires: **SWDIO**, **SWCLK**, **GND**
    - Connect probe SWD pins to the matching debug port on your board

 ### Optional

 4. **nRST reset wire**
    - Used for `under_reset` mode (locked or non-responsive targets)
    - Connect the probe's nRST pin to the board's reset pin

 ### Wiring diagram

 ```text
 Probe (CMSIS-DAP)          Board (Cortex-M)
 ┌─────────────┐           ┌─────────────┐
 │  SWDIO  ──────┼───────────┤  SWDIO      │
 │  SWCLK  ──────┼───────────┤  SWCLK      │
 │  GND    ──────┼───────────┤  GND        │
 │  nRST   ──────┼── (opt) ─┤  NRST       │
 └──────┬──────┘           └─────────────┘
        │ USB
     ┌──┴──┐
     │ PC  │
     └─────┘
 ```

 > **Tip**: Pin layouts vary by probe and board. Always check your hardware's
 > pinout diagram to match SWDIO/SWCLK/GND correctly.

 ---

 ## Files you need

 Features work in layers --- some need no extra files, others do:

 | Feature | File needed | Where to get it |
 | --- | --- | --- |
 | Basic debug (memory, registers, execution) | None --- works out of the box | --- |
 | Named peripheral access | SVD file | Chip vendor SDK or CMSIS-Pack |
 | Flash programming | Keil FLM flash algorithm file | IDE installation directory or chip vendor |
 | Symbol-level debug (watch/RTT/EVR) | Firmware ELF or AXF file | Your compiler output |

 **FLM files** are typically found under the Keil MDK `Flash/` directory,
 named like `TargetChip_64.FLM`.

 **SVD files** describe the chip's peripheral register layout, usually shipped
 with the chip's SDK or CMSIS-Pack, named like `TargetChip.svd`.

 > **Note**: This repository never bundles chip-specific data. All files are
 > provided by you at runtime.

 ---

 ## Environment setup

 ### Option A: npm (recommended)

 npm is the Node.js package manager. Both tools are published as npm packages.

 #### Windows

 ```bash
 # Install Node.js (includes npm) with winget
 winget install OpenJS.NodeJS.LTS

 # Or with scoop
 scoop install nodejs-lts
 ```

 Open a new terminal and verify:

 ```bash
 node --version    # Should show v18.x or later
 npm --version     # Should show 9.x or later
 ```

 #### Linux (Debian/Ubuntu)

 ```bash
 curl -fsSL https://deb.nodesource.com/setup_lts.x | sudo -E bash -
 sudo apt install -y nodejs
 ```

 #### Linux (Fedora/RHEL)

 ```bash
 sudo dnf install -y nodejs npm
 ```

 #### macOS

 ```bash
 brew install node
 ```

 ### Option B: Native binary (offline)

 If Node.js is not available, download the platform binary from
 [GitHub Releases](https://github.com/guohj2021/CMSIS-DAP-MCP/releases).
 Zero runtime dependencies.

 ### Option C: Build from source (developers)

 Install the Rust toolchain:

 ```bash
 curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
 cargo build --release --workspace
 ```

 ---

 ## Install the tools

 ### MCP server (for AI assistants)

 ```bash
 # Verify it runs (zero-install; downloads automatically on first run)
 npx -y cmsis-dap-mcp --help
 ```

 ### CLI (for humans)

 ```bash
 # Zero-install quick trial
 npx -y cmsis-dap-cli --help

 # Or install globally for direct command access
 npm install -g cmsis-dap-cli
 cmsis-dap-cli --help
 ```

 If you downloaded a native binary, add it to your PATH or call it by full
 path.

 ---

 ## Driver setup

 ### Windows

 - **CMSIS-DAP v1 (HID)**: usually driverless; plug in and it works.
 - **CMSIS-DAP v2 (WinUSB)**: requires a WinUSB driver.
   1. Download Zadig from <https://zadig.akeo.ie/>
   2. Plug in the probe and open Zadig
   3. Go to `Options -> List All Devices`
   4. Select your CMSIS-DAP device
   5. Replace the driver with **WinUSB**, click `Replace Driver`

 ### Linux

 Add a udev rule to allow non-root USB access:

 ```bash
 # Create a rule (replace xxxx/yyyy with your probe's VID/PID)
 echo 'SUBSYSTEM=="usb", ATTRS{idVendor}=="xxxx", ATTRS{idProduct}=="yyyy", MODE="0666"' \
   | sudo tee /etc/udev/rules.d/99-cmsis-dap.rules

 # Reload rules
 sudo udevadm control --reload-rules
 sudo udevadm trigger

 # Re-plug the probe
 ```

 > **Tip**: Find your probe's VID/PID in Windows Device Manager, or run
 > `lsusb` on Linux.

 ### macOS

 Usually works out of the box. If the probe is not recognized, check System
 Settings > Privacy & Security for any USB permission prompts.

 ---

 ## Step 1: Connect your hardware

 ### 1. Verify detection

 Plug in the CMSIS-DAP probe and open a terminal:

 ```bash
 cmsis-dap-cli list
 ```

 Expected output (probe id and product name vary by hardware):

 ```text
 CMSIS-DAP probes found:
   id        : 0123456789AB
   product   : CMSIS-DAP
   serial    : (none)
   protocols : SWD, JTAG
 ```

 If the list is empty, check [driver setup](#driver-setup) and the USB
 connection.

 ### 2. Connect to the target chip

 ```bash
 cmsis-dap-cli connect
 ```

 This auto-detects the target. For more detailed memory mapping, specify the
 chip name:

 ```bash
 cmsis-dap-cli --target STM32F030C8 connect
 ```

 Expected output:

 ```text
 target: {"ap_count":1, "core_count":1, "core_type":"Armv6m", ...,
          "memory_regions":[FLASH 0x08000000-0x08010000, SRAM 0x20000000-0x20002000]}
 ```

 ### 3. Read memory to verify

 ```bash
 cmsis-dap-cli read --address 0x20000000 --width u32 --count 4
 ```

 Expected output (values depend on the target's current memory contents):

 ```text
 address: 0x20000000, width: u32, count: 4
   0x20000000: 0x00000040
   0x20000004: 0x00000001
   0x20000008: 0x00000003
   0x2000000C: 0x00000000
 ```

 ### 4. Halt, read a register, resume

 ```bash
 cmsis-dap-cli halt
 cmsis-dap-cli reg get pc
 cmsis-dap-cli resume
 ```

 Expected output:

 ```text
 halted: true
 pc = 0x0800122A
 running: true
 ```

 **Congratulations!** You have connected to the target chip and performed
 basic memory and register operations.

 > **Tip**: Use `repl` to stay in an interactive session:
 >
 > ```bash
 > cmsis-dap-cli repl
 > # At the prompt: connect, halt, reg pc, resume
 > ```

 ---

 ## Next: MCP server setup

 If you use an AI assistant (Codex, Claude Code, or opencode), you can let it
 drive the probe directly. Add the MCP server configuration:

 ### Codex

 ```bash
 codex mcp add cmsis-dap -- npx -y cmsis-dap-mcp
 ```

 ### Claude Code

 ```bash
 claude mcp add --scope local cmsis-dap -- npx -y cmsis-dap-mcp
 ```

 ### opencode

 ```bash
 opencode mcp add cmsis-dap -- npx -y cmsis-dap-mcp
 ```

 After adding the server, restart the client. Then you can say in a chat:

 > List connected debug probes and connect to the target chip.

 The AI will call `list_probes`, `connect`, and other tools automatically.

 ---

 ## Next: Flash programming

 ### Prerequisites

 1. Obtain the FLM flash algorithm file for your chip
 2. Know the chip's Flash and SRAM address ranges (check the datasheet)

 ### CLI workflow

 ```bash
 # Step 1: Generate a target YAML from the FLM (one-time setup)
 cmsis-dap-cli chip generate \
   --flm /path/to/TargetChip.FLM \
   --flash-start 0x08000000 --flash-size 0x10000 \
   --sram-start 0x20000000 --sram-size 0x2000 \
   --name TargetChip --output TargetChip.yaml

 # Step 2: Connect with the generated YAML and program
 cmsis-dap-cli --target-yaml TargetChip.yaml connect
 cmsis-dap-cli flash erase --address 0x08000000 --size 0x10000
 cmsis-dap-cli flash program --address 0x08000000 --file firmware.hex --verify
 ```

 ### MCP workflow

 ```text
 define_chip {
   "flm": "/path/to/TargetChip.FLM",
   "flash_start": 0x08000000, "flash_size": 0x10000,
   "sram_start": 0x20000000, "sram_size": 0x2000,
   "core": "armv6m", "name": "TargetChip"
 }
 connect { "target": "TargetChip", "protocol": "swd" }
 program_flash { "address": 0x08000000, "path": "firmware.hex", "format": "hex", "verify": true }
 ```

 ### Enabling destructive mode

 Flash erase and program are destructive operations, disabled by default.
 Two ways to enable:

 - **At startup**: pass `--allow-destructive`
 - **At runtime**: call `update_config {"allow_destructive": true}` (no restart)

 ---

 ## Next: Named peripherals (SVD)

 SVD files describe the chip's peripheral register layout, letting you work
 with register names instead of raw addresses.

 ```bash
 # CLI
 cmsis-dap-cli --svd TargetChip.svd svd list
 cmsis-dap-cli --svd TargetChip.svd svd read GPIOA.ODR.ODR0
 cmsis-dap-cli --svd TargetChip.svd svd write GPIOA.ODR.ODR0 1
 ```

 ```text
 # MCP
 load_svd { "path": "/path/to/TargetChip.svd" }
 list_peripherals {}
 read_peripheral { "peripheral": "GPIOA", "register": "ODR", "field": "ODR0" }
 write_peripheral { "peripheral": "GPIOA", "register": "ODR", "field": "ODR0", "value": 1 }
 ```

 ---

 ## Next: Live debugging

 The CLI provides three live debugging features, all over SWD/JTAG --- no
 UART needed:

 ### Variable polling (watch)

 ```bash
 cmsis-dap-cli --elf firmware.axf watch counter --interval-ms 200 --count 0
 ```

 ### RTT logging

 ```bash
 cmsis-dap-cli --elf firmware.axf rtt monitor --channel 0 --count 0
 ```

 ### Event Recorder

 ```bash
 cmsis-dap-cli --elf firmware.axf evr monitor --count 0
 ```

 > **Note**: Live debugging requires the firmware ELF file (`--elf`), and the
 > target firmware must have initialized the corresponding component (SEGGER RTT
 > or CMSIS-View Event Recorder).

 ---

 ## Next steps

 - [Quickstart](./quickstart.md) --- MCP server quick setup
 - [AI client configuration](./ai-clients.md) --- detailed config for each AI client
 - [Tools](./tools.md) --- full MCP tool reference
 - [CLI](./cli.md) --- full CLI command reference
 - [Scripting](./scripting.md) --- J-Link / OpenOCD style scripts
 - [SWD and JTAG](./swd-jtag.md) --- protocol selection guide
 - [SVD and Flash](./svd-flash.md) --- peripheral access and flash workflows
 - [Troubleshooting](./troubleshooting.md) --- common issues and solutions
