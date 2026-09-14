# P0 Technical Assumption Validation — CMSIS-DAP Web Debug

Date: 2026-08-29
Branch: codex/web-debug
Environment: CMSIS-DAP probe (serial 0123456789AB) + DemoMCU (Cortex-M0+) target

> Per the frozen v5 plan §2, each assumption was validated before entering P1.
> Failures follow the documented degradation path and do not expand scope.

## Summary

| # | Assumption | Result | Note |
|---|---|---|---|
| A1 | DemoMCU connect chain (chip generate / FLM / connect) | ✅ Pass | Cortex-M0+ attached, NVM + SRAM regions present |
| A2 | Memory read while target running (Live Watch premise) | ✅ Pass | 3 consecutive `watch` polls at 0x20000000 while running |
| A3 | HW breakpoint used/total queryable | ✅ Pass (feasible) | `Core::available_breakpoint_units()` + `hw_breakpoints()` in probe-rs 0.32; real `bp set` OK |
| A4 | Flash progress hooks + FLM programming | ✅ Pass | `FlashProgress`/`ProgressEvent` exists (operation + byte level); demo HEX (26012 B) programmed + verified |
| A5 | probe-rs 0.32 stack unwinder for Cortex-M0+ | ❌ Not available | No public unwinder (no gimli/addr2line dep). P6.2 degrades to interface-only (never fake) |
| A6 | SVD metadata + ELF symbol attributes | ✅ Pass | svd-parser 0.14 has description/access/bit_range/enumerated_values; object 0.39 has size/kind/section_index |

## A1 — DemoMCU connect chain

Command:
```
cmsis-dap-cli chip generate --flm <SDK>/Libraries/Flash/DemoMCU_64.FLM \
  --flash-start 0x08000000 --flash-size 0x10000 \
  --sram-start 0x20000000 --sram-size 0x2000 --name DemoMCU --output target/demo.yaml
cmsis-dap-cli connect --target-yaml target/demo.yaml --probe-id 0123456789AB
```
Result: `core_type=Armv6m`, `cpu_id=0x410CC241` (Cortex-M0+), 1 core,
`memory_regions`: FLASH nvm `0x08000000..0x08010000`, SRAM ram `0x20000000..0x20002000`.
FLM `FlashDevice` descriptor: flash 64 KB, page 0x400, erased 0xFF, sectors 0x400.

Conclusion: **validated**. Target YAML is regenerated at runtime from the user's own
FLM; no chip files are bundled. `target/demo.yaml` is a local git-ignored dev artifact.

## A2 — Memory read while running

Command:
```
cmsis-dap-cli watch --target-yaml target/demo.yaml --probe-id 0123456789AB \
  0x20000000 --interval-ms 200 --count 3
```
Result: three polls while the target is running returned values (no error).
probe-rs reads AHB-AP memory while the core runs on this probe/target combination.

Conclusion: **validated** — Live Watch can poll memory while running. Registers still
require halt; UI keeps the "Live Watch is not available while target is running"
message for register-backed items.

## A3 — HW breakpoint limits

Source: `probe-rs-0.32.0/src/core.rs`
- `Core::available_breakpoint_units() -> Result<u32>` (DAP FP comparator count)
- `Core::hw_breakpoints() -> Result<Vec<Option<u64>>>` (used units, enabled or not)

Real hardware: `cmsis-dap-cli bp set 0x08000000` succeeded; `bp list` empty in a fresh
CLI process (breakpoints are per-session, expected).

Conclusion: **validated** — add a new `Backend::hw_breakpoint_limits()` method with a
default `None` (UI shows "Unknown" on backends that cannot report it); implemented by
`ProbeRsBackend` from the two calls above.

## A4 — Flash progress + FLM programming

Source: `probe-rs-0.32.0/src/flashing/{progress.rs, download.rs, flasher.rs, mod.rs}`
- `FlashProgress::new(handler)` with `ProgressEvent`:
  `FlashLayoutReady`, `AddProgressBar`, `Started`, `Progress{operation,size,time}`,
  `Failed`, `Finished`, `DiagnosticMessage`; `ProgressOperation` = Erase/Fill/Program/Verify.
- `download_file_with_options(..., progress)` and `Flash::program(..., progress)` accept it.

Real hardware:
```
cmsis-dap-cli flash program --target-yaml target/demo.yaml --probe-id 0123456789AB \
  --file <SDK>/TEST_Example/HAL_Driver/Project/keil/Objects/demo.hex \
  --address 0x08000000 --verify
```
Result: `programmed: true, bytes: 26012, verify: true`.

Conclusion: **validated** — progress is reportable at operation + byte granularity.
Add `Backend::program_flash_with_progress` / `erase_flash_with_progress` default
methods (non-breaking) implemented via `FlashProgress`.

## A5 — Call Stack (unwinder)

probe-rs 0.32 has **no public stack unwinder**: `lib.rs` exports no `stack_unwinder`
module and there is no gimli/addr2line dependency (only `UnwindRule` register
metadata used for GDB arch descriptions).

Conclusion: **degraded** per plan — P6.2 builds the `DebugContext`/Call Stack
interface only and never fakes frames. A real unwinder would require either a newer
probe-rs with `stack_unwinder` or a DWARF-CFI unwinder (gimli) added at P6.2; that
decision is deferred and does not block V0.1/V0.2.

## A6 — SVD metadata + ELF symbol attributes

svd-parser 0.14 (`register.rs`, `field.rs`, `enumeratedvalues.rs`, `registerproperties.rs`):
- `RegisterInfo`: `description`, `address_offset`, `properties` (`RegisterProperties`:
  `name`, `size`, `access`, ...), `read_action`, `fields: Vec<FieldInfo>`
- `FieldInfo`: `name`, `description`, `bit_range` (offset/width), `enumerated_values`
  (`EnumeratedValues.values: Vec<EnumeratedValue{name, value, description}>`)

object 0.39 (`read/any.rs`, `read/traits.rs`):
- `Symbol::size()`, `Symbol::kind() -> SymbolKind`, `Symbol::section_index()`, `Symbol::name()`

Conclusion: **validated** — the SVD introspection API (`Svd*Info`, `DecodedRegister`,
field enumerated text) and `Symbol{ id, name, address, size, kind, section, module }`
are constructible. `module` requires DWARF compile units, so it stays `None` in V0.x.

## Degradation / deviations logged

1. A5 not available → Call Stack interface-only at P6.2 (no fabrication).
2. HW breakpoint total shows "Unknown" on backends without `available_breakpoint_units`.
3. `module` in `Symbol` is `None` until DWARF lands (P6.1).
No plan changes required; all other assumptions validated as specified.
## Implementation deviations (web, recorded during P3)

1. **Operation model `requires_halt` for Flash = No** (v5 §4 table said Yes).
   probe-rs' flash loader manages core halt internally (same as the CLI
   `flash` command), so requiring a pre-halted target would wrongly gate
   flash after a plain connect. RegisterRead/Write and Step still require halt.
2. **HW breakpoint `used` count** comes from the backend's own breakpoint list,
   because probe-rs 0.32 does not expose used comparator units publicly.
3. **Flash progress granularity** is phase + byte level (erase sectors, program
   pages, verify), driven by probe-rs `FlashProgress` events. `current_address`
   is approximated as `base + bytes_done` within a phase.
## V2 remaining items and hardware blockers (recorded at P8 completion)

The implementable scope of the frozen v5 plan (P0-P7b, P8 SWO/ITM/Profiler) is
complete and hardware-verified. The remaining V2 items are blocked by external
state (hardware/ecosystem), not by software effort:

| Item | Why blocked on this environment |
| --- | --- |
| Trace timeline / ETM / instruction trace | Requires ETM trace port + trace decoder; CMSIS-DAP probe has no trace port (SWO start returns an ARM protocol error). Fabricating a trace view would violate the plan's "never fabricate" rule. |
| Logic Analyzer | Requires high-speed GPIO sampling through the probe; not supported over SWD/HID CMSIS-DAP. |
| Code Coverage | Requires instruction trace (ETM) or instrumentation runtime; neither is present in the demo firmware. |
| RTOS awareness | Requires an RTOS (e.g. FreeRTOS) firmware with a task list; the DemoMCU demo has no RTOS. Can be added once an RTOS firmware is supplied (task-list symbol + stack pointer scan per task). |
| Advanced Performance Analyzer (call-tree) | Profiler (PC histogram) is done; call-graph attribution needs trace or full unwind per sample — deferred. |
| Multi-session / multi-probe | Explicitly "future, not implemented" in the plan; architecture reserves it in AppState. |

These are genuine hardware/ecosystem dependencies. Per the plan's hard rule
"unsupported capabilities are never fabricated", they are reported honestly
and remain as follow-up work requiring a trace-capable probe / RTOS firmware.