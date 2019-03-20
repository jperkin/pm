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
 * update.rs - handle "pm update" command.
 */

extern crate reqwest;

use crate::config;
use crate::pmdb::PMDB;
use crate::summary::SummaryStream;
use std::fs;
use std::io::BufReader;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::str;
use std::time::SystemTime;

/*
 * Return a list of pkg_summary extensions to search for in the remote
 * repository.  Use the user's chosen value if specified in the config,
 * otherwise use the default list which is ordered by compression size,
 * best to worst.  First match on the remote end wins.
 */
fn get_summary_extensions(repo: &config::Repository) -> Vec<&str> {
    if let Some(extension) = repo.summary_extension() {
        vec![extension]
    } else {
        vec!["xz", "bz2", "gz"]
    }
}

/*
 * Get Vec of packages installed under the chosen prefix.
 */
fn get_local_packages(
    prefix: &config::Prefix,
) -> Result<SummaryStream, Box<std::error::Error>> {
    let pinfo = Command::new(prefix.pkg_info())
        .args(&["-X", "-a"])
        .stdout(Stdio::piped())
        .spawn()
        .expect("could not spawn pkg_info");
    let mut reader = BufReader::new(pinfo.stdout.expect("fail"));
    let mut pinfostr = SummaryStream::new();
    std::io::copy(&mut reader, &mut pinfostr)?;
    /*
     * Look for "automatic" packages (those that have been pulled in as a
     * dependency).  This is a bit hacky, and relies upon the fact that
     * pkg_install only uses this file for automatic=yes right now.
     */
    for pkg in pinfostr.entries_mut() {
        let autofile =
            format!("{}/{}/+INSTALLED_INFO", prefix.pkgdb(), pkg.pkgname());
        let autofile = PathBuf::from(autofile);
        if autofile.exists() {
            pkg.set_automatic();
        }
    }
    Ok(pinfostr)
}

fn update_local_repository(
    prefix: &config::Prefix,
    db: &mut PMDB,
) -> Result<(), Box<std::error::Error>> {
    /*
     * Get the last modified time of the pkgdb to see if we need to refresh
     * the local package database for this prefix.
     */
    let pkgdb_mtime = fs::metadata(prefix.pkgdb())?
        .modified()?
        .duration_since(SystemTime::UNIX_EPOCH)?;
    let pkgdb_mtime_sec = pkgdb_mtime.as_secs() as i64;
    let pkgdb_mtime_nsec = pkgdb_mtime.subsec_nanos() as i32;

    if let Some(r) = db.get_local_repository(prefix.path())? {
        if r.up_to_date(pkgdb_mtime_sec, pkgdb_mtime_nsec) {
            return Ok(());
        } else {
            println!("Refreshing packages installed under {}", prefix.path());
            let pkgs: SummaryStream = get_local_packages(&prefix)?;
            db.update_local_repository(
                prefix.path(),
                pkgdb_mtime_sec,
                pkgdb_mtime_nsec,
                pkgs.entries(),
            )?;
        }
    } else {
        println!("Recording packages installed under {}", prefix.path());
        let pkgs: SummaryStream = get_local_packages(&prefix)?;
        db.insert_local_repository(
            prefix.path(),
            pkgdb_mtime_sec,
            pkgdb_mtime_nsec,
            pkgs.entries(),
        )?;
    }

    Ok(())
}

fn update_remote_repository(
    prefix: &str,
    repo: &config::Repository,
    db: &mut PMDB,
) -> Result<(), Box<std::error::Error>> {
    let client = reqwest::Client::new();

    let summary_extensions = get_summary_extensions(&repo);

    for e in summary_extensions {
        let sumurl = format!("{}/{}.{}", repo.url(), "pkg_summary", e);

        let res = reqwest::Client::get(&client, sumurl.as_str()).send()?;

        /* Not found, try next pkg_summary extension */
        if !res.status().is_success() {
            continue;
        }

        /* XXX: this seems overly verbose, no simpler way? */
        let last_modified: i64 = if let Some(lm) =
            res.headers().get(reqwest::header::LAST_MODIFIED)
        {
            httpdate::parse_http_date(lm.to_str().unwrap())?
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64
        } else {
            continue;
        };

        /*
         * We now have a valid pkg_summary, check DB for existing entry
         * and insert/update as appropriate.
         */
        let mut sumstr = SummaryStream::new();

        if let Some(r) = db.get_remote_repository(repo.url())? {
            if r.up_to_date(last_modified, e) {
                println!("{} is up to date", repo.url());
            } else {
                println!("Updating {}", repo.url());
                sumstr.slurp(&e, res)?;
                db.update_remote_repository(
                    repo.url(),
                    last_modified,
                    e,
                    sumstr.entries(),
                )?;
            }
        } else {
            println!("Creating {}", repo.url());
            sumstr.slurp(&e, res)?;
            db.insert_remote_repository(
                repo.url(),
                prefix,
                last_modified,
                e,
                sumstr.entries(),
            )?;
        }

        /* We're done, skip remaining suffixes */
        break;
    }
    Ok(())
}

pub fn run(
    cfg: &config::Config,
    db: &mut PMDB,
) -> Result<(), Box<std::error::Error>> {
    for prefix in cfg.prefixes() {
        update_local_repository(&prefix, db)?;
        if let Some(repos) = prefix.repositories() {
            for repo in repos {
                update_remote_repository(prefix.path(), repo, db)?;
            }
        }
    }

    Ok(())
}
