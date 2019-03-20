## pm(1) - A Package Manager for pkgsrc

pm manages binary packages created by pkgsrc.

It is still very much work in progress.  The initial goals are to implement
similar functionality as pkgin, but exploring different ways to approach the
problem.  Future work may also expand the remit to provide an alternative
interface to pkg\_install.

### Differences

Enhancements compared to pkgin:

* TOML configuration file for easy setup and extensibility.
* Supports multiple repositories and multiple prefixes.
* Designed for stricter conformance and safety (has already led to finding one
  `pkg_summary(5)` bug on SmartOS).
* Significantly faster!  It may use a bit more memory than pkgin but delivers
  a much faster experience, even with stricter database checks.  Timings for
  various commands in seconds compared on an HP N36L:

| Command |    pm |  pkgin | Improvement |
|--------:|------:|-------:|------------:|
|   avail | 0.20s |  0.75s |      **4x** |
|    list | 0.05s |  0.45s |      **9x** |
|  search | 0.15s |  0.80s |      **5x** |
|  update | 3.35s | 40.25s |     **12x** |

Disadvantages:

* Missing a lot of functionality!

### Usage

```console
USAGE:
    pm [FLAGS] [OPTIONS] <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information
    -v, --verbose    Enable verbose output

OPTIONS:
    -c, --config <config>    Use specified configuration file
    -p, --prefix <prefix>    Set default prefix

SUBCOMMANDS:
    avail     List available packages
    help      Prints this message or the help of the given subcommand(s)
    list      List installed packages
    search    Search available packages
    update    Update pkg_summary from each configured repository
```

### pm.toml

The configuration file for `pm` currently supports the following variables:

```toml
#
# When multiple repositories are configured, choose the ones associated with
# this prefix by default.  Specify -p on the command line to override.
#
default_prefix = "/opt/local"

#
# Enable verbose output.  Defaults to false.
#
verbose = true

#
# A fully-configured prefix.
#
#   * "path" is mandatory.
#   * "pkg_admin", "pkg_info", and "pkgdb" are optional, and will be calculated
#     during initialisation if not specified.
#
[[prefix]]
  path = "/opt/local"
  pkg_admin = "/opt/local/sbin/pkg_admin"
  pkg_info = "/opt/local/sbin/pkg_info"
  pkgdb = "/opt/local/pkg"
#
# A remote binary package repository for /opt/local.
#
#   * "url" is mandatory
#   * "summary_extension" is optional, and overrides the default set of
#     pkg_summary extensions to search for ("xz", "bz2", "gz").
#
  [[prefix.repository]]
    url = "https://pkgsrc.joyent.com/packages/SmartOS/trunk/x86_64/All"
    summary_extension = "gz"

#
# A second prefix using just the bare minimum, everything else is deduced at
# startup (at a small cost to runtime).
#
[[prefix]]
  path = "/opt/tools"
  [[prefix.repository]]
    url = "https://pkgsrc.joyent.com/packages/SmartOS/trunk/tools/All"
```

With the two prefixes configured above, you can perform queries on them:

```console
$ pm up
Recording packages installed under /opt/local
Creating https://pkgsrc.joyent.com/packages/SmartOS/trunk/x86_64/All
Recording packages installed under /opt/tools
Creating https://pkgsrc.joyent.com/packages/SmartOS/trunk/tools/All

: Using the default prefix
$ pm avail | wc -l
   20305
$ pm list | wc -l
     118
$ pm search ^vim-[0-9]
vim-8.1.1004         Vim editor (vi clone) without GUI

: Specifying the alternate prefix
$ pm -p /opt/tools avail | wc -l
     694
$ pm -p /opt/tools list | wc -l
      21
```
