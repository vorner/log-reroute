# Log-reroute

[![Travis Build Status](https://api.travis-ci.org/vorner/log-reroute.png?branch=master)](https://travis-ci.org/vorner/log-reroute)
[![AppVeyor Build status](https://ci.appveyor.com/api/projects/status/a7jxutjn94266ift/branch/master?svg=true)](https://ci.appveyor.com/project/vorner/log-reroute/branch/master)

The [`log`](https://crates.io/crates/log) allows setting the target logger, but
only once during the lifetime of the application. This library helps with that
by providing a logger proxy. The logger behind the proxy can be switched as
necessary.
