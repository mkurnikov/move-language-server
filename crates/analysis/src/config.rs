use std::path::PathBuf;

use core::fmt;
use dialects::{Dialect, DialectName};

use serde::export::fmt::Debug;
use serde::export::Formatter;
use serde::Deserialize;
use utils::io;

#[derive(Clone)]
pub struct Config {
    pub dialect_name: DialectName,
    pub stdlib_folder: Option<PathBuf>,
    pub modules_folders: Vec<PathBuf>,
    pub raw_sender_address: String,
}

impl Debug for Config {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Config")
            .field("dialect", &self.dialect_name)
            .field("stdlib_folder", &self.stdlib_folder)
            .field("module_folders", &self.modules_folders)
            .field("sender_address", &self.raw_sender_address)
            .finish()
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            dialect_name: DialectName::Libra,
            stdlib_folder: None,
            modules_folders: vec![],
            raw_sender_address: "0x0".to_string(),
        }
    }
}

impl Config {
    fn log_available_module_files(&self) {
        let stdlib_modules = self
            .stdlib_folder
            .as_ref()
            .map(io::iter_over_move_files)
            .unwrap_or_else(Vec::new);
        log::info!(
            "available stdlib modules (from {:?}) = {:#?}",
            self.stdlib_folder,
            stdlib_modules
        );

        for folder in &self.modules_folders {
            let files = io::iter_over_move_files(folder);
            log::info!("third party modules (from {:?}) = {:#?}", folder, files);
        }
    }

    pub fn dialect(&self) -> Box<dyn Dialect> {
        self.dialect_name.get_dialect()
    }

    pub fn raw_sender_address(&self) -> String {
        if &self.raw_sender_address == "0x0" {
            self.dialect().zero_address().to_string()
        } else {
            self.dialect()
                .normalize_account_address(&self.raw_sender_address)
                .unwrap()
        }
    }

    pub fn update(&mut self, value: &serde_json::Value) {
        log::info!("Passed configuration = {:#}", value);

        set(value, "/dialect", &mut self.dialect_name);
        self.stdlib_folder = match get::<PathBuf>(value, "/stdlib_folder") {
            None => {
                log::error!("\"stdlib_folder\" not specified or invalid, standard library won't be loaded");
                None
            }
            Some(folder) => {
                if !folder.exists() {
                    log::error!(
                        "Invalid configuration: {:?}  does not exist, standard library won't be loaded", 
                        folder.into_os_string()
                    );
                    None
                } else {
                    Some(folder)
                }
            }
        };
        self.modules_folders = match get::<Vec<PathBuf>>(value, "/modules_folders") {
            None => vec![],
            Some(folders) => folders
                .into_iter()
                .filter(|folder| {
                    if !folder.exists() {
                        log::error!(
                            "Modules folder {:?} does not exist, skipping it",
                            folder.to_str().unwrap()
                        );
                        false
                    } else {
                        true
                    }
                })
                .collect(),
        };
        self.raw_sender_address = match get(value, "/sender_address") {
            None => {
                log::info!("Using default account address 0x0");
                "0x0".to_string()
            }
            Some(address) => match self.dialect().normalize_account_address(address) {
                Ok(parsed_address) => parsed_address,
                Err(error) => {
                    log::error!("Invalid sender_address string: {:?}", error);
                    log::info!("Using default account address 0x0");
                    "0x0".to_string()
                }
            },
        };

        log::info!("Config updated to = {:#?}", self);
        self.log_available_module_files();

        fn get<'a, T: Deserialize<'a>>(value: &'a serde_json::Value, pointer: &str) -> Option<T> {
            value
                .pointer(pointer)
                .and_then(|it| T::deserialize(it).ok())
        }

        fn set<'a, T: Deserialize<'a>>(
            value: &'a serde_json::Value,
            pointer: &str,
            slot: &mut T,
        ) {
            if let Some(new_value) = get(value, pointer) {
                *slot = new_value
            }
        }
    }
}
