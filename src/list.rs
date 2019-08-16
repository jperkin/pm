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
 * list.rs - handle commands that require a list of packages.
 */

use crate::config;
use crate::pmdb::PMDB;

/*
 * A PackageList is an entry from the database of either a local or remote
 * package.  There is a lot of duplication with SummaryEntry, but they are
 * distinct types so we keep them separate.
 *
 * Some entries are specific to local packages, some only to remote.  We
 * derive from Default and ensure all entries are set to default values and
 * it is up to callers to use the correct information.
 */
#[derive(Clone, Debug, Default)]
pub struct PackageList {
    pub id: i64,
    pub repository_id: i64,
    pub automatic: bool,
    pub build_date: String,
    pub categories: String,
    pub comment: String,
    pub description: String,
    pub file_name: String,
    pub file_size: i64,
    pub homepage: String,
    pub license: String,
    pub opsys: String,
    pub os_version: String,
    pub pkg_options: String,
    pub pkgbase: String,
    pub pkgname: String,
    pub pkgpath: String,
    pub pkgtools_version: String,
    pub pkgversion: String,
    pub size_pkg: i64,
}

impl PackageList {
    #[allow(dead_code)]
    pub fn id(&self) -> &i64 {
        &self.id
    }
    pub fn repository_id(&self) -> &i64 {
        &self.repository_id
    }
    pub fn automatic(&self) -> &bool {
        &self.automatic
    }
    pub fn build_date(&self) -> &String {
        &self.build_date
    }
    pub fn categories(&self) -> &String {
        &self.categories
    }
    pub fn comment(&self) -> &String {
        &self.comment
    }
    pub fn description(&self) -> &String {
        &self.description
    }
    pub fn file_name(&self) -> &String {
        &self.file_name
    }
    pub fn file_size(&self) -> &i64 {
        &self.file_size
    }
    pub fn homepage(&self) -> &String {
        &self.homepage
    }
    pub fn license(&self) -> &String {
        &self.license
    }
    pub fn opsys(&self) -> &String {
        &self.opsys
    }
    pub fn os_version(&self) -> &String {
        &self.os_version
    }
    pub fn pkg_options(&self) -> &String {
        &self.pkg_options
    }
    pub fn pkgbase(&self) -> &String {
        &self.pkgbase
    }
    pub fn pkgname(&self) -> &String {
        &self.pkgname
    }
    pub fn pkgpath(&self) -> &String {
        &self.pkgpath
    }
    pub fn pkgtools_version(&self) -> &String {
        &self.pkgtools_version
    }
    pub fn pkgversion(&self) -> &String {
        &self.pkgversion
    }
    pub fn size_pkg(&self) -> &i64 {
        &self.size_pkg
    }
}

pub fn avail(
    cfg: &config::Config,
    db: &mut PMDB,
) -> Result<(), Box<std::error::Error>> {
    let pkgs = db.get_remote_pkglist_by_prefix(cfg.prefix())?;
    if pkgs.is_empty() {
        eprintln!("No packages available for prefix={}", cfg.prefix());
        std::process::exit(1);
    }
    for pkg in pkgs {
        println!("{:20} {}", pkg.pkgname(), pkg.comment());
    }
    Ok(())
}

pub fn list(
    cfg: &config::Config,
    db: &mut PMDB,
) -> Result<(), Box<std::error::Error>> {
    let pkgs = db.get_local_pkglist_by_prefix(cfg.prefix())?;
    if pkgs.is_empty() {
        eprintln!("No packages recorded under {}", cfg.prefix());
        std::process::exit(1);
    }
    for pkg in pkgs {
        println!("{:20} {}", pkg.pkgname(), pkg.comment());
    }
    Ok(())
}
