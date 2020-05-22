use std::io::{self, Stdout, Write};

use chrono::prelude::*;
use serde::{Deserialize, Serialize};

use super::Ui;
use crate::api::Api;
use crate::data::statement::Loc;
use crate::plugins::{LocalizationItem, Rationale, RationaleChunk};
use crate::queries::Stmts;

#[derive(Serialize, Deserialize)]
struct Output {
    version: String,
    utc_time: DateTime<Utc>,
    local_time: DateTime<Local>,
    statements_count: usize,
    executed_statements_count: usize,
    plugins: Vec<Plugin>,
}

#[derive(Serialize, Deserialize)]
struct ErrorOutput {
    version: String,
    utc_time: DateTime<Utc>,
    local_time: DateTime<Local>,
    error: String,
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

pub struct JsonUi {
    terminal: Stdout,
    output: Output,
}

impl JsonUi {
    pub fn new() -> Self {
        JsonUi {
            terminal: io::stdout(),
            output: Output {
                version: String::from("v1"),
                utc_time: Utc::now(),
                local_time: Local::now(),
                statements_count: 0,
                executed_statements_count: 0,
                plugins: Vec::new(),
            },
        }
    }

    fn rationale(&mut self, rationale: &Rationale, api: &Api) -> (String, Vec<Location>) {
        let mut output = String::new();
        let mut anchors = Vec::new();

        for chunk in rationale.chunks() {
            match chunk {
                RationaleChunk::Text(text) => {
                    output.push_str(&text);
                }
                RationaleChunk::Anchor(anchor) => {
                    anchors.push(self.location(anchor, api));
                    output.push_str(&format!("[{}]", anchors.len()));
                }
            }
        }

        (output, anchors)
    }

    fn location(&self, loc: &Loc, api: &Api) -> Location {
        Location {
            file: api.file(&loc.file_id).unwrap().to_str().unwrap().to_owned(),
            line_begin: loc.line_begin,
            col_begin: loc.col_begin,
            line_end: loc.line_end,
            col_end: loc.col_end,
        }
    }
}

impl Ui for JsonUi {
    fn prolog(&mut self, api: &Api) {
        let stmts = api.query::<Stmts>().unwrap();
        self.output.statements_count = stmts.get_n_total();
        self.output.executed_statements_count = stmts.get_n_executed();
    }

    fn plugin(&mut self, id: &str, _api: &Api) {
        self.output.plugins.push(Plugin {
            name: id.to_owned(),
            results: Vec::new(),
        });
    }

    fn result(&mut self, item: &LocalizationItem, api: &Api) {
        let (rationale, anchors) = self.rationale(&item.rationale, api);
        let location = self.location(&item.loc, api);

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

    fn epilog(&mut self, _api: &Api) {
        write!(
            self.terminal,
            "{}",
            serde_json::to_string(&self.output).unwrap()
        )
        .unwrap();
    }

    fn error(&mut self, error: &str) {
        let error_output = ErrorOutput {
            version: self.output.version.clone(),
            utc_time: self.output.utc_time,
            local_time: self.output.local_time,
            error: error.to_owned(),
        };

        write!(
            self.terminal,
            "{}",
            serde_json::to_string(&error_output).unwrap()
        )
        .unwrap();
    }
}
