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
 * pmdb.rs - handle sqlite3 database interaction using rusqlite
 */

extern crate rusqlite;

use rusqlite::Connection;

#[derive(Debug)]
pub struct PMDB {
    conn: Connection,
    repositories: Vec<Repository>,
}

#[derive(Debug)]
pub struct Repository {
    url: String,
    mtime: i64,
    summary_suffix: String,
    need_update: bool,
}

impl PMDB {
    pub fn new(p: &std::path::Path) -> rusqlite::Result<PMDB> {
        let c = Connection::open(p)?;
        Ok(PMDB {
            conn: c,
            repositories: Vec::new(),
        })
    }

    /*
     * Test for the existance of the "repositories" table to determine if we
     * need to create the initial set of tables or not.
     */
    pub fn is_created(&self) -> rusqlite::Result<bool> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*)
               FROM sqlite_master
              WHERE type='table'
                AND name='repositories'",
            rusqlite::NO_PARAMS,
            |r| r.get(0),
        )?;
        Ok(count > 0)
    }

    /*
     * Create the default set of tables.  For now upgrades aren't supported,
     * we simply drop everything and start again on schema changes.
     *
     * XXX: I don't understand why using a transaction means I'm forced to
     * make the whole thing mutable, would prefer to avoid that.
     */
    pub fn create_default_tables(&mut self) -> rusqlite::Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute_batch(
            "
            CREATE TABLE repositories (
                id                  INTEGER PRIMARY KEY,
                url                 TEXT UNIQUE,
                summary_suffix      TEXT,
                mtime               INTEGER
            );
            ",
        )?;
        tx.commit()
    }

    pub fn get_repository(
        &self,
        url: &str,
    ) -> rusqlite::Result<Option<Repository>> {
        let mut stmt = self.conn.prepare(
            "SELECT mtime, summary_suffix
               FROM repositories
              WHERE url = ?",
        )?;
        let mut rows = stmt.query(&[url])?;
        match rows.next() {
            Some(row) => {
                let row = row?;
                Ok(Some(Repository {
                    url: url.to_string(),
                    mtime: row.get(0),
                    summary_suffix: row.get(1),
                    need_update: false,
                }))
            }
            None => Ok(None),
        }
    }

    pub fn create_repository(
        &self,
        url: &str,
        mtime: i64,
        summary_suffix: &str,
    ) -> rusqlite::Result<usize> {
        let mut stmt = self.conn.prepare(
            "INSERT INTO repositories (url, mtime, summary_suffix)
                  VALUES (:url, :mtime, :summary_suffix)",
        )?;
        stmt.execute_named(&[
            (":url", &url),
            (":mtime", &mtime),
            (":summary_suffix", &summary_suffix),
        ])
    }

    pub fn update_repository(
        &self,
        url: &str,
        mtime: i64,
        summary_suffix: &str,
    ) -> rusqlite::Result<usize> {
        let mut stmt = self.conn.prepare(
            "UPDATE repositories
                SET mtime = :mtime,
                    summary_suffix = :summary_suffix
              WHERE url = :url",
        )?;
        stmt.execute_named(&[
            (":url", &url),
            (":mtime", &mtime),
            (":summary_suffix", &summary_suffix),
        ])
    }
}

impl Repository {
    pub fn up_to_date(&self, mtime: i64, summary_suffix: &str) -> bool {
        self.mtime == mtime && self.summary_suffix == summary_suffix
    }
}
