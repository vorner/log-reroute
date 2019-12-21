#![doc(
    html_root_url = "https://docs.rs/log-reroute/0.1.4/log-reroute/",
    test(attr(deny(warnings)))
)]
#![deny(missing_docs)]
#![forbid(unsafe_code)]

//! Crate to reroute logging messages at runtime.
//!
//! The [`log`](https://crates.io/crates/log) logging facade allows to set only a single
//! destination during the whole lifetime of program. If you want to change the logging destination
//! multiple times, you can use [`Reroute`](struct.Reroute.html) (either directly, or through the
//! [`init`](fn.init.html) and [`reroute`](fn.reroute.html) functions).
//!
//! This may be useful if you want to log to `stderr` before you know where the main logs will go.
//!
//! ```rust
//! use fern::Dispatch;
//! use log::{info, LevelFilter};
//!
//! fn main() {
//!     log::set_max_level(LevelFilter::Off);
//!     info!("This log message goes nowhere");
//!     log_reroute::init().unwrap();
//!     info!("Still goes nowhere");
//!     // Log to stderr
//!     let early_logger = Dispatch::new().chain(std::io::stderr()).into_log().1;
//!     log_reroute::reroute_boxed(early_logger);
//!     info!("This one goes to stderr");
//!     // Load file name from config and log to that file
//!     let file = tempfile::tempfile().unwrap();
//!     let logger = Dispatch::new().chain(file).into_log().1;
//!     log_reroute::reroute_boxed(logger);
//!     info!("And this one to the file");
//!     // Stop logging
//!     log_reroute::reroute(log_reroute::Dummy);
//! }
//! ```

use std::sync::Arc;

use arc_swap::ArcSwap;
use log::{Log, Metadata, Record, SetLoggerError};
use once_cell::sync::Lazy;

/// A logger that doesn't log.
///
/// This is used to stub out the reroute in case no other log is set.
pub struct Dummy;

impl Log for Dummy {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        false
    }
    fn log(&self, _record: &Record) {}
    fn flush(&self) {}
}

/// A logging proxy.
///
/// This logger forwards all calls to currently configured slave logger.
///
/// The log routing is implemented in a lock-less and wait-less manner. While not necessarily faster
/// than using a mutex (unless there's a lot of contention and the slave logger also doesn't lock),
/// it makes it usable in some weird places (like a signal handler).
///
/// The rerouting (eg. changing the slave) is lock-less, but may have to wait for current logging
/// calls to end and concurrent reroutes will block each other.
///
/// # Note
///
/// When switching a logger, no care is taken to pair logging calls. In other words, it is possible
/// a message is written to the old logger and the new logger is flushed. This shouldn't matter in
/// practice, since a logger should flush itself once it is dropped.
pub struct Reroute {
    inner: ArcSwap<Box<dyn Log>>,
}

impl Reroute {
    /// Creates a new [`Reroute`] logger.
    ///
    /// No destination is set yet (it's sent to the [`Dummy`] instance), therefore all log messages
    /// are thrown away.
    pub fn new() -> Self {
        Default::default()
    }

    /// Sets a new slave logger.
    ///
    /// In case it is already in a box, you should prefer this method over
    /// [`reroute`](#fn.reroute), since there'll be less indirection.
    ///
    /// The old logger (if any) is flushed before dropping. In general, loggers should flush
    /// themselves on drop, but that may take time. This way we (mostly) ensure the cost of
    /// flushing is payed here.
    pub fn reroute_boxed(&self, log: Box<dyn Log>) {
        let old = self.inner.swap(Arc::new(log));
        old.flush();
    }

    /// Sets a new slave logger.
    pub fn reroute<L: Log + 'static>(&self, log: L) {
        self.reroute_boxed(Box::new(log));
    }

    /// Stubs out the logger.
    ///
    /// Sets the slave logger to one that does nothing (eg. [`Dummy`](struct.Dummy.html)).
    pub fn clear(&self) {
        self.reroute(Dummy);
    }
}

impl Log for Reroute {
    fn enabled(&self, metadata: &Metadata) -> bool {
        self.inner.load().enabled(metadata)
    }
    fn log(&self, record: &Record) {
        self.inner.load().log(record)
    }
    fn flush(&self) {
        self.inner.load().flush()
    }
}

impl Default for Reroute {
    /// Creates a reroute with a [`Dummy`](struct.Dummy.html) slave logger.
    fn default() -> Self {
        Self {
            inner: ArcSwap::from(Arc::new(Box::new(Dummy) as Box<dyn Log>)),
        }
    }
}

/// A global [`Reroute`](struct.Reroute.html) object.
///
/// This one is manipulated by the global functions:
///
/// * [`init`](fn.init.html)
/// * [`reroute`](fn.reroute.html)
/// * [`reroute_boxed`](fn.reroute_boxed.html)
pub static REROUTE: Lazy<Reroute> = Lazy::new(Reroute::default);

/// Installs the global [`Reroute`](struct.Reroute.html) instance into the
/// [`log`](https://crates.io/crates/log) facade.
///
/// Note that the default slave is [`Dummy`](struct.Dummy.html) and you need to call
/// [`reroute`](fn.reroute.html) or [`reroute_boxed`](fn.reroute_boxed.html).
pub fn init() -> Result<(), SetLoggerError> {
    log::set_logger(&*REROUTE)
}

/// Changes the slave of the global [`Reroute`](struct.Reroute.html) instance.
///
/// If you have a boxed logger, use [`reroute_boxed`](fn.reroute_boxed.html).
pub fn reroute<L: Log + 'static>(log: L) {
    REROUTE.reroute(log);
}

/// Changes the slave of the global [`Reroute`](struct.Reroute.html) instance.
pub fn reroute_boxed(log: Box<dyn Log>) {
    REROUTE.reroute_boxed(log)
}
