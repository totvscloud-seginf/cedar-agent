use fmt::Debug;
use std::borrow::Borrow;
use std::fmt;
use std::path::PathBuf;

use clap::Parser;
use log::{LevelFilter, info};

use serde::{Deserialize, Serialize};

#[derive(Parser, Serialize, Deserialize, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Config {
    
    /// Sets an authentication
    #[arg(short, long)]
    pub authentication: Option<String>,
    
    /// Sets an address
    #[arg(long)]
    pub addr: Option<String>,
    
    /// Sets a port number
    #[arg(short, long)]
    pub port: Option<u16>,

    /// Sets a log level
    #[arg(short, long, value_enum)]
    pub log_level: Option<LevelFilter>,

    /// Sets a data json file
    #[arg(short, long)]
    pub data: Option<PathBuf>,

    /// Sets a policies json file
    #[arg(long)]
    pub policies: Option<PathBuf>,

    /// Watch file changes and reload data and policies
    #[arg(short)]
    pub file_watcher: Option<bool>,
}

impl Into<rocket::figment::Figment> for &Config {
    fn into(self) -> rocket::figment::Figment {
        let mut config = rocket::Config::figment();
        if let Some(authentication) = self.authentication.borrow() {
            config = config.merge(("authentication", authentication));
        }
        if let Some(addr) = self.addr.borrow() {
            config = config.merge(("address", addr));
        }
        if let Some(port) = self.port.borrow() {
            config = config.merge(("port", port));
        } else {
            config = config.merge(("port", 8180))
        }
        if let Some(data) = self.data.borrow() {
            config = config.merge(("data", data));
        }
        if let Some(policy) = self.policies.borrow() {
            config = config.merge(("policy", policy));
        }
        if let Some(file_watcher) = self.file_watcher.borrow() {
            config = config.merge(("file_watcher", file_watcher));
        }

        config
    }
}

impl Config {
    fn new() -> Self {
        Config {
            authentication: None,
            addr: None,
            port: None,
            log_level: None,
            data: None,
            policies: None,
            file_watcher: None,
        }
    }

    fn merge(configs: Vec<Config>) -> Config {
        let mut config = Config::new();
        for c in configs {
            config.authentication = c.authentication.or(config.authentication);
            config.addr = c.addr.or(config.addr);
            config.port = c.port.or(config.port);
            config.log_level = c.log_level.or(config.log_level);
            config.data = c.data.or(config.data);
            config.policies = c.policies.or(config.policies);
            config.file_watcher = c.file_watcher.or(config.file_watcher);
        }

        config
    }

    fn from_args() -> Self {
        Self::parse()
    }

    fn from_env() -> Self {
        match envy::prefixed("CEDAR_AGENT_").from_env() {
            Ok(env) => {
                info!("Loaded config from environment variables");
                env
            },
            Err(err) => {                
                println!("Failed to load config from environment variables: {}", err);
                Self::new()
            },
        }
    }
}

pub fn init() -> Config {
    let args = Config::from_args();
    let env = Config::from_env();

    Config::merge(vec![args, env])
}
