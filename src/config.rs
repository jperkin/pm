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
use std::process::Command;

/*
 * Parsed configuration file.
 */
#[derive(Debug)]
pub struct Config {
    filename: PathBuf,
    prefix: String,
    prefixes: Vec<Prefix>,
    verbose: bool,
}

/*
 * Struct used for deserializing from the TOML configuration file, this is
 * parsed into Config for use throughout the program.
 */
#[derive(Debug, Deserialize)]
struct ConfigFile {
    default_prefix: Option<String>,
    verbose: Option<bool>,
    prefix: Option<Vec<Prefix>>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Prefix {
    path: String,
    pkg_admin: Option<String>,
    pkg_info: Option<String>,
    pkgdb: Option<String>,
    repository: Option<Vec<Repository>>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Repository {
    url: String,
    name: Option<String>,
    summary_extension: Option<String>,
}

impl Config {
    pub fn prefix(&self) -> &str {
        &self.prefix
    }

    pub fn prefixes(&self) -> &Vec<Prefix> {
        &self.prefixes
    }

    #[allow(dead_code)]
    pub fn verbose(&self) -> bool {
        self.verbose
    }

    pub fn load(argv: &OptArgs) -> Result<Config, std::io::Error> {
        /*
         * Start with an empty Config, then populate it based on the input
         * from the user pm.toml.
         */
        let mut config = Config {
            filename: PathBuf::new(),
            prefix: String::new(),
            prefixes: vec![],
            verbose: false,
        };

        /*
         * Load user-specific configuration file otherwise the default.
         */
        config.filename = if argv.config.is_some() {
            PathBuf::from(argv.config.clone().unwrap().as_str())
        } else {
            dirs::config_dir().unwrap().join("pm.toml")
        };
        if !config.filename.exists() {
            eprintln!(
                "ERROR: Configuration file {} does not exist",
                config.filename.display()
            );
            std::process::exit(1);
        }

        let cfgfile: ConfigFile =
            toml::from_str(&fs::read_to_string(&config.filename)?).unwrap();

        /*
         * Validate and insert configured prefixes.  Save the first prefix to
         * use as the default if not otherwise specified.
         */
        let mut first_prefix: Option<String> = None;
        if let Some(prefixes) = cfgfile.prefix {
            for prefix in prefixes {
                let mut p = prefix.clone();
                first_prefix.get_or_insert(p.path().to_string());
                p.pkg_admin
                    .get_or_insert(format!("{}/sbin/pkg_admin", p.path()));
                p.pkg_info
                    .get_or_insert(format!("{}/sbin/pkg_info", p.path()));
                if !PathBuf::from(p.pkg_admin.as_ref().unwrap()).exists()
                    || !PathBuf::from(p.pkg_info.as_ref().unwrap()).exists()
                {
                    eprintln!(
                        "SKIPPING: No pkg_install found under {}",
                        p.path()
                    );
                    continue;
                }
                /*
                 * Calculate PKG_DBDIR from pkg_admin(1) if not specified.
                 */
                if p.pkgdb.is_none() {
                    let pkgdb = Command::new(p.pkg_admin())
                        .args(&["config-var", "PKG_DBDIR"])
                        .output()
                        .expect("could not execute pkg_admin");
                    let pkgdb =
                        std::str::from_utf8(&pkgdb.stdout).unwrap().trim();
                    p.pkgdb = Some(pkgdb.to_string());
                }
                config.prefixes.push(p);
            }
        }

        /*
         * Set the default prefix to use for operations.
         */
        if let Some(p) = &argv.prefix {
            config.prefix = p.clone();
        } else if let Some(p) = cfgfile.default_prefix {
            config.prefix = p.clone();
        } else if let Some(p) = first_prefix {
            config.prefix = p.clone();
        }

        config.verbose = argv.verbose || cfgfile.verbose.unwrap_or(false);

        Ok(config)
    }
}

impl Prefix {
    pub fn path(&self) -> &str {
        &self.path
    }

    /*
     * These are all safe to unwrap as they are checked during the loading of
     * the configuration prior to use.
     */
    pub fn pkg_admin(&self) -> &str {
        &self.pkg_admin.as_ref().unwrap()
    }
    pub fn pkg_info(&self) -> &str {
        &self.pkg_info.as_ref().unwrap()
    }
    pub fn pkgdb(&self) -> &str {
        &self.pkgdb.as_ref().unwrap()
    }

    pub fn repositories(&self) -> &Option<Vec<Repository>> {
        &self.repository
    }
}

impl Repository {
    pub fn url(&self) -> &String {
        &self.url
    }

    pub fn summary_extension(&self) -> &Option<String> {
        &self.summary_extension
    }
}
