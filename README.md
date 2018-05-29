# Log-reroute

The [`log`](https://crates.io/crates/log) allows setting the target logger, but
only once during the lifetime of the application. This library helps with that
by providing a logger proxy. The logger behind the proxy can be switched as
necessary.
