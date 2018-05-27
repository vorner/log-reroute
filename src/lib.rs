extern crate arc_swap;
#[macro_use]
extern crate lazy_static;
extern crate log;

use std::sync::Arc;

use arc_swap::ArcSwap;
use log::{Log, Metadata, Record, SetLoggerError};

pub struct Dummy;

impl Log for Dummy {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        false
    }
    fn log(&self, _record: &Record) {}
    fn flush(&self) {}
}

pub struct Reroute {
    inner: ArcSwap<Box<Log>>,
}

impl Reroute {
    pub fn reroute_boxed(&self, log: Box<Log>) {
        self.inner.store(Arc::new(log));
    }
    pub fn reroute<L: Log + 'static>(&self, log: L) {
        self.reroute_boxed(Box::new(log));
    }
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
    fn default() -> Self {
        Self {
            inner: ArcSwap::from(Arc::new(Box::new(Dummy) as Box<Log>)),
        }
    }
}

lazy_static! {
    pub static ref REROUTE: Reroute = Reroute::default();
}

pub fn init() -> Result<(), SetLoggerError> {
    log::set_logger(&*REROUTE)
}

pub fn reroute<L: Log + 'static>(log: L) {
    REROUTE.reroute(log);
}

pub fn reroute_boxed(log: Box<Log>) {
    REROUTE.reroute_boxed(log)
}
