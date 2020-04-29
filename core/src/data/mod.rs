pub mod access;
mod consts;
pub mod module;
pub mod parser;
pub mod statement;
pub mod tests;
pub mod trace;
pub mod types;
pub mod values;

use std::io::BufRead;

use module::Modules;
use tests::TestSuite;
use trace::Trace;

pub struct RawData {
    pub modules: Modules,
    pub trace: Trace,
    pub test_suite: TestSuite,
}

impl RawData {
    pub fn new(modules: Modules, trace: Trace, test_suite: TestSuite) -> Self {
        RawData {
            modules,
            trace,
            test_suite,
        }
    }

    pub fn parse<'a, R1: BufRead + 'a, R2: BufRead, R3: BufRead>(
        module_files: impl Iterator<Item = &'a mut R1>,
        trace_file: &mut R2,
        test_suite_file: &mut R3,
    ) -> parser::ParseResult<RawData> {
        let mut modules = Modules::new();
        let mut trace = Trace::new();
        let mut test_suite = TestSuite::new();

        for module_file in module_files {
            parser::parse_module(module_file, &mut modules)?;
        }

        parser::parse_trace(trace_file, &mut trace)?;
        parser::parse_test_suite(test_suite_file, &mut test_suite)?;

        Ok(RawData::new(modules, trace, test_suite))
    }
}
