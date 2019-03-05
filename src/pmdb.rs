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
const DB_VERSION: i64 = 20190305;

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
                version             INTEGER NOT NULL
            );
            CREATE TABLE local_repository (
                id                  INTEGER PRIMARY KEY,
                prefix              TEXT NOT NULL UNIQUE,
                mtime               INTEGER NOT NULL,
                ntime               INTEGER NOT NULL
            );
            CREATE TABLE local_pkg (
                id                  INTEGER PRIMARY KEY,
                repository_id       INTEGER NOT NULL,
                automatic           INTEGER NOT NULL,
                build_date          TEXT NOT NULL,
                categories          TEXT NOT NULL,
                comment             TEXT NOT NULL,
                description         TEXT NOT NULL,
                homepage            TEXT,
                license             TEXT,
                opsys               TEXT NOT NULL,
                os_version          TEXT NOT NULL,
                pkg_options         TEXT,
                pkgbase             TEXT NOT NULL,
                pkgname             TEXT NOT NULL,
                pkgpath             TEXT NOT NULL,
                pkgtools_version    TEXT NOT NULL,
                pkgversion          TEXT NOT NULL,
                size_pkg            INTEGER NOT NULL
            );
            CREATE TABLE local_conflicts (
                id                  INTEGER PRIMARY KEY,
                repository_id       INTEGER NOT NULL,
                pkg_id              INTEGER NOT NULL,
                conflicts           TEXT NOT NULL
            );
            CREATE TABLE local_depends (
                id                  INTEGER PRIMARY KEY,
                repository_id       INTEGER NOT NULL,
                pkg_id              INTEGER NOT NULL,
                depends             TEXT NOT NULL
            );
            CREATE TABLE local_provides (
                id                  INTEGER PRIMARY KEY,
                repository_id       INTEGER NOT NULL,
                pkg_id              INTEGER NOT NULL,
                provides            TEXT NOT NULL
            );
            CREATE TABLE local_requires (
                id                  INTEGER PRIMARY KEY,
                repository_id       INTEGER NOT NULL,
                pkg_id              INTEGER NOT NULL,
                requires            TEXT NOT NULL
            );
            CREATE TABLE remote_repository (
                id                  INTEGER PRIMARY KEY,
                prefix              TEXT NOT NULL,
                url                 TEXT NOT NULL UNIQUE,
                summary_suffix      TEXT NOT NULL,
                mtime               INTEGER NOT NULL
            );
            CREATE TABLE remote_pkg (
                id                  INTEGER PRIMARY KEY,
                repository_id       INTEGER NOT NULL,
                build_date          TEXT NOT NULL,
                categories          TEXT NOT NULL,
                comment             TEXT NOT NULL,
                description         TEXT NOT NULL,
                file_size           INTEGER NOT NULL,
                homepage            TEXT,
                license             TEXT,
                opsys               TEXT NOT NULL,
                os_version          TEXT NOT NULL,
                pkg_options         TEXT,
                pkgbase             TEXT NOT NULL,
                pkgname             TEXT NOT NULL,
                pkgpath             TEXT NOT NULL,
                pkgtools_version    TEXT NOT NULL,
                pkgversion          TEXT NOT NULL,
                size_pkg            INTEGER NOT NULL
            );
            CREATE TABLE remote_conflicts (
                id                  INTEGER PRIMARY KEY,
                repository_id       INTEGER NOT NULL,
                pkg_id              INTEGER NOT NULL,
                conflicts           TEXT NOT NULL
            );
            CREATE TABLE remote_depends (
                id                  INTEGER PRIMARY KEY,
                repository_id       INTEGER NOT NULL,
                pkg_id              INTEGER NOT NULL,
                depends             TEXT NOT NULL
            );
            CREATE TABLE remote_provides (
                id                  INTEGER PRIMARY KEY,
                repository_id       INTEGER NOT NULL,
                pkg_id              INTEGER NOT NULL,
                provides            TEXT NOT NULL
            );
            CREATE TABLE remote_requires (
                id                  INTEGER PRIMARY KEY,
                repository_id       INTEGER NOT NULL,
                pkg_id              INTEGER NOT NULL,
                requires            TEXT NOT NULL
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
        repository_id: i64,
        pkgs: &[SummaryEntry],
    ) -> rusqlite::Result<()> {
        let mut insert_pkg = tx.prepare(
            "INSERT INTO local_pkg
                    (repository_id, automatic, build_date, categories,
                     comment, description, homepage, license, opsys,
                     os_version, pkg_options, pkgbase, pkgname, pkgpath,
                     pkgtools_version, pkgversion, size_pkg)
             VALUES (:repository_id, :automatic, :build_date, :categories,
                     :comment, :description, :homepage, :license, :opsys,
                     :os_version, :pkg_options, :pkgbase, :pkgname, :pkgpath,
                     :pkgtools_version, :pkgversion, :size_pkg)",
        )?;
        let mut insert_conflicts = tx.prepare(
            "INSERT INTO local_conflicts
                    (repository_id, pkg_id, conflicts)
             VALUES (:repository_id, :pkg_id, :conflicts)",
        )?;
        let mut insert_depends = tx.prepare(
            "INSERT INTO local_depends
                    (repository_id, pkg_id, depends)
             VALUES (:repository_id, :pkg_id, :depends)",
        )?;
        let mut insert_provides = tx.prepare(
            "INSERT INTO local_provides
                    (repository_id, pkg_id, provides)
             VALUES (:repository_id, :pkg_id, :provides)",
        )?;
        let mut insert_requires = tx.prepare(
            "INSERT INTO local_requires
                    (repository_id, pkg_id, requires)
             VALUES (:repository_id, :pkg_id, :requires)",
        )?;
        for p in pkgs {
            /*
             * These values have all been checked earlier when inserted so
             * we are safe to unwrap.
             */
            insert_pkg.execute_named(&[
                (":repository_id", &repository_id),
                (":automatic", &p.automatic()),
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
            let pkg_id = tx.last_insert_rowid();
            if !p.conflicts().is_empty() {
                for conflicts in p.conflicts() {
                    insert_conflicts.execute_named(&[
                        (":repository_id", &repository_id),
                        (":pkg_id", &pkg_id),
                        (":conflicts", &conflicts),
                    ])?;
                }
            }
            if !p.depends().is_empty() {
                for depends in p.depends() {
                    insert_depends.execute_named(&[
                        (":repository_id", &repository_id),
                        (":pkg_id", &pkg_id),
                        (":depends", &depends),
                    ])?;
                }
            }
            if !p.provides().is_empty() {
                for provides in p.provides() {
                    insert_provides.execute_named(&[
                        (":repository_id", &repository_id),
                        (":pkg_id", &pkg_id),
                        (":provides", &provides),
                    ])?;
                }
            }
            if !p.requires().is_empty() {
                for requires in p.requires() {
                    insert_requires.execute_named(&[
                        (":repository_id", &repository_id),
                        (":pkg_id", &pkg_id),
                        (":requires", &requires),
                    ])?;
                }
            }
        }
        Ok(())
    }

    fn insert_remote_pkgs(
        tx: &rusqlite::Transaction,
        repository_id: i64,
        pkgs: &[SummaryEntry],
    ) -> rusqlite::Result<()> {
        let mut insert_pkg = tx.prepare(
            "INSERT INTO remote_pkg
                    (repository_id, build_date, categories, comment,
                     description, file_size, homepage, license, opsys,
                     os_version, pkg_options, pkgbase, pkgname, pkgpath,
                     pkgtools_version, pkgversion, size_pkg)
             VALUES (:repository_id, :build_date, :categories, :comment,
                     :description, :file_size, :homepage, :license, :opsys,
                     :os_version, :pkg_options, :pkgbase, :pkgname, :pkgpath,
                     :pkgtools_version, :pkgversion, :size_pkg)",
        )?;
        let mut insert_conflicts = tx.prepare(
            "INSERT INTO remote_conflicts
                    (repository_id, pkg_id, conflicts)
             VALUES (:repository_id, :pkg_id, :conflicts)",
        )?;
        let mut insert_depends = tx.prepare(
            "INSERT INTO remote_depends
                    (repository_id, pkg_id, depends)
             VALUES (:repository_id, :pkg_id, :depends)",
        )?;
        let mut insert_provides = tx.prepare(
            "INSERT INTO remote_provides
                    (repository_id, pkg_id, provides)
             VALUES (:repository_id, :pkg_id, :provides)",
        )?;
        let mut insert_requires = tx.prepare(
            "INSERT INTO remote_requires
                    (repository_id, pkg_id, requires)
             VALUES (:repository_id, :pkg_id, :requires)",
        )?;

        for p in pkgs {
            /*
             * These values have all been checked earlier when inserted so
             * we are safe to unwrap.
             */
            insert_pkg.execute_named(&[
                (":repository_id", &repository_id),
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
            let pkg_id = tx.last_insert_rowid();
            if !p.conflicts().is_empty() {
                for conflicts in p.conflicts() {
                    insert_conflicts.execute_named(&[
                        (":repository_id", &repository_id),
                        (":pkg_id", &pkg_id),
                        (":conflicts", &conflicts),
                    ])?;
                }
            }
            if !p.depends().is_empty() {
                for depends in p.depends() {
                    insert_depends.execute_named(&[
                        (":repository_id", &repository_id),
                        (":pkg_id", &pkg_id),
                        (":depends", &depends),
                    ])?;
                }
            }
            if !p.provides().is_empty() {
                for provides in p.provides() {
                    insert_provides.execute_named(&[
                        (":repository_id", &repository_id),
                        (":pkg_id", &pkg_id),
                        (":provides", &provides),
                    ])?;
                }
            }
            if !p.requires().is_empty() {
                for requires in p.requires() {
                    insert_requires.execute_named(&[
                        (":repository_id", &repository_id),
                        (":pkg_id", &pkg_id),
                        (":requires", &requires),
                    ])?;
                }
            }
        }
        Ok(())
    }

    fn delete_local_pkgs(
        tx: &rusqlite::Transaction,
        repository_id: i64,
    ) -> rusqlite::Result<()> {
        let delete_tables = [
            "local_pkg",
            "local_conflicts",
            "local_depends",
            "local_provides",
            "local_requires",
        ];
        for table in &delete_tables {
            let sql = format!(
                "DELETE FROM {} WHERE repository_id = :repository_id",
                &table
            );
            tx.execute_named(&sql, &[(":repository_id", &repository_id)])?;
        }
        Ok(())
    }

    fn delete_remote_pkgs(
        tx: &rusqlite::Transaction,
        repository_id: i64,
    ) -> rusqlite::Result<()> {
        let delete_tables = [
            "remote_pkg",
            "remote_conflicts",
            "remote_depends",
            "remote_provides",
            "remote_requires",
        ];
        for table in &delete_tables {
            let sql = format!(
                "DELETE FROM {} WHERE repository_id = :repository_id",
                &table
            );
            tx.execute_named(&sql, &[(":repository_id", &repository_id)])?;
        }
        Ok(())
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

            let repository_id = tx.last_insert_rowid();
            PMDB::insert_local_pkgs(&tx, repository_id, &pkgs)?;
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

            let repository_id = tx.last_insert_rowid();
            PMDB::insert_remote_pkgs(&tx, repository_id, &pkgs)?;
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
            let repository_id = tx.query_row_named(
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
            PMDB::delete_local_pkgs(&tx, repository_id)?;
            PMDB::insert_local_pkgs(&tx, repository_id, &pkgs)?;

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
            let repository_id = tx.query_row_named(
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
            PMDB::delete_remote_pkgs(&tx, repository_id)?;
            PMDB::insert_remote_pkgs(&tx, repository_id, &pkgs)?;

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
                 WHERE remote_repository.prefix = :prefix
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
