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
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

extern crate dirs;
extern crate toml;

/*
 * Parsed configuration file.
 */
#[derive(Debug)]
pub struct Config {
    filename: PathBuf,
    prefix: String,
    prefixmap: HashMap<String, Vec<Repository>>,
    verbose: bool,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Repository {
    url: String,
    prefix: String,
    summary_extension: Option<String>,
}

/*
 * Struct used for deserializing from the TOML configuration file, this is
 * parsed into Config for use throughout the program.
 */
#[derive(Clone, Debug, Deserialize)]
struct ConfigFile {
    default_prefix: Option<String>,
    verbose: Option<bool>,
    repository: Option<Vec<Repository>>,
}

impl Config {
    pub fn prefix(&self) -> &str {
        &self.prefix
    }

    pub fn prefixmap(&self) -> &HashMap<String, Vec<Repository>> {
        &self.prefixmap
    }

    #[allow(dead_code)]
    pub fn verbose(&self) -> bool {
        self.verbose
    }

    pub fn load(argv: &OptArgs) -> Result<Config, std::io::Error> {
        /*
         * Load user-specific configuration file otherwise the default.
         */
        let cfgfilename: PathBuf = if argv.config.is_some() {
            PathBuf::from(argv.config.clone().unwrap().as_str())
        } else {
            dirs::config_dir().unwrap().join("pm.toml")
        };
        if !cfgfilename.exists() {
            eprintln!(
                "ERROR: Configuration file {} does not exist",
                cfgfilename.display()
            );
            std::process::exit(1);
        }

        let cfgfile: ConfigFile =
            toml::from_str(&fs::read_to_string(&cfgfilename)?).unwrap();

        let mut prefix;
        if let Some(p) = &argv.prefix {
            prefix = p.to_string();
        } else if let Some(p) = &cfgfile.default_prefix() {
            prefix = p.to_string();
        } else if let Some(p) = &cfgfile.default_repo_prefix() {
            prefix = p.to_string()
        } else {
            eprintln!("ERROR: No repositories specified");
            std::process::exit(1);
        }

        /*
         * Validate and insert configured repositories into a HashMap
         * indexed by prefix.
         */
        let mut prefixmap: HashMap<String, Vec<Repository>> = HashMap::new();
        if let Some(repos) = &cfgfile.repositories() {
            for repo in repos {
                let pkg_info = format!("{}/sbin/pkg_info", &repo.prefix());
                let pkg_info = PathBuf::from(pkg_info);
                if !pkg_info.exists() {
                    eprintln!(
                        "WARNING: No pkg_install found under {}, skipping",
                        &repo.prefix()
                    );
                    continue;
                }
                if let Some(r) = prefixmap.get_mut(&repo.prefix().to_string()) {
                    r.push(repo.clone());
                } else {
                    prefixmap
                        .insert(repo.prefix().to_string(), vec![repo.clone()]);
                }
            }
        }

        let verbose = argv.verbose || cfgfile.verbose.unwrap_or(false);

        Ok(Config {
            filename: cfgfilename,
            prefix,
            prefixmap,
            verbose,
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

    pub fn repositories(&self) -> &Option<Vec<Repository>> {
        &self.repository
    }
}

impl Repository {
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
