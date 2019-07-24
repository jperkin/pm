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
 * pm(1) - a package manager for pkgsrc.
 */

mod config;
mod list;
mod pmdb;
mod search;
mod summary;
mod update;

use crate::pmdb::PMDB;
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
        name = "list",
        alias = "ls",
        about = "List installed packages"
    )]
    List,
    #[structopt(
        name = "search",
        alias = "se",
        about = "Search available packages"
    )]
    Search {
        #[structopt(help = "Query string (regular expression)")]
        query: String,
    },
    #[structopt(
        name = "update",
        alias = "up",
        about = "Update pkg_summary from each configured repository"
    )]
    Update,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cmd = OptArgs::from_args();

    /* Pass cmd so that the user can override the default with -c */
    let cfg = config::Config::load(&cmd)?;

    let pmdb_file = dirs::data_dir().unwrap().join("pm.db");
    let mut db = PMDB::new(&pmdb_file)?;

    match &cmd.subcmd {
        SubCmd::Avail => {
            list::avail(&cfg, &mut db)?;
        }
        SubCmd::List => {
            list::list(&cfg, &mut db)?;
        }
        SubCmd::Search { query } => {
            search::run(&cfg, &mut db, &query)?;
        }
        SubCmd::Update => {
            update::run(&cfg, &mut db)?;
        }
    };

    Ok(())
}
