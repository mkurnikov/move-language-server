use anyhow::{Context, Result};

use lang::{dfina, libra};

use shared::errors::CompilerError;
use shared::results::ResourceChange;

use shared::bech32::bech32_into_libra;

use std::str::FromStr;
use utils::{FilesSourceText, MoveFilePath};

#[derive(serde::Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DialectName {
    Libra,
    DFinance,
}

impl DialectName {
    pub fn get_dialect(&self) -> Box<dyn Dialect> {
        match self {
            DialectName::Libra => Box::new(MoveDialect::default()),
            DialectName::DFinance => Box::new(DFinanceDialect::default()),
        }
    }
}

impl FromStr for DialectName {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "libra" => Ok(DialectName::Libra),
            "dfinance" => Ok(DialectName::DFinance),
            _ => Err(anyhow::format_err!("Invalid dialect {:?}", s)),
        }
    }
}

pub trait Dialect {
    fn name(&self) -> &str;

    fn zero_address(&self) -> &str;

    fn normalize_account_address(&self, addr: &str) -> Result<String>;

    fn check_with_compiler(
        &self,
        current: (MoveFilePath, String),
        deps: Vec<(MoveFilePath, String)>,
        raw_sender_string: &str,
    ) -> Result<(), Vec<CompilerError>>;

    fn compile_and_run(
        &self,
        script: (MoveFilePath, String),
        deps: &[(MoveFilePath, String)],
        raw_sender_string: String,
        genesis_changes: Vec<ResourceChange>,
        args: Vec<String>,
    ) -> Result<Vec<ResourceChange>>;

    fn print_compiler_errors_and_exit(
        &self,
        files: FilesSourceText,
        errors: Vec<CompilerError>,
    ) -> !;
}

#[derive(Default)]
pub struct MoveDialect;

impl Dialect for MoveDialect {
    fn name(&self) -> &str {
        "libra"
    }

    fn zero_address(&self) -> &str {
        "0x0000000000000000"
    }

    fn normalize_account_address(&self, addr: &str) -> Result<String> {
        libra::parse_account_address(&addr).map(|address| format!("0x{}", address))
    }

    fn check_with_compiler(
        &self,
        current: (MoveFilePath, String),
        deps: Vec<(MoveFilePath, String)>,
        raw_sender_string: &str,
    ) -> Result<(), Vec<CompilerError>> {
        libra::check_with_compiler(current, deps, raw_sender_string.to_string())
    }

    fn compile_and_run(
        &self,
        script: (MoveFilePath, String),
        deps: &[(MoveFilePath, String)],
        raw_sender_string: String,
        genesis_changes: Vec<ResourceChange>,
        args: Vec<String>,
    ) -> Result<Vec<ResourceChange>> {
        let genesis_write_set = libra::resources::changes_into_writeset(genesis_changes)
            .with_context(|| "Provided genesis serialization error")?;
        libra::executor::compile_and_run(script, deps, raw_sender_string, genesis_write_set, args)
    }

    fn print_compiler_errors_and_exit(
        &self,
        files: FilesSourceText,
        errors: Vec<CompilerError>,
    ) -> ! {
        libra::report_errors(files, errors)
    }
}

#[derive(Default)]
pub struct DFinanceDialect;

impl Dialect for DFinanceDialect {
    fn name(&self) -> &str {
        "dfinance"
    }

    fn zero_address(&self) -> &str {
        "0x00000000000000000000"
    }

    fn normalize_account_address(&self, addr: &str) -> Result<String> {
        bech32_into_libra(addr).with_context(|| {
            format!("Address {:?} is not a valid wallet1 bech32 address", addr)
        })?;
        Ok(addr.to_string())
    }

    fn check_with_compiler(
        &self,
        current: (MoveFilePath, String),
        deps: Vec<(MoveFilePath, String)>,
        raw_sender_string: &str,
    ) -> Result<(), Vec<CompilerError>> {
        dfina::check_with_compiler(current, deps, raw_sender_string.to_string())
    }

    fn compile_and_run(
        &self,
        script: (MoveFilePath, String),
        deps: &[(MoveFilePath, String)],
        raw_sender_string: String,
        genesis_changes: Vec<ResourceChange>,
        args: Vec<String>,
    ) -> Result<Vec<ResourceChange>> {
        let genesis_write_set = dfina::resources::changes_into_writeset(genesis_changes)
            .with_context(|| "Provided genesis serialization error")?;
        dfina::executor::compile_and_run(script, deps, raw_sender_string, genesis_write_set, args)
    }

    fn print_compiler_errors_and_exit(
        &self,
        files: FilesSourceText,
        errors: Vec<CompilerError>,
    ) -> ! {
        dfina::report_errors(files, errors)
    }
}
