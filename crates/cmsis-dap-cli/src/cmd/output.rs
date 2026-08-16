use serde_json::Value;

/// Print a structured command result in JSON or human-readable form.
pub fn print_result(json_mode: bool, value: &Value) {
    if json_mode {
        println!(
            "{}",
            serde_json::to_string(value).unwrap_or_else(|_| "null".into())
        );
    } else {
        print_human(value);
    }
}

fn print_human(value: &Value) {
    if let Some(probes) = value.get("probes").and_then(|v| v.as_array()) {
        println!("{:<18} {:<14} {:<28} SERIAL", "ID", "VENDOR", "PRODUCT");
        for p in probes {
            println!(
                "{:<18} {:<14} {:<28} {}",
                p["id"].as_str().unwrap_or(""),
                p["vendor"].as_str().unwrap_or(""),
                p["product"].as_str().unwrap_or(""),
                p["serial"].as_str().unwrap_or(""),
            );
        }
        return;
    }
    if let Some(values) = value.get("values").and_then(|v| v.as_array()) {
        let address = value["address"].as_u64().unwrap_or(0);
        let width = match value["width"].as_str() {
            Some("u8") => 1u64,
            Some("u16") => 2,
            Some("u64") => 8,
            _ => 4,
        };
        let digits = (width * 2) as usize;
        for (i, v) in values.iter().enumerate() {
            println!(
                "0x{:08X}: 0x{:0width$X}",
                address + (i as u64) * width,
                v.as_u64().unwrap_or(0),
                width = digits
            );
        }
        return;
    }
    if let Some(register) = value.get("register").and_then(|v| v.as_str()) {
        println!(
            "{} = 0x{:X}",
            register,
            value["value"].as_u64().unwrap_or(0)
        );
        return;
    }
    if let Some(registers) = value.get("registers").and_then(|v| v.as_array()) {
        for r in registers {
            println!("{}", r.as_str().unwrap_or(""));
        }
        return;
    }
    if let Some(peripherals) = value.get("peripherals").and_then(|v| v.as_array()) {
        for p in peripherals {
            println!("{}", p.as_str().unwrap_or(""));
        }
        return;
    }
    if let Some(bps) = value.get("breakpoints").and_then(|v| v.as_array()) {
        for b in bps {
            println!("0x{:X}", b.as_u64().unwrap_or(0));
        }
        return;
    }
    if let Some(wps) = value.get("watchpoints").and_then(|v| v.as_array()) {
        for w in wps {
            println!(
                "0x{:X} ({})",
                w["address"].as_u64().unwrap_or(0),
                w["access"].as_str().unwrap_or("")
            );
        }
        return;
    }
    if value.get("verified").is_some() {
        let mismatches = value["mismatches"].as_array().map(|a| a.len()).unwrap_or(0);
        println!(
            "verified: {} ({} mismatches)",
            value["verified"].as_bool().unwrap_or(false),
            mismatches
        );
        return;
    }
    if let Some(state) = value.get("state").and_then(|v| v.as_str()) {
        let mut line = format!("state: {state}");
        if let Some(reason) = value["halt_reason"].as_str() {
            line.push_str(&format!(", halt_reason: {reason}"));
        }
        if let Some(pc) = value["pc"].as_u64() {
            line.push_str(&format!(", pc: 0x{pc:X}"));
        }
        println!("{line}");
        return;
    }
    if let Some(report) = value.get("report") {
        if let Some(results) = report["results"].as_array() {
            for r in results {
                println!(
                    "{} -> {}",
                    r["command"].as_str().unwrap_or(""),
                    r["status"].as_str().unwrap_or("")
                );
            }
            println!(
                "script {} ({} commands)",
                if report["ok"].as_bool().unwrap_or(false) {
                    "ok"
                } else {
                    "failed"
                },
                report["commands"].as_u64().unwrap_or(0)
            );
        }
        return;
    }
    if let Some(results) = value.get("results").and_then(|v| v.as_array()) {
        for r in results {
            println!(
                "{} -> {}",
                r["command"].as_str().unwrap_or(""),
                r["status"].as_str().unwrap_or("")
            );
        }
        println!(
            "script {} ({} commands)",
            if value["ok"].as_bool().unwrap_or(false) {
                "ok"
            } else {
                "failed"
            },
            value["commands"].as_u64().unwrap_or(0)
        );
        return;
    }
    if let Some(mismatches) = value.get("mismatches").and_then(|v| v.as_array()) {
        for m in mismatches {
            println!(
                "mismatch @0x{:X}: expected 0x{:X}, actual 0x{:X}",
                m["address"].as_u64().unwrap_or(0),
                m["expected"].as_u64().unwrap_or(0),
                m["actual"].as_u64().unwrap_or(0),
            );
        }
        return;
    }
    if let Some(obj) = value.as_object() {
        for (k, v) in obj {
            println!("{k}: {}", scalar_or_json(v));
        }
        return;
    }
    println!(
        "{}",
        serde_json::to_string_pretty(value).unwrap_or_default()
    );
}

fn scalar_or_json(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => "null".into(),
        other => serde_json::to_string(other).unwrap_or_default(),
    }
}
