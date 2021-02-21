# Log-reroute

[![Actions Status](https://github.com/vorner/log-reroute/workflows/test/badge.svg)](https://github.com/vorner/log-reroute/actions)
[![codecov](https://codecov.io/gh/vorner/log-reroute/branch/main/graph/badge.svg?token=AVGQ6JM0VU)](https://codecov.io/gh/vorner/log-reroute)
[![docs](https://docs.rs/log-reroute/badge.svg)](https://docs.rs/log-reroute)

The [`log`](https://crates.io/crates/log) allows setting the target logger, but
only once during the lifetime of the application. This library helps with that
by providing a logger proxy. The logger behind the proxy can be switched as
necessary.
