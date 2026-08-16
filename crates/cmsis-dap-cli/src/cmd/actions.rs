use super::{
    parse_register, parse_svd_target, parse_width, BpAction, BpArgs, CliError, DapAction, DapArgs,
    FlashAction, FlashArgs, ReadArgs, RegAction, RegArgs, ResetArgs, SvdAction, SvdArgs,
    VerifyArgs, WpAction, WpArgs, WriteArgs,
};
use cmsis_dap_core::backend::{AccessWidth, ExportFormat, ImageFileFormat, ResetMode, WatchAccess};
use cmsis_dap_core::session::SessionManager;
use serde_json::{json, Value};

pub fn read(session: &mut SessionManager, a: &ReadArgs) -> Result<Value, CliError> {
    if let Some(path) = &a.output {
        let format = ExportFormat::parse(&a.format).ok_or_else(|| {
            CliError::InvalidArgument(format!(
                "export format must be bin or hex, got {}",
                a.format
            ))
        })?;
        if a.count == 0 {
            return Err(CliError::InvalidArgument(
                "export count (bytes) must be greater than zero".into(),
            ));
        }
        let bytes = session
            .backend()
            .export_memory(path, format, a.address, a.count as u64)?;
        return Ok(json!({
            "exported": true,
            "path": path.display().to_string(),
            "format": format.as_str(),
            "bytes": bytes,
        }));
    }
    let width = parse_width(&a.width)?;
    let values = session.backend().read_memory(a.address, width, a.count)?;
    Ok(json!({ "address": a.address, "width": a.width, "values": values }))
}

pub fn write(session: &mut SessionManager, a: &WriteArgs) -> Result<Value, CliError> {
    let width = parse_width(&a.width)?;
    session
        .backend()
        .write_memory(a.address, width, &a.values)?;
    Ok(json!({
        "address": a.address,
        "width": a.width,
        "written": true,
        "values": a.values,
    }))
}

pub fn verify(session: &mut SessionManager, a: &VerifyArgs) -> Result<Value, CliError> {
    let width = parse_width(&a.width)?;
    let report = session
        .backend()
        .verify_memory(a.address, width, &a.values)?;
    Ok(json!({ "verified": report.verified, "mismatches": report.mismatches }))
}

pub fn regs(session: &mut SessionManager) -> Result<Value, CliError> {
    let registers = session.backend().list_core_registers()?;
    Ok(json!({ "registers": registers }))
}

pub fn reg(session: &mut SessionManager, a: &RegArgs) -> Result<Value, CliError> {
    match &a.action {
        RegAction::Get(g) => {
            let reg = parse_register(&g.register)?;
            let value = session.backend().read_core_register(&reg)?;
            Ok(json!({ "register": g.register, "value": value }))
        }
        RegAction::Set(s) => {
            let reg = parse_register(&s.register)?;
            session.backend().write_core_register(&reg, s.value)?;
            Ok(json!({
                "register": s.register,
                "value": s.value,
                "written": true,
            }))
        }
    }
}

pub fn status(session: &mut SessionManager) -> Result<Value, CliError> {
    let info = session.backend().get_core_status()?;
    Ok(json!({
        "state": info.state,
        "halt_reason": info.halt_reason,
        "pc": info.pc,
    }))
}

pub fn reset(session: &mut SessionManager, a: &ResetArgs) -> Result<Value, CliError> {
    let mode = match a.mode.as_str() {
        "run" => ResetMode::Run,
        "halt" => ResetMode::Halt,
        other => {
            return Err(CliError::InvalidArgument(format!(
                "reset mode must be run or halt, got {other}"
            )))
        }
    };
    session.backend().reset(mode)?;
    Ok(json!({ "reset": true, "mode": a.mode }))
}

pub fn bp(session: &mut SessionManager, a: &BpArgs) -> Result<Value, CliError> {
    match &a.action {
        BpAction::Set(s) => {
            session.backend().set_breakpoint(s.address)?;
            Ok(json!({ "breakpoint": s.address, "set": true }))
        }
        BpAction::List => {
            let list = session.backend().list_breakpoints()?;
            Ok(json!({ "breakpoints": list }))
        }
        BpAction::Clear => {
            session.backend().clear_breakpoints()?;
            Ok(json!({ "cleared": true }))
        }
    }
}

pub fn wp(session: &mut SessionManager, a: &WpArgs) -> Result<Value, CliError> {
    match &a.action {
        WpAction::Set(s) => {
            let access = WatchAccess::parse(&s.access).ok_or_else(|| {
                CliError::InvalidArgument(format!(
                    "access must be read, write or rw, got {}",
                    s.access
                ))
            })?;
            session.backend().set_watchpoint(s.address, access)?;
            Ok(json!({
                "watchpoint": { "address": s.address, "access": s.access },
                "set": true,
            }))
        }
        WpAction::List => {
            let list = session.backend().list_watchpoints()?;
            Ok(json!({ "watchpoints": list }))
        }
        WpAction::Clear => {
            session.backend().clear_watchpoints()?;
            Ok(json!({ "cleared": true }))
        }
    }
}

pub fn dap(session: &mut SessionManager, a: &DapArgs) -> Result<Value, CliError> {
    match &a.action {
        DapAction::Read(r) => {
            let value = session.backend().read_dap(r.address)?;
            Ok(json!({ "address": r.address, "value": value }))
        }
        DapAction::Write(w) => {
            session.backend().write_dap(w.address, w.value)?;
            Ok(json!({
                "address": w.address,
                "value": w.value,
                "written": true,
            }))
        }
    }
}

pub fn svd(session: &mut SessionManager, a: &SvdArgs) -> Result<Value, CliError> {
    match &a.action {
        SvdAction::List => {
            let db = session.svd()?;
            Ok(json!({ "peripherals": db.list_peripherals() }))
        }
        SvdAction::Read(r) => {
            let (peripheral, register, field) = parse_svd_target(&r.target)?;
            let (addr, mask) = session
                .svd()?
                .resolve(&peripheral, &register, field.as_deref())?;
            let value = session
                .backend()
                .read_memory(addr, AccessWidth::U32, 1)?
                .first()
                .copied()
                .unwrap_or(0);
            let result = match mask {
                Some((mask, offset)) => (value & mask as u64) >> offset,
                None => value,
            };
            Ok(json!({
                "peripheral": peripheral,
                "register": register,
                "field": field,
                "address": addr,
                "value": result,
            }))
        }
        SvdAction::Write(w) => {
            let (peripheral, register, field) = parse_svd_target(&w.target)?;
            let (addr, mask) = session
                .svd()?
                .resolve(&peripheral, &register, field.as_deref())?;
            match mask {
                Some((mask, offset)) => {
                    let current = session
                        .backend()
                        .read_memory(addr, AccessWidth::U32, 1)?
                        .first()
                        .copied()
                        .unwrap_or(0);
                    let updated = (current & !((mask as u64) << offset))
                        | ((w.value & mask as u64) << offset);
                    session
                        .backend()
                        .write_memory(addr, AccessWidth::U32, &[updated])?;
                }
                None => session
                    .backend()
                    .write_memory(addr, AccessWidth::U32, &[w.value])?,
            }
            Ok(json!({
                "peripheral": peripheral,
                "register": register,
                "field": field,
                "address": addr,
                "value": w.value,
                "written": true,
            }))
        }
    }
}

pub fn flash(session: &mut SessionManager, a: &FlashArgs, yes: bool) -> Result<Value, CliError> {
    match &a.action {
        FlashAction::Erase(e) => {
            super::confirm_destructive(yes, "flash erase")?;
            session.backend().erase_flash(e.address, e.size)?;
            Ok(json!({
                "erased": true,
                "address": e.address,
                "size": e.size,
            }))
        }
        FlashAction::Program(p) => {
            super::confirm_destructive(yes, "flash program")?;
            let format = match &p.format {
                Some(f) => ImageFileFormat::parse(f).ok_or_else(|| {
                    CliError::InvalidArgument(format!(
                        "file format must be elf/axf/bin/hex, got {f}"
                    ))
                })?,
                None => ImageFileFormat::from_extension(&p.file).ok_or_else(|| {
                    CliError::InvalidArgument(format!(
                        "cannot infer file format from extension {}",
                        p.file.display()
                    ))
                })?,
            };
            let bytes = session
                .backend()
                .program_file(&p.file, format, p.address, p.verify)?;
            Ok(json!({
                "programmed": true,
                "path": p.file.display().to_string(),
                "format": format.as_str(),
                "bytes": bytes,
                "verify": p.verify,
            }))
        }
    }
}
