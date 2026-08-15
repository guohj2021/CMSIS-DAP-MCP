use cmsis_dap_mcp::error::ErrorCode;
use cmsis_dap_mcp::security::{SecurityLevel, SecurityPolicy};

#[test]
fn read_only_always_allowed() {
    let p = SecurityPolicy { allow_destructive: false };
    assert!(p.check(SecurityLevel::ReadOnly).is_ok());
}

#[test]
fn destructive_blocked_by_default() {
    let p = SecurityPolicy { allow_destructive: false };
    let err = p.check(SecurityLevel::Destructive).unwrap_err();
    assert_eq!(err.code, ErrorCode::DestructiveDisabled);
}

#[test]
fn destructive_allowed_when_enabled() {
    let p = SecurityPolicy { allow_destructive: true };
    assert!(p.check(SecurityLevel::Destructive).is_ok());
}