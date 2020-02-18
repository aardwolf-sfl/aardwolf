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

macro_rules! get_lazy_infallible {
    ($api:expr, $prop:ident) => {{
        if !($api).$prop.filled() {
            ($api).$prop.fill($api.make().unwrap()).ok();
        }

        ($api).$prop.borrow().unwrap()
    }};
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

    pub fn get_stmts(&'a self) -> &Stmts<'a> {
        get_lazy_infallible!(self, stmts)
    }

    pub fn get_tests(&'a self) -> &Tests<'a> {
        get_lazy_infallible!(self, tests)
    }

    pub fn get_def_use(&'a self) -> &DefUse<'a> {
        get_lazy_infallible!(self, def_use)
    }

    pub fn get_spectra(&'a self) -> &Spectra<'a> {
        get_lazy_infallible!(self, spectra)
    }

    pub fn get_cfg(&'a self) -> &Cfg<'a> {
        get_lazy_infallible!(self, cfg)
    }
}
