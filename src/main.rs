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
 * pm(1) - a package manager for pkgsrc
 */

mod config;
mod pmdb;
mod summary;

extern crate bzip2;
extern crate chrono;
extern crate flate2;
extern crate httpdate;
extern crate rusqlite;
extern crate structopt;
extern crate xz2;

use crate::config::Config;
use crate::pmdb::PMDB;
use crate::summary::SummaryStream;
use std::time::SystemTime;
use structopt::StructOpt;

/*
 * Command line options.
 */
#[derive(Debug, StructOpt)]
#[structopt(name = "pm", about = "A binary package manager for pkgsrc")]
struct OptArgs {
    #[structopt(short = "v", long = "verbose", help = "Enable verbose output")]
    verbose: bool,
    #[structopt(subcommand)]
    subcmd: SubCmd,
}

#[derive(Debug, StructOpt)]
enum SubCmd {
    #[structopt(
        name = "update",
        alias = "up",
        about = "Update pkg_summary from each configured repository"
    )]
    Update,
}

/*
 * Return a list of pkg_summary extensions to search for in the remote
 * repository.  Use the user's chosen value if specified in the config,
 * otherwise use the default list which is ordered by compression size,
 * best to worst.  First match on the remote end wins.
 */
fn get_summary_extensions(repo: &config::RepoConfig) -> Vec<&str> {
    if let Some(extension) = repo.summary_extension() {
        vec![extension]
    } else {
        vec!["xz", "bz2", "gz"]
    }
}

fn update(
    verbose: bool,
    cfg: &Config,
    db: &mut PMDB,
) -> Result<(), Box<std::error::Error>> {
    let client = reqwest::Client::new();

    /*
     * Get pkg_summary from each repository and check Last-Modified against
     * our database.
     */
    if let Some(repos) = cfg.repositories() {
        for repo in repos {
            let summary_extensions = get_summary_extensions(&repo);

            for e in summary_extensions {
                if verbose {
                    println!("Trying summary_suffix={}", e);
                }

                let sumurl = format!("{}/{}.{}", repo.url(), "pkg_summary", e);

                let res =
                    reqwest::Client::get(&client, sumurl.as_str()).send()?;

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

                if let Some(r) = db.get_repository(repo.url())? {
                    if r.up_to_date(last_modified, e) {
                        println!("{} is up to date", repo.url());
                    } else {
                        println!("Updating {}", repo.url());
                        sumstr.slurp(&e, res)?;
                        sumstr.parse();
                        db.update_repository(
                            repo.url(),
                            last_modified,
                            e,
                            sumstr.entries(),
                        )?;
                    }
                } else {
                    println!("Creating {}", repo.url());
                    sumstr.slurp(&e, res)?;
                    sumstr.parse();
                    db.insert_repository(
                        repo.url(),
                        last_modified,
                        e,
                        sumstr.entries(),
                    )?;
                }

                /* We're done, skip remaining suffixes */
                break;
            }
        }
    }

    Ok(())
}

fn main() -> Result<(), Box<std::error::Error>> {
    let cmd = OptArgs::from_args();
    let cfg = Config::load_default()?;

    let pmdb_file = dirs::data_dir().unwrap().join("pm.db");
    let mut db = PMDB::new(&pmdb_file)?;

    /* Command line --verbose overrides config file */
    let verbose = cmd.verbose || cfg.verbose();

    match cmd.subcmd {
        SubCmd::Update => update(verbose, &cfg, &mut db)?,
    };

    Ok(())
}
