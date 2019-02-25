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

fn main() -> Result<(), Box<std::error::Error>> {
    let cmd = OptArgs::from_args();
    match cmd.subcmd {
        SubCmd::Update => {}
    };

    let pmdb_file = dirs::data_dir().unwrap().join("pm.db");
    let mut db = PMDB::new(&pmdb_file)?;

    if !db.is_created()? {
        db.create_default_tables()?;
    }

    let cfg = config::Config::load_default()?;
    let client = reqwest::Client::new();

    /* Command line --verbose overrides config file */
    let verbose = cmd.verbose || cfg.verbose();

    /*
     * Get pkg_summary from each repository and check Last-Modified against
     * our database.
     */
    if let Some(repos) = cfg.repositories() {
        for repo in repos {
            /*
             * Start with the default set of extensions.  XXX: presumably
             * there is a way to do this in one match statement, but I can't
             * get past various issues when trying to return a vec!
             */
            let mut summary_extensions = vec!["xz", "bz2", "gz"];

            if let Some(ext) = repo.summary_extension() {
                summary_extensions = vec![ext];
            }

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
