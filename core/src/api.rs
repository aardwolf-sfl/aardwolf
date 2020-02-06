use lazycell::LazyCell;

use crate::raw::data::Data;
use crate::structures::*;

pub struct Api<'a> {
    data: Data,
    stmts: LazyCell<Stmts<'a>>,
    tests: LazyCell<Tests<'a>>,
    def_use: LazyCell<DefUse<'a>>,
    spectra: LazyCell<Spectra<'a>>,
}

impl<'a> Api<'a> {
    pub(crate) fn new(data: Data) -> Self {
        Api {
            data,
            stmts: LazyCell::new(),
            tests: LazyCell::new(),
            def_use: LazyCell::new(),
            spectra: LazyCell::new(),
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
}
