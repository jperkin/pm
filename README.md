## pm(1) - A Package Manager for pkgsrc

pm manage binary packages created by pkgsrc.

It is still very much work in progress.  The initial goals are to implement
the same functionality as pkgin, but future work may also expand the remit to
provide an alternative interface to pkg\_install.

### pm.toml

The configuration file for `pm` currently supports as follows:

```toml
# Enable verbose output.  Defaults to false.
verbose = true

# A configured repository.  summary_extension allows you to manually specify
# which pkg_summary.<ext> to use, the default is ["xz", "bz2", "gz"].
[[repository]]
url = "https://pkgsrc.joyent.com/packages/SmartOS/trunk/x86_64/All"
summary_extension = "gz"

# Another configured repository.  Obviously these two repositories are
# incompatible and bad things would happen in real life, but are merely
# shown to provide an example.
[[repository]]
url = "https://pkgsrc.joyent.com/packages/SmartOS/2018Q4/x86_64/All"
```
