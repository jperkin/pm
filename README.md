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
  pkg\_summary(5) bug on SmartOS).
* Significantly faster!  pm(1) may use a bit more memory than pkgin but
  delivers a much faster experience, even with stricter database checks.
  Timings for various commands compared on an HP N36L from cold:

| Command |   pm | pkgin |
|---------|------|-------|
|   avail | 0.2s |  0.7s |
|  update | 3.5s | 40.5s |

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
    -p, --prefix <prefix>    Set default prefix

SUBCOMMANDS:
    avail     List available packages
    help      Prints this message or the help of the given subcommand(s)
    update    Update pkg_summary from each configured repository
```

### pm.toml

The configuration file for `pm` currently supports the following variables:

```toml
# Enable verbose output.  Defaults to false.
verbose = true

# When multiple repositories are configured, choose the ones associated with
# this prefix by default.  Specify -p on the command line to override.
default_prefix = "/opt/local"

# A configured repository.  "url" and "prefix" are mandatory.
[[repository]]
url = "https://pkgsrc.joyent.com/packages/SmartOS/trunk/x86_64/All"
prefix = "/opt/local"
# Override the default ["xz", "bz2", "gz"] to force a particular extension.
summary_extension = "gz"

# Another configured repository.  Obviously these two repositories are
# incompatible and bad things would happen in real life, but are merely
# shown to provide an example.
[[repository]]
url = "https://pkgsrc.joyent.com/packages/Darwin/trunk/x86_64/All"
prefix = "/opt/pkg"
```
