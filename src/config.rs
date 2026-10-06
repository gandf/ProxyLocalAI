use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize, Clone, Debug, Default)]
pub struct RequestReplacement {
    #[serde(alias = "from", alias = "text_to_replace", alias = "find", alias = "old", alias = "search")]
    pub from: String,
    #[serde(alias = "to", alias = "text_replacing", alias = "replace", alias = "with", alias = "replacement")]
    pub to: String,
    #[serde(default)]
    #[serde(alias = "first_only", alias = "replace_once")]
    pub first_only: bool,
    #[serde(default)]
    #[serde(alias = "from_end", alias = "search_from_end", alias = "last", alias = "match_end")]
    pub from_end: bool,
}

#[derive(Deserialize, Clone)]
#[serde(default)]
pub struct Config {
    pub listen: String,
    pub target: String,
    pub log_enabled: bool,
    pub log_file: String,
    pub log_max_bytes: u64,
    pub log_max_files: u32,
    pub max_body_log_bytes: usize,
    pub max_retries: u32,
    pub retry_delay_ms: u64,
    pub request_timeout_s: u64,
    pub request_replacements: Vec<RequestReplacement>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            listen: "127.0.0.1:8081".into(),
            target: "127.0.0.1:8080".into(),
            log_enabled: true,
            log_file: "proxyia.log".into(),
            log_max_bytes: 10 * 1024 * 1024,
            log_max_files: 5,
            max_body_log_bytes: 64 * 1024,
            max_retries: 2,
            retry_delay_ms: 500,
            request_timeout_s: 600,
            request_replacements: Vec::new(),
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let path = std::env::args_os().nth(1).map(PathBuf::from).unwrap_or_else(|| {
            let mut p = std::env::current_exe().unwrap_or_default();
            p.set_file_name("proxyia.toml");
            p
        });
        match std::fs::read_to_string(&path) {
            Ok(s) => toml::from_str(&s).unwrap_or_else(|e| {
                eprintln!("Config invalide ({}): {e}", path.display());
                std::process::exit(1);
            }),
            Err(_) => {
                eprintln!("Config absente ({}), valeurs par défaut", path.display());
                Self::default()
            }
        }
    }
}
