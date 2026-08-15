use crate::error::{ErrorCode, McpError};
use std::path::Path;

pub struct SvdSummary {
    pub name: String,
    pub peripherals: usize,
}

#[derive(Clone)]
pub struct SvdDatabase {
    name: String,
    peripherals: Vec<SvdPeripheral>,
}

#[derive(Clone)]
struct SvdPeripheral {
    name: String,
    base: u64,
    registers: Vec<SvdRegister>,
}

#[derive(Clone)]
struct SvdRegister {
    name: String,
    offset: u64,
    fields: Vec<SvdField>,
}

#[derive(Clone)]
struct SvdField {
    name: String,
    offset: u32,
    width: u32,
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
            .map(|p| SvdPeripheral {
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
                    .map(|r| SvdRegister {
                        name: r.name.clone(),
                        offset: r.address_offset as u64,
                        fields: r
                            .fields
                            .clone()
                            .unwrap_or_default()
                            .into_iter()
                            .map(|f| match f {
                                svd_parser::svd::MaybeArray::Single(info) => info,
                                svd_parser::svd::MaybeArray::Array(info, _) => info,
                            })
                            .map(|info| SvdField {
                                name: info.name.clone(),
                                offset: info.bit_offset(),
                                width: info.bit_width(),
                            })
                            .collect(),
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

    pub fn list_peripherals(&self) -> Vec<String> {
        self.peripherals.iter().map(|p| p.name.clone()).collect()
    }

    pub fn resolve(
        &self,
        peripheral: &str,
        register: &str,
        field: Option<&str>,
    ) -> Result<(u64, Option<(u32, u32)>), McpError> {
        let p = self
            .peripherals
            .iter()
            .find(|p| p.name.eq_ignore_ascii_case(peripheral))
            .ok_or_else(|| {
                McpError::new(
                    ErrorCode::InvalidArgument,
                    format!("peripheral {peripheral} not found"),
                )
            })?;
        let r = p
            .registers
            .iter()
            .find(|r| r.name.eq_ignore_ascii_case(register))
            .ok_or_else(|| {
                McpError::new(
                    ErrorCode::InvalidArgument,
                    format!("register {register} not found"),
                )
            })?;
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
