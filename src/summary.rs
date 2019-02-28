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
 * summary.rs - handle pkg_summary(5) parsing
 */

use std::io::Write;

#[derive(Debug)]
pub struct SummaryStream {
    buf: Vec<u8>,
    entries: Vec<SummaryEntry>,
    cur: usize,
}

/*
 * i64 types are used even though the values cannot be expressed as negative
 * as it avoids having to convert to sqlite which does not support u64.
 */
#[derive(Debug, Default)]
pub struct SummaryEntry {
    build_date: String,
    categories: Vec<String>,
    comment: String,
    conflicts: Vec<String>,
    depends: Vec<String>,
    description: Vec<String>,
    file_cksum: Option<String>,
    file_name: Option<String>,
    file_size: Option<i64>,
    /* Non-standard field, used to split pkgname into constituent parts */
    fullpkgname: String,
    homepage: Option<String>,
    license: Option<String>,
    machine_arch: String,
    opsys: String,
    os_version: String,
    pkg_options: Option<String>,
    pkgname: String,
    pkgpath: String,
    pkgtools_version: String,
    /* Non-standard field, used to split pkgname into constituent parts */
    pkgversion: String,
    prev_pkgpath: Option<String>,
    provides: Vec<String>,
    requires: Vec<String>,
    size_pkg: Option<i64>,
    supersedes: Vec<String>,
}

impl SummaryEntry {
    pub fn new() -> SummaryEntry {
        let sum: SummaryEntry = Default::default();
        sum
    }

    /*
     * XXX: Some are Strings, some are str due to unwrapping Option, I need to
     * figure out what's best here depending on how they will be used.
     */
    pub fn build_date(&self) -> &String {
        &self.build_date
    }
    pub fn categories(&self) -> &Vec<String> {
        &self.categories
    }
    pub fn comment(&self) -> &String {
        &self.comment
    }
    #[allow(dead_code)]
    pub fn conflicts(&self) -> &Vec<String> {
        &self.conflicts
    }
    #[allow(dead_code)]
    pub fn depends(&self) -> &Vec<String> {
        &self.depends
    }
    pub fn description(&self) -> &Vec<String> {
        &self.description
    }
    #[allow(dead_code)]
    pub fn file_cksum(&self) -> &str {
        match &self.file_cksum {
            Some(s) => s.as_str(),
            None => "",
        }
    }
    #[allow(dead_code)]
    pub fn file_name(&self) -> &str {
        match &self.file_name {
            Some(s) => s.as_str(),
            None => "",
        }
    }
    pub fn file_size(&self) -> &Option<i64> {
        &self.file_size
    }
    pub fn fullpkgname(&self) -> &String {
        &self.fullpkgname
    }
    pub fn homepage(&self) -> &str {
        match &self.homepage {
            Some(s) => s.as_str(),
            None => "",
        }
    }
    pub fn license(&self) -> &str {
        match &self.license {
            Some(s) => s.as_str(),
            None => "",
        }
    }
    #[allow(dead_code)]
    pub fn machine_arch(&self) -> &String {
        &self.machine_arch
    }
    pub fn opsys(&self) -> &String {
        &self.opsys
    }
    pub fn os_version(&self) -> &String {
        &self.os_version
    }
    pub fn pkg_options(&self) -> &str {
        match &self.pkg_options {
            Some(s) => s.as_str(),
            None => "",
        }
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
    #[allow(dead_code)]
    pub fn prev_pkgpath(&self) -> &str {
        match &self.prev_pkgpath {
            Some(s) => s.as_str(),
            None => "",
        }
    }
    #[allow(dead_code)]
    pub fn provides(&self) -> &Vec<String> {
        &self.provides
    }
    #[allow(dead_code)]
    pub fn requires(&self) -> &Vec<String> {
        &self.requires
    }
    pub fn size_pkg(&self) -> &Option<i64> {
        &self.size_pkg
    }
    #[allow(dead_code)]
    pub fn supersedes(&self) -> &Vec<String> {
        &self.supersedes
    }

    pub fn parse_entry(
        &mut self,
        key: &str,
        value: &str,
    ) -> Result<(), &'static str> {
        let valstring = value.to_string();
        let vali64 = value.parse::<i64>();
        /*
         * Ideally there'd be a way to automatically parse these based on
         * the struct member names and their types, but so far I haven't
         * found a way to do that.
         */
        match key {
            "BUILD_DATE" => self.build_date = valstring,
            "CATEGORIES" => self.categories.push(valstring),
            "COMMENT" => self.comment = valstring,
            "CONFLICTS" => self.conflicts.push(valstring),
            "DEPENDS" => self.depends.push(valstring),
            "DESCRIPTION" => self.description.push(valstring),
            "FILE_CKSUM" => self.file_cksum = Some(valstring),
            "FILE_NAME" => self.file_name = Some(valstring),
            "FILE_SIZE" => self.file_size = Some(vali64.unwrap()),
            "HOMEPAGE" => self.homepage = Some(valstring),
            "LICENSE" => self.license = Some(valstring),
            "MACHINE_ARCH" => self.machine_arch = valstring,
            "OPSYS" => self.opsys = valstring,
            "OS_VERSION" => self.os_version = valstring,
            "PKG_OPTIONS" => self.pkg_options = Some(valstring),
            /* Split PKGNAME into constituent parts */
            "PKGNAME" => {
                let splitstring = valstring.clone();
                self.fullpkgname = valstring;
                let v: Vec<&str> = splitstring.rsplitn(2, '-').collect();
                self.pkgname = v[1].to_string();
                self.pkgversion = v[0].to_string();
            }
            "PKGPATH" => self.pkgpath = valstring,
            "PKGTOOLS_VERSION" => self.pkgtools_version = valstring,
            "PREV_PKGPATH" => self.prev_pkgpath = Some(valstring),
            "PROVIDES" => self.provides.push(valstring),
            "REQUIRES" => self.requires.push(valstring),
            "SIZE_PKG" => self.size_pkg = Some(vali64.unwrap()),
            "SUPERSEDES" => self.supersedes.push(valstring),
            _ => return Err("Unhandled key"),
        }
        Ok(())
    }

    /*
     * Ensure required fields (as per pkg_summary(5)) are set
     */
    pub fn validate(&self) -> Result<(), &'static str> {
        /*
         * Again, there's probably a fancy way to match these.
         */
        if self.build_date.is_empty() {
            return Err("Missing BUILD_DATE");
        }
        if self.categories.is_empty() {
            return Err("Missing CATEGORIES");
        }
        if self.comment.is_empty() {
            return Err("Missing COMMENT");
        }
        if self.description.is_empty() {
            return Err("Missing DESCRIPTION");
        }
        if self.machine_arch.is_empty() {
            return Err("Missing MACHINE_ARCH");
        }
        if self.opsys.is_empty() {
            return Err("Missing OPSYS");
        }
        if self.os_version.is_empty() {
            return Err("Missing OS_VERSION");
        }
        if self.pkgname.is_empty() {
            return Err("Missing PKGNAME");
        }
        if self.pkgpath.is_empty() {
            return Err("Missing PKGPATH");
        }
        if self.pkgtools_version.is_empty() {
            return Err("Missing PKGTOOLS_VERSION");
        }
        /*
         * SIZE_PKG is a required field but a size of 0 is valid (meta-pkgs)
         * so it needs to be an Option().
         */
        if self.size_pkg.is_none() {
            return Err("Missing SIZE_PKG");
        }
        Ok(())
    }
}

impl SummaryStream {
    pub fn new() -> SummaryStream {
        SummaryStream {
            buf: vec![],
            entries: vec![],
            cur: 0,
        }
    }

    /*
     * At some point this needs to be a Read trait but the different types
     * of encoders are making it difficult for now.
     */
    pub fn slurp(
        &mut self,
        sumext: &str,
        res: reqwest::Response,
    ) -> std::io::Result<u64> {
        if sumext == "xz" {
            let mut decomp = xz2::read::XzDecoder::new(res);
            std::io::copy(&mut decomp, self)
        } else if sumext == "bz2" {
            let mut decomp = bzip2::read::BzDecoder::new(res);
            std::io::copy(&mut decomp, self)
        } else if sumext == "gz" {
            let mut decomp = flate2::read::GzDecoder::new(res);
            std::io::copy(&mut decomp, self)
        } else {
            panic!("Unsupported summary_extension");
        }
    }

    pub fn entries(&self) -> &Vec<SummaryEntry> {
        &self.entries
    }

    pub fn parse(&mut self) {
        let allsum = match std::str::from_utf8(&self.buf) {
            Ok(s) => s,
            _ => panic!("Err parse"),
        };
        let v: Vec<&str> = allsum.split_terminator("\n\n").collect();
        for sum_entry in v {
            let mut sum = SummaryEntry::new();
            for line in sum_entry.lines() {
                let kv: Vec<&str> = line.splitn(2, '=').collect();
                match sum.parse_entry(kv[0], kv[1]) {
                    Ok(_) => {}
                    Err(err) => {
                        println!("PARSE ERROR: {}", err);
                        println!("{:#?}", sum);
                    }
                }
            }
            match sum.validate() {
                Ok(_) => {
                    self.entries.push(sum);
                }
                Err(err) => {
                    println!("VALIDATE ERROR: {}", err);
                    println!("{:#?}", sum);
                }
            }
        }
    }
}

impl Write for SummaryStream {
    /*
     * For now we just buffer the whole thing into memory and then process
     * it at the end.  A 20,000 package uncompressed pkg_summary is in the
     * region of 25MB, which is fine, and it makes everything a lot simpler
     * and (possibly) faster.
     *
     * If we want to move to a streaming model in the future then we'll need
     * to account for incomplete records and split on "\n\n" records, as well
     * as figuring out a way to differentiate between EOF and a record that
     * happens to end at the end of a buffer.
     */
    fn write(&mut self, inbuf: &[u8]) -> std::io::Result<usize> {
        self.buf.extend_from_slice(inbuf);
        Ok(inbuf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
