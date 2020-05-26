//! Aardwolf logging system.

use std::fmt;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use std::time::Instant;

/// A simple logging system.
pub struct Logger {
    file: File,
    timer: Instant,
}

impl Logger {
    /// Creates new logger which will write the messages into given file.
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Logger {
            file: File::create(path).unwrap(),
            timer: Instant::now(),
        }
    }

    /// Write given message at "info" level.
    pub fn info<M: fmt::Display>(&mut self, message: M) {
        self.log("info", message);
    }

    // pub fn debug<M: fmt::Display>(&mut self, message: M) {
    //     self.log("debug", message);
    // }

    /// Returns handle which, when stopped, will write time execution log with given message.
    pub fn perf<'a, S: Into<String>>(&'a mut self, id: S) -> PerfHandle<'a> {
        PerfHandle {
            logger: self,
            id: id.into(),
            started: Instant::now(),
        }
    }

    fn log<M: fmt::Display>(&mut self, header: &str, message: M) {
        writeln!(
            self.file,
            "[{:>9.5}] {}: {}",
            self.timer.elapsed().as_secs_f32(),
            header,
            message
        )
        .unwrap();
    }
}

/// Handle for time measurement logs.
pub struct PerfHandle<'a> {
    logger: &'a mut Logger,
    id: String,
    started: Instant,
}

impl<'a> PerfHandle<'a> {
    /// Writes the message along with the elapsed time.
    pub fn stop(self) {
        let elapsed = self.started.elapsed().as_secs_f32();
        self.logger
            .log("perf", format!("\"{}\" took {:.5} secs", self.id, elapsed));
    }
}
