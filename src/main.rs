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

mod avail;
mod config;
mod pmdb;
mod summary;

extern crate bzip2;
extern crate chrono;
extern crate flate2;
extern crate httpdate;
extern crate regex;
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
pub struct OptArgs {
    #[structopt(
        short = "c",
        long = "config",
        help = "Use specified configuration file"
    )]
    config: Option<String>,
    #[structopt(short = "p", long = "prefix", help = "Set default prefix")]
    prefix: Option<String>,
    #[structopt(short = "v", long = "verbose", help = "Enable verbose output")]
    verbose: bool,
    #[structopt(subcommand)]
    subcmd: SubCmd,
}

#[derive(Debug, StructOpt)]
enum SubCmd {
    #[structopt(
        name = "avail",
        alias = "av",
        about = "List available packages"
    )]
    Avail,
    #[structopt(
        name = "search",
        alias = "se",
        about = "Search available packages"
    )]
    Search { query: String },
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

fn update(cfg: &Config, db: &mut PMDB) -> Result<(), Box<std::error::Error>> {
    let client = reqwest::Client::new();

    /*
     * Get pkg_summary from each repository and check Last-Modified against
     * our database.
     */
    if let Some(repos) = cfg.repositories() {
        for repo in repos {
            let summary_extensions = get_summary_extensions(&repo);

            for e in summary_extensions {
                if cfg.verbose() {
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
                        repo.prefix(),
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

/*
 * Check that we have a valid prefix for commands that require one, and return
 * as a str for easy handling, otherwise exit.
 */
fn valid_prefix_or_errx(prefix: &Option<String>) -> &str {
    match prefix {
        Some(v) => v.as_str(),
        None => {
            eprintln!("ERROR: No prefix configured");
            std::process::exit(1);
        }
    }
}

fn main() -> Result<(), Box<std::error::Error>> {
    let cmd = OptArgs::from_args();

    /* Pass cmd so that the user can override the default with -c */
    let mut cfg = Config::load(&cmd)?;
    cfg.set_config_from_cmdline(&cmd);

    let pmdb_file = dirs::data_dir().unwrap().join("pm.db");
    let mut db = PMDB::new(&pmdb_file)?;

    match cmd.subcmd {
        SubCmd::Avail => {
            let prefix = valid_prefix_or_errx(&cfg.prefix());
            avail::run(&mut db, prefix)?;
        }
        SubCmd::Search { query } => {
            let prefix = valid_prefix_or_errx(&cfg.prefix());
            avail::search(&mut db, prefix, &query)?;
        }
        SubCmd::Update => update(&cfg, &mut db)?,
    };

    Ok(())
}
