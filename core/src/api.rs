use std::env;
use std::path::PathBuf;

use lazycell::LazyCell;

use crate::data::{types::FileId, RawData};
use crate::structures::*;

#[derive(Debug)]
pub enum EmptyDataReason {
    Static,
    Runtime,
    TestSuite,
}

#[derive(Debug)]
pub enum InvalidData {
    NoFailingTest,
    Empty(EmptyDataReason),
}

pub struct Api {
    data: RawData,
    stmts: LazyCell<Stmts>,
    tests: LazyCell<Tests>,
    def_use: LazyCell<DefUse>,
    spectra: LazyCell<Spectra>,
    cfgs: LazyCell<Cfgs>,
    pdgs: LazyCell<Pdgs>,
    vars: LazyCell<Vars>,
}

macro_rules! get_lazy_fallible {
    ($api:expr, $prop:ident) => {{
        if !($api).$prop.filled() {
            match $api.make() {
                Ok(prop) => {
                    ($api).$prop.fill(prop).ok();
                }
                // TODO: Save the error to api in order to warn the user.
                Err(_) => return None,
            }
        }

        ($api).$prop.borrow()
    }};
}

macro_rules! get_lazy_infallible {
    ($api:expr, $prop:ident) => {{
        if !($api).$prop.filled() {
            ($api).$prop.fill($api.make().unwrap()).ok();
        }

        ($api).$prop.borrow().unwrap()
    }};
}

impl Api {
    pub(crate) fn new(data: RawData) -> Result<Self, InvalidData> {
        if data.modules.files.is_empty() || data.modules.functions.is_empty() {
            Err(InvalidData::Empty(EmptyDataReason::Static))
        } else if data.trace.trace.is_empty() {
            Err(InvalidData::Empty(EmptyDataReason::Runtime))
        } else if data.test_suite.tests.is_empty() {
            Err(InvalidData::Empty(EmptyDataReason::TestSuite))
        } else if data
            .test_suite
            .tests
            .values()
            .all(|status| status.is_passed())
        {
            Err(InvalidData::NoFailingTest)
        } else {
            Ok(Api {
                data,
                stmts: LazyCell::new(),
                tests: LazyCell::new(),
                def_use: LazyCell::new(),
                spectra: LazyCell::new(),
                cfgs: LazyCell::new(),
                pdgs: LazyCell::new(),
                vars: LazyCell::new(),
            })
        }
    }

    pub fn make<T: FromRawData>(&self) -> Result<T, FromRawDataError> {
        T::from_raw(&self.data, &self)
    }

    pub fn get_stmts(&self) -> &Stmts {
        get_lazy_infallible!(self, stmts)
    }

    pub fn get_tests(&self) -> &Tests {
        get_lazy_infallible!(self, tests)
    }

    pub fn get_def_use(&self) -> &DefUse {
        get_lazy_infallible!(self, def_use)
    }

    pub fn get_spectra(&self) -> &Spectra {
        get_lazy_infallible!(self, spectra)
    }

    pub fn get_cfgs(&self) -> &Cfgs {
        get_lazy_infallible!(self, cfgs)
    }

    pub fn get_pdgs(&self) -> &Pdgs {
        get_lazy_infallible!(self, pdgs)
    }

    pub fn get_vars(&self) -> Option<&Vars> {
        get_lazy_fallible!(self, vars)
    }

    pub fn get_filepath(&self, file_id: &FileId) -> Option<PathBuf> {
        let ptr = self.data.modules.files.get(file_id)?;
        let raw = PathBuf::from(ptr.as_ref());

        raw.canonicalize()
            .ok()?
            .strip_prefix(env::current_dir().ok()?)
            .ok()
            .map(|path| path.to_path_buf())
    }
}
