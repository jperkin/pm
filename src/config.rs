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
 * config.rs - handle loading/parsing of pm(1) config files written in TOML
 */

use serde_derive::Deserialize;
use std::fs::File;
use std::io::prelude::*;

extern crate dirs;
extern crate toml;

/*
 * Parsed configuration file.
 */
#[derive(Debug)]
pub struct Config {
    file: ConfigFile,
    filename: std::fs::File,
}

#[derive(Debug, Deserialize)]
pub struct ConfigFile {
    #[serde(default)]
    verbose: Option<bool>,
    repository: Option<Vec<RepoConfig>>,
}

#[derive(Debug, Deserialize)]
pub struct RepoConfig {
    url: String,
    summary_extension: Option<String>,
}

impl Config {
    pub fn repositories(&self) -> &Option<Vec<RepoConfig>> {
        &self.file.repository
    }

    pub fn verbose(&self) -> bool {
        /* XXX: probably a one-liner way of doing this? */
        match self.file.verbose {
            Some(v) => v,
            None => false,
        }
    }

    pub fn load_default() -> Result<Config, std::io::Error> {
        let default_config = dirs::config_dir().unwrap().join("pm.toml");
        let mut config_file = File::open(default_config)?;
        let mut config_str = String::new();
        config_file.read_to_string(&mut config_str)?;
        let cfg: ConfigFile = toml::from_str(&config_str).unwrap();
        Ok(Config {
            file: cfg,
            filename: config_file,
        })
    }
}

impl RepoConfig {
    pub fn url(&self) -> &String {
        &self.url
    }

    pub fn summary_extension(&self) -> &Option<String> {
        &self.summary_extension
    }
}
