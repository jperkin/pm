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
 * pmdb.rs - handle sqlite3 database interaction using rusqlite.
 */

extern crate rusqlite;

use crate::list::ListPackage;
use crate::summary::SummaryEntry;
use rusqlite::Connection;
use std::fs;

#[derive(Debug)]
pub struct PMDB {
    db: Connection,
    repositories: Vec<RemoteRepository>,
}

#[derive(Debug)]
pub struct LocalRepository {
    prefix: String,
    mtime: i64,
    ntime: i32,
    need_update: bool,
}

#[derive(Debug)]
pub struct RemoteRepository {
    url: String,
    mtime: i64,
    summary_suffix: String,
    need_update: bool,
}

#[cfg_attr(feature = "cargo-clippy", allow(clippy::unreadable_literal))]
const DB_VERSION: i64 = 20190304;

impl PMDB {
    /*
     * Open a new connection to the database and perform any necessary setup
     * prior to returning.
     */
    pub fn new(p: &std::path::Path) -> rusqlite::Result<PMDB> {
        fs::create_dir_all(
            p.parent().expect("Could not determine database path"),
        )
        .expect("Could not create database directory");
        let mut db = Connection::open(p)?;

        /*
         * pkgin plays rather fast and loose with the database, let's try
         * instead going the other way and making it as safe as possible.
         */
        db.execute("PRAGMA synchronous = EXTRA;", rusqlite::NO_PARAMS)?;

        if !PMDB::is_created(&db)? {
            PMDB::create_default_tables(&mut db)?;
        } else if !PMDB::is_current(&db)? {
            PMDB::drop_default_tables(&mut db)?;
            PMDB::create_default_tables(&mut db)?;
        }

        Ok(PMDB {
            db,
            repositories: Vec::new(),
        })
    }

    /*
     * Test for the existance of the "metadata" table to determine if
     * we need to create the initial set of tables or not.
     */
    fn is_created(db: &Connection) -> rusqlite::Result<bool> {
        let count: i64 = db.query_row(
            "SELECT COUNT(*)
               FROM sqlite_master
              WHERE type='table'
                AND name='metadata'",
            rusqlite::NO_PARAMS,
            |r| r.get(0),
        )?;
        Ok(count > 0)
    }

    fn is_current(db: &Connection) -> rusqlite::Result<bool> {
        let current: i64 = db.query_row(
            "SELECT version
               FROM metadata",
            rusqlite::NO_PARAMS,
            |r| r.get(0),
        )?;
        Ok(current == DB_VERSION)
    }

    /*
     * Create the default set of tables.  For now upgrades aren't supported,
     * we simply drop everything and start again on schema changes.
     *
     * XXX: I don't understand why using a transaction means I'm forced to
     * make the whole thing mutable, would prefer to avoid that.
     */
    pub fn create_default_tables(db: &mut Connection) -> rusqlite::Result<()> {
        let tx = db.transaction()?;
        tx.execute_batch(
            "
            CREATE TABLE metadata (
                version             INTEGER
            );
            CREATE TABLE local_repository (
                id                  INTEGER PRIMARY KEY,
                prefix              TEXT UNIQUE,
                mtime               INTEGER,
                ntime               INTEGER
            );
            CREATE TABLE remote_repository (
                id                  INTEGER PRIMARY KEY,
                prefix              TEXT,
                url                 TEXT UNIQUE,
                summary_suffix      TEXT,
                mtime               INTEGER
            );
            CREATE TABLE local_pkg (
                id                  INTEGER PRIMARY KEY,
                repository_id       INTEGER,
                build_date          TEXT,
                categories          TEXT,
                comment             TEXT,
                description         TEXT,
                homepage            TEXT NULL,
                license             TEXT NULL,
                opsys               TEXT,
                os_version          TEXT,
                pkg_options         TEXT NULL,
                pkgbase             TEXT,
                pkgname             TEXT,
                pkgpath             TEXT,
                pkgtools_version    TEXT,
                pkgversion          TEXT,
                preserve            INTEGER,
                size_pkg            INTEGER
            );
            CREATE TABLE remote_pkg (
                id                  INTEGER PRIMARY KEY,
                repository_id       INTEGER,
                build_date          TEXT,
                categories          TEXT,
                comment             TEXT,
                description         TEXT,
                file_size           INTEGER,
                homepage            TEXT NULL,
                license             TEXT NULL,
                opsys               TEXT,
                os_version          TEXT,
                pkg_options         TEXT NULL,
                pkgbase             TEXT,
                pkgname             TEXT,
                pkgpath             TEXT,
                pkgtools_version    TEXT,
                pkgversion          TEXT,
                size_pkg            INTEGER
            );
            ",
        )?;
        {
            let mut stmt = tx.prepare(
                "REPLACE INTO metadata
                         (version)
                  VALUES (:version)",
            )?;
            stmt.execute_named(&[(":version", &DB_VERSION)])?;
        }
        tx.commit()
    }

    pub fn drop_default_tables(db: &mut Connection) -> rusqlite::Result<()> {
        let tx = db.transaction()?;
        tx.execute_batch(
            "
            DROP TABLE IF EXISTS metadata;
            DROP TABLE IF EXISTS local_repository;
            DROP TABLE IF EXISTS remote_repository;
            DROP TABLE IF EXISTS local_pkg;
            DROP TABLE IF EXISTS remote_pkg;
        ",
        )?;
        tx.commit()
    }

    pub fn get_local_repository(
        &self,
        prefix: &str,
    ) -> rusqlite::Result<Option<LocalRepository>> {
        let mut stmt = self.db.prepare(
            "SELECT mtime, ntime
               FROM local_repository
              WHERE prefix = :prefix",
        )?;
        let mut rows = stmt.query_named(&[(":prefix", &prefix)])?;
        match rows.next() {
            Some(row) => {
                let row = row?;
                Ok(Some(LocalRepository {
                    prefix: prefix.to_string(),
                    mtime: row.get(0),
                    ntime: row.get(1),
                    need_update: false,
                }))
            }
            None => Ok(None),
        }
    }

    pub fn get_remote_repository(
        &self,
        url: &str,
    ) -> rusqlite::Result<Option<RemoteRepository>> {
        let mut stmt = self.db.prepare(
            "SELECT mtime, summary_suffix
               FROM remote_repository
              WHERE url = :url",
        )?;
        let mut rows = stmt.query_named(&[(":url", &url)])?;
        match rows.next() {
            Some(row) => {
                let row = row?;
                Ok(Some(RemoteRepository {
                    url: url.to_string(),
                    mtime: row.get(0),
                    summary_suffix: row.get(1),
                    need_update: false,
                }))
            }
            None => Ok(None),
        }
    }

    fn insert_local_pkgs(
        tx: &rusqlite::Transaction,
        repo_id: i64,
        pkgs: &[SummaryEntry],
    ) -> rusqlite::Result<()> {
        let mut stmt = tx.prepare(
            "INSERT INTO local_pkg
                    (repository_id, build_date, categories, comment,
                     description, homepage, license, opsys, os_version,
                     pkg_options, pkgbase, pkgname, pkgpath,
                     pkgtools_version, pkgversion, size_pkg)
             VALUES (:repo_id, :build_date, :categories, :comment,
                     :description, :homepage, :license, :opsys, :os_version,
                     :pkg_options, :pkgbase, :pkgname, :pkgpath,
                     :pkgtools_version, :pkgversion, :size_pkg)",
        )?;

        for p in pkgs {
            /*
             * These values have all been checked earlier when inserted so
             * we are safe to unwrap.
             */
            stmt.execute_named(&[
                (":repo_id", &repo_id),
                (":build_date", &p.build_date()),
                (":categories", &p.categories().join(" ")),
                (":comment", &p.comment()),
                (":description", &p.description().join("\n")),
                (":homepage", &p.homepage()),
                (":license", &p.license()),
                (":opsys", &p.opsys()),
                (":os_version", &p.os_version()),
                (":pkg_options", &p.pkg_options()),
                (":pkgbase", &p.pkgbase()),
                (":pkgname", &p.pkgname()),
                (":pkgpath", &p.pkgpath()),
                (":pkgtools_version", &p.pkgtools_version()),
                (":pkgversion", &p.pkgversion()),
                (":size_pkg", &(p.size_pkg().unwrap())),
            ])?;
        }

        Ok(())
    }

    fn insert_remote_pkgs(
        tx: &rusqlite::Transaction,
        repo_id: i64,
        pkgs: &[SummaryEntry],
    ) -> rusqlite::Result<()> {
        let mut stmt = tx.prepare(
            "INSERT INTO remote_pkg
                    (repository_id, build_date, categories, comment,
                     description, file_size, homepage, license, opsys,
                     os_version, pkg_options, pkgbase, pkgname, pkgpath,
                     pkgtools_version, pkgversion, size_pkg)
             VALUES (:repo_id, :build_date, :categories, :comment,
                     :description, :file_size, :homepage, :license, :opsys,
                     :os_version, :pkg_options, :pkgbase, :pkgname, :pkgpath,
                     :pkgtools_version, :pkgversion, :size_pkg)",
        )?;

        for p in pkgs {
            /*
             * These values have all been checked earlier when inserted so
             * we are safe to unwrap.
             */
            stmt.execute_named(&[
                (":repo_id", &repo_id),
                (":build_date", &p.build_date()),
                (":categories", &p.categories().join(" ")),
                (":comment", &p.comment()),
                (":description", &p.description().join("\n")),
                (":file_size", &(p.file_size())),
                (":homepage", &p.homepage()),
                (":license", &p.license()),
                (":opsys", &p.opsys()),
                (":os_version", &p.os_version()),
                (":pkg_options", &p.pkg_options()),
                (":pkgbase", &p.pkgbase()),
                (":pkgname", &p.pkgname()),
                (":pkgpath", &p.pkgpath()),
                (":pkgtools_version", &p.pkgtools_version()),
                (":pkgversion", &p.pkgversion()),
                (":size_pkg", &(p.size_pkg().unwrap())),
            ])?;
        }

        Ok(())
    }

    fn delete_local_pkgs(
        tx: &rusqlite::Transaction,
        repo_id: i64,
    ) -> rusqlite::Result<usize> {
        let mut stmt = tx.prepare(
            "DELETE
               FROM local_pkg
              WHERE repository_id = :repo_id",
        )?;
        stmt.execute_named(&[(":repo_id", &repo_id)])
    }

    fn delete_remote_pkgs(
        tx: &rusqlite::Transaction,
        repo_id: i64,
    ) -> rusqlite::Result<usize> {
        let mut stmt = tx.prepare(
            "DELETE
               FROM remote_pkg
              WHERE repository_id = :repo_id",
        )?;
        stmt.execute_named(&[(":repo_id", &repo_id)])
    }

    pub fn insert_local_repository(
        &mut self,
        prefix: &str,
        mtime: i64,
        ntime: i32,
        pkgs: &[SummaryEntry],
    ) -> rusqlite::Result<()> {
        let tx = self.db.transaction()?;

        {
            let mut stmt = tx.prepare(
                "INSERT INTO local_repository
                        (prefix, mtime, ntime)
                 VALUES (:prefix, :mtime, :ntime)",
            )?;
            stmt.execute_named(&[
                (":prefix", &prefix),
                (":mtime", &mtime),
                (":ntime", &ntime),
            ])?;

            let repo_id = tx.last_insert_rowid();
            PMDB::insert_local_pkgs(&tx, repo_id, &pkgs)?;
        }

        tx.commit()
    }

    pub fn insert_remote_repository(
        &mut self,
        url: &str,
        prefix: &str,
        mtime: i64,
        summary_suffix: &str,
        pkgs: &[SummaryEntry],
    ) -> rusqlite::Result<()> {
        let tx = self.db.transaction()?;

        {
            let mut stmt = tx.prepare(
                "INSERT INTO remote_repository
                        (url, prefix, mtime, summary_suffix)
                 VALUES (:url, :prefix, :mtime, :summary_suffix)",
            )?;
            stmt.execute_named(&[
                (":url", &url),
                (":prefix", &prefix),
                (":mtime", &mtime),
                (":summary_suffix", &summary_suffix),
            ])?;

            let repo_id = tx.last_insert_rowid();
            PMDB::insert_remote_pkgs(&tx, repo_id, &pkgs)?;
        }

        tx.commit()
    }

    pub fn update_local_repository(
        &mut self,
        prefix: &str,
        mtime: i64,
        ntime: i32,
        pkgs: &[SummaryEntry],
    ) -> rusqlite::Result<()> {
        let tx = self.db.transaction()?;

        {
            let repo_id = tx.query_row_named(
                "SELECT id
                   FROM local_repository
                  WHERE prefix = :prefix",
                &[(":prefix", &prefix)],
                |row| row.get(0),
            )?;

            /*
             * Trying to update a repository in-place would just be a
             * nightmare.  Dropping and re-inserting is a lot simpler and
             * faster.
             */
            PMDB::delete_local_pkgs(&tx, repo_id)?;
            PMDB::insert_local_pkgs(&tx, repo_id, &pkgs)?;

            let mut stmt = tx.prepare(
                "UPDATE local_repository
                    SET mtime = :mtime,
                        ntime = :ntime
                  WHERE prefix = :prefix",
            )?;
            stmt.execute_named(&[
                (":mtime", &mtime),
                (":ntime", &ntime),
                (":prefix", &prefix),
            ])?;
        }

        tx.commit()
    }

    pub fn update_remote_repository(
        &mut self,
        url: &str,
        mtime: i64,
        summary_suffix: &str,
        pkgs: &[SummaryEntry],
    ) -> rusqlite::Result<()> {
        let tx = self.db.transaction()?;

        {
            let repo_id = tx.query_row_named(
                "SELECT id
                   FROM remote_repository
                  WHERE url = :url",
                &[(":url", &url)],
                |row| row.get(0),
            )?;

            /*
             * Trying to update a repository in-place would just be a
             * nightmare.  Dropping and re-inserting is a lot simpler and
             * faster.
             */
            PMDB::delete_remote_pkgs(&tx, repo_id)?;
            PMDB::insert_remote_pkgs(&tx, repo_id, &pkgs)?;

            let mut stmt = tx.prepare(
                "UPDATE remote_repository
                    SET mtime = :mtime,
                        summary_suffix = :summary_suffix
                  WHERE url = :url",
            )?;
            stmt.execute_named(&[
                (":mtime", &mtime),
                (":summary_suffix", &summary_suffix),
                (":url", &url),
            ])?;
        }

        tx.commit()
    }

    /*
     * Support functions for "avail" and "list".
     */
    pub fn get_local_pkgs_by_prefix(
        &mut self,
        prefix: &str,
    ) -> rusqlite::Result<Vec<ListPackage>> {
        let mut result = Vec::new();
        let mut stmt = self.db.prepare(
            "
                SELECT pkgname, comment
                  FROM local_pkg
            INNER JOIN local_repository
                    ON local_repository.id = local_pkg.repository_id
                 WHERE local_repository.prefix = :prefix
              ORDER BY pkgname ASC",
        )?;
        let rows =
            stmt.query_map_named(&[(":prefix", &prefix)], |row| ListPackage {
                pkgname: row.get(0),
                comment: row.get(1),
            })?;
        for row in rows {
            result.push(row?)
        }
        Ok(result)
    }
    pub fn get_remote_pkgs_by_prefix(
        &mut self,
        prefix: &str,
    ) -> rusqlite::Result<Vec<ListPackage>> {
        let mut result = Vec::new();
        let mut stmt = self.db.prepare(
            "
                SELECT pkgname, comment
                  FROM remote_pkg
            INNER JOIN remote_repository
                    ON remote_repository.id = remote_pkg.repository_id
                 WHERE remote_repository.prefix = :prefix",
        )?;
        let rows =
            stmt.query_map_named(&[(":prefix", &prefix)], |row| ListPackage {
                pkgname: row.get(0),
                comment: row.get(1),
            })?;
        for row in rows {
            result.push(row?)
        }
        Ok(result)
    }
}

impl LocalRepository {
    pub fn up_to_date(&self, mtime: i64, ntime: i32) -> bool {
        self.mtime == mtime && self.ntime == ntime
    }
}

impl RemoteRepository {
    pub fn up_to_date(&self, mtime: i64, summary_suffix: &str) -> bool {
        self.mtime == mtime && self.summary_suffix == summary_suffix
    }
}
