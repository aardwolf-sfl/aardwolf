use std::fmt;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use std::time::Instant;

pub struct Logger {
    file: File,
    timer: Instant,
}

impl Logger {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Logger {
            file: File::create(path).unwrap(),
            timer: Instant::now(),
        }
    }

    pub fn info<M: fmt::Display>(&mut self, message: M) {
        self.log("info", message);
    }

    // pub fn debug<M: fmt::Display>(&mut self, message: M) {
    //     self.log("debug", message);
    // }

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

pub struct PerfHandle<'a> {
    logger: &'a mut Logger,
    id: String,
    started: Instant,
}

impl<'a> PerfHandle<'a> {
    pub fn stop(self) {
        let elapsed = self.started.elapsed().as_secs_f32();
        self.logger
            .log("perf", format!("\"{}\" took {:.5} secs", self.id, elapsed));
    }
}
