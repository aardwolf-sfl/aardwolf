use lazycell::LazyCell;

use crate::raw::data::Data;
use crate::structures::*;

#[derive(Debug)]
pub enum EmptyDataReason {
    Static,
    Dynamic,
    Tests,
}

#[derive(Debug)]
pub enum InvalidData {
    NoFailingTest,
    Empty(EmptyDataReason),
}

pub struct Api<'a> {
    data: Data,
    stmts: LazyCell<Stmts<'a>>,
    tests: LazyCell<Tests<'a>>,
    def_use: LazyCell<DefUse<'a>>,
    spectra: LazyCell<Spectra<'a>>,
    cfg: LazyCell<Cfg<'a>>,
}

impl<'a> Api<'a> {
    pub(crate) fn new(data: Data) -> Result<Self, InvalidData> {
        if data.static_data.files.is_empty() || data.static_data.functions.is_empty() {
            Err(InvalidData::Empty(EmptyDataReason::Static))
        } else if data.dynamic_data.trace.is_empty() {
            Err(InvalidData::Empty(EmptyDataReason::Dynamic))
        } else if data.test_data.tests.is_empty() {
            Err(InvalidData::Empty(EmptyDataReason::Tests))
        } else if data
            .test_data
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
                cfg: LazyCell::new(),
            })
        }
    }

    pub fn make<T: FromRawData<'a>>(&'a self) -> Result<T, FromRawDataError> {
        T::from_raw(&self.data, &self)
    }

    pub fn get_stmts(&'a self) -> Option<&Stmts<'a>> {
        // TODO: Use macros for all getters' bodies (get_lazy_infallible!).
        if !self.stmts.filled() {
            // Stmts::from_raw does not fail, we can call unwrap.
            self.stmts.fill(self.make::<Stmts>().unwrap()).ok();
        }

        self.stmts.borrow()
    }

    pub fn get_tests(&'a self) -> Option<&Tests<'a>> {
        // TODO: Use macros for all getters' bodies (get_lazy_infallible!).
        if !self.tests.filled() {
            // Tests::from_raw does not fail, we can call unwrap.
            self.tests.fill(self.make::<Tests>().unwrap()).ok();
        }

        self.tests.borrow()
    }

    pub fn get_def_use(&'a self) -> Option<&DefUse<'a>> {
        if !self.def_use.filled() {
            // DefUse::from_raw does not fail, we can call unwrap.
            self.def_use.fill(self.make::<DefUse>().unwrap()).ok();
        }

        self.def_use.borrow()
    }

    pub fn get_spectra(&'a self) -> Option<&'a Spectra<'a>> {
        if !self.spectra.filled() {
            // Spectra::from_raw does not fail, we can call unwrap.
            self.spectra.fill(self.make::<Spectra>().unwrap()).ok();
        }

        self.spectra.borrow()
    }

    pub fn get_cfg(&'a self) -> Option<&'a Cfg<'a>> {
        if !self.cfg.filled() {
            // CFG::from_raw does not fail, we can call unwrap.
            self.cfg.fill(self.make::<Cfg>().unwrap()).ok();
        }

        self.cfg.borrow()
    }
}
