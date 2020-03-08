use std::io::{self, Stdout, Write};

use chrono::prelude::*;
use serde::{Deserialize, Serialize};

use super::Ui;
use crate::api::Api;
use crate::plugins::{LocalizationItem, Rationale, RationaleChunk};
use crate::raw::data::Loc;

#[derive(Serialize, Deserialize)]
struct Output {
    version: String,
    utc_time: DateTime<Utc>,
    local_time: DateTime<Local>,
    plugins: Vec<Plugin>,
}

#[derive(Serialize, Deserialize)]
struct Plugin {
    name: String,
    results: Vec<Hypothesis>,
}

#[derive(Serialize, Deserialize)]
struct Hypothesis {
    location: Location,
    suspiciousness: f32,
    rationale: String,
    anchors: Vec<Location>,
}

#[derive(Serialize, Deserialize)]
struct Location {
    file: String,
    line_begin: u32,
    col_begin: u32,
    line_end: u32,
    col_end: u32,
}

pub struct JsonUi<'data> {
    api: &'data Api<'data>,
    terminal: Stdout,
    output: Output,
}

impl<'data> JsonUi<'data> {
    pub fn new(api: &'data Api<'data>) -> Self {
        JsonUi {
            api,
            terminal: io::stdout(),
            output: Output {
                version: String::from("v1"),
                utc_time: Utc::now(),
                local_time: Local::now(),
                plugins: Vec::new(),
            },
        }
    }

    fn rationale(&mut self, rationale: &Rationale) -> (String, Vec<Location>) {
        let mut output = String::new();
        let mut anchors = Vec::new();

        for chunk in rationale.chunks() {
            match chunk {
                RationaleChunk::Text(text) => {
                    output.push_str(&text);
                }
                RationaleChunk::Anchor(anchor) => {
                    anchors.push(self.location(anchor));
                    output.push_str(&format!("[{}]", anchors.len()));
                }
            }
        }

        (output, anchors)
    }

    fn location(&self, loc: &Loc) -> Location {
        Location {
            file: self
                .api
                .get_filepath(loc.file_id)
                .unwrap()
                .to_str()
                .unwrap()
                .to_owned(),
            line_begin: loc.line_begin,
            col_begin: loc.col_begin,
            line_end: loc.line_end,
            col_end: loc.col_end,
        }
    }
}

impl<'data> Ui<'data> for JsonUi<'data> {
    fn plugin(&mut self, id: &str) {
        self.output.plugins.push(Plugin {
            name: id.to_owned(),
            results: Vec::new(),
        });
    }

    fn result(&mut self, item: &LocalizationItem<'data>) {
        let (rationale, anchors) = self.rationale(&item.rationale);
        let location = self.location(&item.loc);

        self.output
            .plugins
            .last_mut()
            .unwrap()
            .results
            .push(Hypothesis {
                location,
                suspiciousness: item.score,
                rationale,
                anchors,
            });
    }

    fn epilog(&mut self) {
        write!(
            self.terminal,
            "{}",
            serde_json::to_string(&self.output).unwrap()
        )
        .unwrap();
    }
}
