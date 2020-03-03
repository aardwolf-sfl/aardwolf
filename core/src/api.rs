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

pub struct Api<'data> {
    data: Data,
    stmts: LazyCell<Stmts<'data>>,
    tests: LazyCell<Tests<'data>>,
    def_use: LazyCell<DefUse<'data>>,
    spectra: LazyCell<Spectra<'data>>,
    cfgs: LazyCell<Cfgs<'data>>,
    vars: LazyCell<Vars<'data>>,
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

impl<'data> Api<'data> {
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
                cfgs: LazyCell::new(),
                vars: LazyCell::new(),
            })
        }
    }

    pub fn make<T: FromRawData<'data>>(&'data self) -> Result<T, FromRawDataError> {
        T::from_raw(&self.data, &self)
    }

    pub fn get_stmts(&'data self) -> &Stmts<'data> {
        get_lazy_infallible!(self, stmts)
    }

    pub fn get_tests(&'data self) -> &Tests<'data> {
        get_lazy_infallible!(self, tests)
    }

    pub fn get_def_use(&'data self) -> &DefUse<'data> {
        get_lazy_infallible!(self, def_use)
    }

    pub fn get_spectra(&'data self) -> &Spectra<'data> {
        get_lazy_infallible!(self, spectra)
    }

    pub fn get_cfgs(&'data self) -> &Cfgs<'data> {
        get_lazy_infallible!(self, cfgs)
    }

    pub fn get_vars(&'data self) -> Option<&Vars<'data>> {
        get_lazy_fallible!(self, vars)
    }
}
