/*
 * Copyright (c) 2019 Jonathan Perkin <jonathan@perkin.org.uk>
 *
 * Permission to use, copy, modify, and distribute this software for any
 * purpose with or without fee is hereby granted, provided that the above
 * copyright notice and this permission notice appear in all copies.
 *
 * THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
 * WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
 * MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
 * ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
 * WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN
 * ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF
 * OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.
 *
 * config.rs - handle loading/parsing of pm(1) config files written in TOML.
 */

use crate::OptArgs;
use serde_derive::Deserialize;
use std::fs;
use std::path::PathBuf;

extern crate dirs;
extern crate toml;

/*
 * Parsed configuration file.
 */
#[derive(Debug)]
pub struct Config {
    file: ConfigFile,
    filename: PathBuf,
    prefix: Option<String>,
    verbose: bool,
}

#[derive(Debug, Deserialize)]
pub struct ConfigFile {
    #[serde(default)]
    default_prefix: Option<String>,
    verbose: Option<bool>,
    repository: Option<Vec<RepoConfig>>,
}

#[derive(Debug, Deserialize)]
pub struct RepoConfig {
    url: String,
    prefix: String,
    summary_extension: Option<String>,
}

impl Config {
    pub fn set_config_from_cmdline(&mut self, argv: &OptArgs) {
        if argv.prefix.is_some() {
            self.prefix = argv.prefix.clone();
        }
        if argv.verbose {
            self.verbose = true;
        }
    }

    pub fn repositories(&self) -> &Option<Vec<RepoConfig>> {
        &self.file.repository
    }

    pub fn prefix(&self) -> &Option<String> {
        &self.prefix
    }

    pub fn verbose(&self) -> bool {
        self.verbose
    }

    pub fn load(argv: &OptArgs) -> Result<Config, std::io::Error> {
        let config_file: PathBuf = if argv.config.is_some() {
            PathBuf::from(argv.config.clone().unwrap().as_str())
        } else {
            dirs::config_dir().unwrap().join("pm.toml")
        };

        if !config_file.exists() {
            eprintln!(
                "ERROR: Configuration file {} does not exist",
                config_file.display()
            );
            std::process::exit(1);
        }

        let config_str: String = fs::read_to_string(&config_file)?;
        let cfg: ConfigFile = toml::from_str(&config_str).unwrap();
        let default_prefix: Option<String> =
            if let Some(p) = &cfg.default_prefix() {
                Some(p.to_string())
            } else if let Some(p) = &cfg.default_repo_prefix() {
                Some(p.to_string())
            } else {
                None
            };
        let default_verbose = cfg.verbose.unwrap_or(false);
        Ok(Config {
            file: cfg,
            filename: config_file,
            prefix: default_prefix,
            verbose: default_verbose,
        })
    }
}

impl ConfigFile {
    pub fn default_prefix(&self) -> &Option<String> {
        &self.default_prefix
    }

    pub fn default_repo_prefix(&self) -> Option<&String> {
        match &self.repository {
            Some(r) => Some(&r[0].prefix),
            None => None,
        }
    }
}

impl RepoConfig {
    pub fn url(&self) -> &String {
        &self.url
    }

    pub fn prefix(&self) -> &String {
        &self.prefix
    }

    pub fn summary_extension(&self) -> &Option<String> {
        &self.summary_extension
    }
}
