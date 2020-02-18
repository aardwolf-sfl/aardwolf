use std::collections::HashMap;

use yaml_rust::Yaml;

use crate::api::Api;
use crate::plugins::{AardwolfPlugin, LocalizationItem, PluginInitError, Rationale};

pub const SAFE_DENOMINATOR: f32 = 0.5;

struct Counters {
    pub nep: f32,
    pub nnp: f32,
    pub nef: f32,
    pub nnf: f32,
}

impl Counters {
    fn new() -> Self {
        Counters {
            nep: 0.0,
            nnp: 0.0,
            nef: 0.0,
            nnf: 0.0,
        }
    }

    fn dstar(&self) -> f32 {
        self.nef.powf(2.0) / (self.nnf + self.nep + SAFE_DENOMINATOR)
    }

    fn ochiai(&self) -> f32 {
        self.nef / (((self.nef + self.nnf) * (self.nef + self.nep)).sqrt() + SAFE_DENOMINATOR)
    }

    fn tarantula(&self) -> f32 {
        let expr1 = self.nef / (self.nef + self.nnf + SAFE_DENOMINATOR);
        let expr2 = self.nep / (self.nep + self.nnp + SAFE_DENOMINATOR);
        expr1 / (expr1 + expr2 + SAFE_DENOMINATOR)
    }
}

pub struct Sbfl {
    metric: fn(&Counters) -> f32,
}

impl AardwolfPlugin for Sbfl {
    fn init<'a>(_api: &'a Api<'a>, opts: &HashMap<String, Yaml>) -> Result<Self, PluginInitError>
    where
        Self: Sized,
    {
        let metric = match opts.get("metric").and_then(|metric| metric.as_str()) {
            Some("dstar") => Counters::dstar,
            Some("ochiai") => Counters::ochiai,
            Some("tarantula") => Counters::tarantula,
            None => Counters::dstar,
            Some(unknown) => return Err(format!("Unknown metric '{}'.", unknown)),
        };

        Ok(Sbfl { metric })
    }

    fn run_loc<'a, 'b>(&'b self, api: &'a Api<'a>) -> Vec<LocalizationItem<'b>> {
        let stmts = api.get_stmts().unwrap();
        let tests = api.get_tests().unwrap();
        let spectra = api.get_spectra().unwrap();

        let mut rationale = Rationale::new();
        rationale
            .add_text("The element is executed more in failing tests and less in passing tests.");

        let mut counters = HashMap::new();

        stmts
            .iter_stmts()
            .map(|stmt| {
                let stmt_counters = counters.entry(stmt).or_insert(Counters::new());

                for test in tests.iter_names() {
                    match (spectra.is_executed_in(test, stmt), tests.is_passed(test)) {
                        (false, false) => stmt_counters.nnf += 1.0,
                        (false, true) => stmt_counters.nnp += 1.0,
                        (true, false) => stmt_counters.nef += 1.0,
                        (true, true) => stmt_counters.nep += 1.0,
                    }
                }

                LocalizationItem::new(stmt.loc, (self.metric)(stmt_counters), rationale.clone())
                    .unwrap()
            })
            .collect::<Vec<_>>()
    }
}
