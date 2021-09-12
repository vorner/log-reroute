#![doc(test(attr(deny(warnings))))]
#![warn(missing_docs)]
#![forbid(unsafe_code)]
// We have Arc<Box<dyn ...>>. It is redundant allocation from a PoV, but arc-swap needs
// Arc<S: Sized>, so we don't have much choice in that matter.
#![allow(clippy::redundant_allocation)]

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
//!     // Enable logging of Debug and more severe messages.
//!     log::set_max_level(LevelFilter::Debug);
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
/// than using a mutex, the performance should be more predictable and stable in face of contention
/// from multiple threads. This assumes the slave logger also doesn't lock.
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
    /// flushing is paid here.
    pub fn reroute_boxed(&self, log: Box<dyn Log>) {
        self.reroute_arc(Arc::new(log))
    }

    /// Sets a slave logger.
    ///
    /// Another variant of [`reroute_boxed`][Reroute::reroute_boxed], accepting the inner
    /// representation. This can be combined with a previous [`get`][Reroute::get].
    ///
    /// Note that the `Arc<Box<dyn Log>>` (double indirection) is necessary evil, since arc-swap
    /// can't accept `!Sized` types.
    pub fn reroute_arc(&self, log: Arc<Box<dyn Log>>) {
        let old = self.inner.swap(log);
        old.flush();
    }

    /// Sets a new slave logger.
    ///
    /// See [`reroute_boxed`][Reroute::reroute_boxed] for more details.
    pub fn reroute<L: Log + 'static>(&self, log: L) {
        self.reroute_boxed(Box::new(log));
    }

    /// Stubs out the logger.
    ///
    /// Sets the slave logger to one that does nothing (eg. [`Dummy`](struct.Dummy.html)).
    pub fn clear(&self) {
        self.reroute(Dummy);
    }

    /// Gives access to the inner logger.
    ///
    /// # Notes
    ///
    /// The logger may be still in use by other threads, etc. It may be in use even after the
    /// current thread called [`clear`][Reroute::clear] or [`reroute`][Reroute::reroute], at least
    /// for a while.
    pub fn get(&self) -> Arc<Box<dyn Log>> {
        self.inner.load_full()
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
