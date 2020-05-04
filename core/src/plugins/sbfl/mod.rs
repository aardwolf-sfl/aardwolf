mod metrics;

use std::collections::HashMap;

use yaml_rust::Yaml;

use self::metrics::{from_opts, Metric};
use crate::api::Api;
use crate::plugins::{
    AardwolfPlugin, IrrelevantItems, LocalizationItem, PluginError, PluginInitError, Rationale,
    Results,
};

struct Counters {
    pub aep: f32,
    pub anp: f32,
    pub aef: f32,
    pub anf: f32,
}

impl Counters {
    fn new() -> Self {
        Counters {
            aep: 0.0,
            anp: 0.0,
            aef: 0.0,
            anf: 0.0,
        }
    }

    fn as_spectrum(&self) -> (f32, f32, f32, f32) {
        (self.aep, self.anp, self.aef, self.anf)
    }
}

pub struct Sbfl {
    metric: Box<dyn Metric>,
}

impl AardwolfPlugin for Sbfl {
    fn init<'data>(_api: &'data Api, opts: &HashMap<String, Yaml>) -> Result<Self, PluginInitError>
    where
        Self: Sized,
    {
        Ok(Sbfl {
            metric: from_opts(opts)?,
        })
    }

    fn run_loc<'data, 'param>(
        &self,
        api: &'data Api,
        results: &'param mut Results,
        irrelevant: &'param IrrelevantItems,
    ) -> Result<(), PluginError> {
        let stmts = api.get_stmts();
        let tests = api.get_tests();
        let spectra = api.get_spectra();

        let mut rationale = Rationale::new();
        rationale
            .add_text("The element is executed more in failing tests and less in passing tests.");

        let mut counters = HashMap::new();

        for stmt in stmts
            .iter_stmts()
            .filter(|stmt| irrelevant.is_stmt_relevant(stmt.as_ref()))
        {
            let stmt_counters = counters.entry(stmt).or_insert(Counters::new());

            for test in tests.iter_names() {
                match (
                    spectra.is_executed_in(test, stmt.as_ref()),
                    tests.is_passed(test),
                ) {
                    (false, false) => stmt_counters.anf += 1.0,
                    (false, true) => stmt_counters.anp += 1.0,
                    (true, false) => stmt_counters.aef += 1.0,
                    (true, true) => stmt_counters.aep += 1.0,
                }
            }

            let spectrum = stmt_counters.as_spectrum();

            results.add(
                LocalizationItem::new(
                    stmt.as_ref().loc,
                    *stmt,
                    self.metric
                        .calc(spectrum.0, spectrum.1, spectrum.2, spectrum.3),
                    rationale.clone(),
                )
                .unwrap(),
            );
        }

        Ok(())
    }
}
