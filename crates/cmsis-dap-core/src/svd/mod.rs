//! SVD parsing and named peripheral/register/field resolution (Peripheral
//! Introspection API, frozen v5 §8).

use crate::error::{ErrorCode, McpError};
use serde::Serialize;
use std::path::Path;

pub struct SvdSummary {
    pub name: String,
    pub peripherals: usize,
}

/// One enumerated value of a field (from `<enumeratedValue>`).
#[derive(Debug, Clone, Serialize)]
pub struct SvdEnumValue {
    pub name: String,
    pub value: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// A bit field of a register.
#[derive(Debug, Clone, Serialize)]
pub struct SvdFieldInfo {
    pub name: String,
    pub offset: u32,
    pub width: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Enumerated values (empty when the SVD does not define any).
    pub values: Vec<SvdEnumValue>,
}

/// A register of a peripheral.
#[derive(Debug, Clone, Serialize)]
pub struct SvdRegisterInfo {
    pub name: String,
    pub offset: u64,
    /// Register size in bits (default 32 when unknown).
    pub size_bits: u32,
    /// "read-only" | "write-only" | "read-write" | unknown.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub fields: Vec<SvdFieldInfo>,
}

/// A peripheral with its base address and registers.
#[derive(Debug, Clone, Serialize)]
pub struct SvdPeripheralInfo {
    pub name: String,
    pub base: u64,
    pub registers: Vec<SvdRegisterInfo>,
}

/// One decoded field of a register value.
#[derive(Debug, Clone, Serialize)]
pub struct DecodedField {
    pub name: String,
    pub value: u32,
    pub offset: u32,
    pub width: u32,
    /// Matching enumerated value name, when present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// A register value decoded into its bit fields.
#[derive(Debug, Clone, Serialize)]
pub struct DecodedRegister {
    pub value: u32,
    pub fields: Vec<DecodedField>,
}

#[derive(Clone)]
pub struct SvdDatabase {
    name: String,
    peripherals: Vec<SvdPeripheralInfo>,
}

impl SvdDatabase {
    pub fn load(path: &Path) -> Result<Self, McpError> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| McpError::new(ErrorCode::SvdNotLoaded, e.to_string()))?;
        let parsed = svd_parser::parse(&text)
            .map_err(|e| McpError::new(ErrorCode::SvdNotLoaded, e.to_string()))?;
        let peripherals = parsed
            .peripherals
            .iter()
            .map(|p| SvdPeripheralInfo {
                name: p.name.clone(),
                base: p.base_address,
                registers: p
                    .registers
                    .clone()
                    .unwrap_or_default()
                    .into_iter()
                    .filter_map(|rc| match rc {
                        svd_parser::svd::RegisterCluster::Register(r) => Some(r),
                        svd_parser::svd::RegisterCluster::Cluster(_) => None,
                    })
                    .map(|r| {
                        let props = &r.properties;
                        SvdRegisterInfo {
                            name: r.name.clone(),
                            offset: r.address_offset as u64,
                            size_bits: props.size.unwrap_or(32),
                            access: props.access.as_ref().map(|a| {
                                match a {
                                    svd_parser::svd::Access::ReadOnly => "read-only",
                                    svd_parser::svd::Access::WriteOnly => "write-only",
                                    svd_parser::svd::Access::ReadWrite => "read-write",
                                    svd_parser::svd::Access::WriteOnce => "write-once",
                                    svd_parser::svd::Access::ReadWriteOnce => "read-write-once",
                                }
                                .to_string()
                            }),
                            description: r.description.clone(),
                            fields: r
                                .fields
                                .clone()
                                .unwrap_or_default()
                                .into_iter()
                                .map(|f| match f {
                                    svd_parser::svd::MaybeArray::Single(info) => info,
                                    svd_parser::svd::MaybeArray::Array(info, _) => info,
                                })
                                .map(|info| {
                                    let values = info
                                        .enumerated_values
                                        .iter()
                                        .flat_map(|ev| ev.values.iter())
                                        .filter_map(|v| {
                                            let value = v.value? as u32;
                                            Some(SvdEnumValue {
                                                name: v.name.clone(),
                                                value,
                                                description: v.description.clone(),
                                            })
                                        })
                                        .collect::<Vec<SvdEnumValue>>();
                                    SvdFieldInfo {
                                        name: info.name.clone(),
                                        offset: info.bit_offset(),
                                        width: info.bit_width(),
                                        description: info.description.clone(),
                                        values,
                                    }
                                })
                                .collect(),
                        }
                    })
                    .collect(),
            })
            .collect();
        Ok(Self {
            name: parsed.name.clone(),
            peripherals,
        })
    }

    pub fn summary(&self) -> SvdSummary {
        SvdSummary {
            name: self.name.clone(),
            peripherals: self.peripherals.len(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn list_peripherals(&self) -> Vec<String> {
        self.peripherals.iter().map(|p| p.name.clone()).collect()
    }

    /// All peripherals with full metadata (Peripheral Introspection API).
    pub fn peripherals(&self) -> &[SvdPeripheralInfo] {
        &self.peripherals
    }

    pub fn get_peripheral(&self, peripheral: &str) -> Option<&SvdPeripheralInfo> {
        self.peripherals
            .iter()
            .find(|p| p.name.eq_ignore_ascii_case(peripheral))
    }

    pub fn get_register(&self, peripheral: &str, register: &str) -> Option<&SvdRegisterInfo> {
        self.get_peripheral(peripheral)?
            .registers
            .iter()
            .find(|r| r.name.eq_ignore_ascii_case(register))
    }

    /// Decode a register value into bit fields with enumerated text
    /// (server-side decoding; the UI only displays).
    pub fn decode_register(
        &self,
        peripheral: &str,
        register: &str,
        value: u32,
    ) -> Result<DecodedRegister, McpError> {
        let r = self.get_register(peripheral, register).ok_or_else(|| {
            McpError::new(
                ErrorCode::InvalidArgument,
                format!("register {peripheral}.{register} not found"),
            )
        })?;
        let fields = r
            .fields
            .iter()
            .map(|f| {
                let mask = if f.width >= 32 {
                    u32::MAX
                } else {
                    (1u32 << f.width) - 1
                };
                let field_value = (value >> f.offset) & mask;
                let text = f
                    .values
                    .iter()
                    .find(|v| v.value == field_value)
                    .map(|v| v.name.clone());
                DecodedField {
                    name: f.name.clone(),
                    value: field_value,
                    offset: f.offset,
                    width: f.width,
                    text,
                    description: f.description.clone(),
                }
            })
            .collect();
        Ok(DecodedRegister { value, fields })
    }

    /// Resolve a named register (or field) to an address and optional field mask.
    pub fn resolve(
        &self,
        peripheral: &str,
        register: &str,
        field: Option<&str>,
    ) -> Result<(u64, Option<(u32, u32)>), McpError> {
        let r = self.get_register(peripheral, register).ok_or_else(|| {
            McpError::new(
                ErrorCode::InvalidArgument,
                format!("register {register} not found in {peripheral}"),
            )
        })?;
        let p = self
            .get_peripheral(peripheral)
            .expect("register implies peripheral");
        let addr = p.base + r.offset;
        match field {
            None => Ok((addr, None)),
            Some(name) => {
                let f = r
                    .fields
                    .iter()
                    .find(|f| f.name.eq_ignore_ascii_case(name))
                    .ok_or_else(|| {
                        McpError::new(
                            ErrorCode::InvalidArgument,
                            format!("field {name} not found"),
                        )
                    })?;
                let mask = if f.width >= 32 {
                    u32::MAX
                } else {
                    (1u32 << f.width) - 1
                };
                Ok((addr, Some((mask, f.offset))))
            }
        }
    }
}
