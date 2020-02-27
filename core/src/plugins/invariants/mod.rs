use std::cmp;
use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};

use yaml_rust::Yaml;

use crate::api::Api;
use crate::plugins::{AardwolfPlugin, LocalizationItem, PluginInitError, Rationale};
use crate::raw::data::{Access, TestStatus, VariableData, VariableDataType};

// TODO: This should perhaps globally available macro.
macro_rules! required {
    ($structure:expr) => {
        match $structure {
            Some(structure) => structure,
            None => return Vec::new(),
        }
    };
}

pub struct Invariants;

impl AardwolfPlugin for Invariants {
    fn init<'a>(_api: &'a Api<'a>, opts: &HashMap<String, Yaml>) -> Result<Self, PluginInitError>
    where
        Self: Sized,
    {
        Ok(Invariants)
    }

    fn run_loc<'a, 'b>(&'b self, api: &'a Api<'a>) -> Vec<LocalizationItem<'a, 'b>> {
        let stmts = api.get_stmts();
        let tests = api.get_tests();
        let vars = required!(api.get_vars());

        let mut holder = VarHolder::new();

        for test in tests.iter_names().filter(|name| tests.is_passed(name)) {
            for item in vars.iter_vars(test).unwrap() {
                for (access, data) in item.zip() {
                    holder.add(access, data);
                }
            }
        }

        let invariants = holder.detect_invariants();

        let failing = tests
            .iter_statuses()
            .find(|(name, status)| **status == TestStatus::Failed)
            .map(|(name, status)| name)
            .unwrap();

        let mut results = Vec::new();

        for item in vars.iter_vars(failing).unwrap() {
            for (access, data) in item.zip() {
                let access_wrapper = AccessWrapper::new(access);
                if let Some(access_invariants) = invariants.get(&access_wrapper) {
                    let failed = self
                        .check_invariants(data, access_invariants.iter())
                        .collect::<Vec<_>>();

                    if !failed.is_empty() {
                        let confidence = failed
                            .iter()
                            .map(|details| details.confidence)
                            // Confidence must be a finite number.
                            .max_by(|lhs, rhs| lhs.partial_cmp(rhs).unwrap())
                            // We checked that the vector is not empty.
                            .unwrap();

                        let mut explanation = failed
                            .iter()
                            .map(|failed| self.explain_failed_invariant(data, failed))
                            .collect::<Vec<_>>()
                            .join(", ");
                        explanation.push('.');

                        let mut rationale = Rationale::new();

                        // NOTE: Could be configurable to disable args, calls, etc.
                        if item.stmt.is_arg() {
                            rationale.add_text("The value of this argument violates some invariants inferred from passing runs.");
                        } else if item.stmt.is_ret() {
                            rationale.add_text("The return value violates some invariants inferred from passing runs.");
                        } else if item.stmt.is_call() {
                            rationale.add_text("The result of this function call violates some invariants inferred from passing runs.");
                        } else {
                            rationale.add_text("The result of this statement violates some invariants inferred from passing runs.");
                        }

                        rationale
                            .add_text(" The violations are: ")
                            .add_text(explanation);

                        results.push(
                            LocalizationItem::new(item.stmt.loc, item.stmt, confidence, rationale)
                                .unwrap(),
                        );
                    }
                }
            }
        }

        results
    }
}

impl Invariants {
    fn check_invariants<'a>(
        &'a self,
        data: &'a VariableData,
        invariants: impl Iterator<Item = &'a InvariantDetails<'a>>,
    ) -> impl Iterator<Item = &'a InvariantDetails<'a>> {
        invariants.filter(move |details| match &details.inv {
            Invariant::Constant(cst) => *cst != data,
            Invariant::RangeBoth(min, max) => min.is_less(data) || max.is_greater(data),
            Invariant::NonZero => data.is_zero(),
            Invariant::TypeStability(typ) => *typ != data.get_type(),
            _ => false,
        })
    }

    fn explain_failed_invariant<'a>(
        &'a self,
        data: &'a VariableData,
        details: &'a InvariantDetails<'a>,
    ) -> String {
        match &details.inv {
            Invariant::Constant(cst) => {
                format!("expected to be constantly {}, but is {}", cst, data)
            }
            Invariant::RangeBoth(min, max) => format!(
                "expected to be in range [{}, {}], but is {}",
                min, max, data
            ),
            Invariant::NonZero => format!("expected not to be zero, but it is"),
            Invariant::TypeStability(typ) => format!(
                "expected to have a stable type {}, but is of type {}",
                typ,
                data.get_type()
            ),
            _ => String::new(),
        }
    }
}

#[derive(Debug)]
struct AccessWrapper<'a>(&'a Access);

impl<'a> AccessWrapper<'a> {
    pub fn new(access: &'a Access) -> Self {
        AccessWrapper(access)
    }

    fn wrapper_hash<H: Hasher>(&self, state: &mut H, access: &Access) {
        match access {
            Access::Scalar(scalar) => scalar.hash(state),
            Access::Structural(obj, field) => {
                self.wrapper_hash(state, obj);
                self.wrapper_hash(state, field);
            }
            // Ignore index variables.
            Access::ArrayLike(array, _) => self.wrapper_hash(state, array),
        }
    }

    fn wrapper_eq(&self, lhs: &Access, rhs: &Access) -> bool {
        match (lhs, rhs) {
            (Access::Scalar(scalar_lhs), Access::Scalar(scalar_rhs)) => scalar_lhs == scalar_rhs,
            (Access::Structural(obj_lhs, field_lhs), Access::Structural(obj_rhs, field_rhs)) => {
                self.wrapper_eq(obj_lhs, obj_rhs) && self.wrapper_eq(field_lhs, field_rhs)
            }
            // Ignore index variables.
            (Access::ArrayLike(array_lhs, _), Access::ArrayLike(array_rhs, _)) => {
                self.wrapper_eq(array_lhs, array_rhs)
            }
            _ => false,
        }
    }
}

impl<'a> Hash for AccessWrapper<'a> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.wrapper_hash(state, self.0);
    }
}

impl<'a> PartialEq for AccessWrapper<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.wrapper_eq(self.0, other.0)
    }
}

impl<'a> Eq for AccessWrapper<'a> {}

impl<'a> AsRef<Access> for AccessWrapper<'a> {
    fn as_ref(&self) -> &Access {
        &self.0
    }
}

#[derive(Debug, PartialEq)]
enum RangeBoundary {
    Signed(i64),
    Unsigned(u64),
    Floating(f64),
}

impl RangeBoundary {
    pub fn signed_max() -> Self {
        RangeBoundary::Signed(std::i64::MAX)
    }

    pub fn signed_min() -> Self {
        RangeBoundary::Signed(std::i64::MIN)
    }

    pub fn unsigned_max() -> Self {
        RangeBoundary::Unsigned(std::u64::MAX)
    }

    pub fn unsigned_min() -> Self {
        RangeBoundary::Unsigned(std::u64::MIN)
    }

    pub fn floating_max() -> Self {
        RangeBoundary::Floating(std::f64::MAX)
    }

    pub fn floating_min() -> Self {
        RangeBoundary::Floating(std::f64::MIN)
    }

    pub fn max(typ: &VariableDataType) -> Option<Self> {
        if typ.is_signed() {
            Some(RangeBoundary::signed_max())
        } else if typ.is_unsigned() {
            Some(RangeBoundary::unsigned_max())
        } else if typ.is_floating() {
            Some(RangeBoundary::floating_max())
        } else {
            None
        }
    }

    pub fn min(typ: &VariableDataType) -> Option<Self> {
        if typ.is_signed() {
            Some(RangeBoundary::signed_min())
        } else if typ.is_unsigned() {
            Some(RangeBoundary::unsigned_min())
        } else if typ.is_floating() {
            Some(RangeBoundary::floating_min())
        } else {
            None
        }
    }

    pub fn update_min(self, value: &VariableData) -> Self {
        match self {
            RangeBoundary::Signed(min) => {
                if let Some(x) = value.as_signed() {
                    RangeBoundary::Signed(cmp::min(min, x))
                } else {
                    self
                }
            }
            RangeBoundary::Unsigned(min) => {
                if let Some(x) = value.as_unsigned() {
                    RangeBoundary::Unsigned(cmp::min(min, x))
                } else {
                    self
                }
            }
            RangeBoundary::Floating(min) => {
                if let Some(x) = value.as_floating() {
                    if x.is_finite() {
                        RangeBoundary::Floating(if x < min { x } else { min })
                    } else {
                        self
                    }
                } else {
                    self
                }
            }
        }
    }

    pub fn update_max(self, value: &VariableData) -> Self {
        match self {
            RangeBoundary::Signed(max) => {
                if let Some(x) = value.as_signed() {
                    RangeBoundary::Signed(cmp::max(max, x))
                } else {
                    self
                }
            }
            RangeBoundary::Unsigned(max) => {
                if let Some(x) = value.as_unsigned() {
                    RangeBoundary::Unsigned(cmp::max(max, x))
                } else {
                    self
                }
            }
            RangeBoundary::Floating(max) => {
                if let Some(x) = value.as_floating() {
                    if x.is_finite() {
                        RangeBoundary::Floating(if x > max { x } else { max })
                    } else {
                        self
                    }
                } else {
                    self
                }
            }
        }
    }

    pub fn is_greater(&self, value: &VariableData) -> bool {
        match self {
            RangeBoundary::Signed(a) => {
                if let Some(x) = value.as_signed() {
                    x > *a
                } else {
                    false
                }
            }
            RangeBoundary::Unsigned(a) => {
                if let Some(x) = value.as_unsigned() {
                    x > *a
                } else {
                    false
                }
            }
            RangeBoundary::Floating(a) => {
                if let Some(x) = value.as_floating() {
                    x > *a
                } else {
                    false
                }
            }
        }
    }

    pub fn is_less(&self, value: &VariableData) -> bool {
        match self {
            RangeBoundary::Signed(a) => {
                if let Some(x) = value.as_signed() {
                    x < *a
                } else {
                    false
                }
            }
            RangeBoundary::Unsigned(a) => {
                if let Some(x) = value.as_unsigned() {
                    x < *a
                } else {
                    false
                }
            }
            RangeBoundary::Floating(a) => {
                if let Some(x) = value.as_floating() {
                    x < *a
                } else {
                    false
                }
            }
        }
    }
}

impl fmt::Display for RangeBoundary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RangeBoundary::Signed(value) => write!(f, "{}", value),
            RangeBoundary::Unsigned(value) => write!(f, "{}", value),
            RangeBoundary::Floating(value) => write!(f, "{}", value),
        }
    }
}

#[derive(Debug)]
enum Relation {
    Equal,
    LessOrEqual,
    GreaterOrEqual,
}

// NOTE: Add something like confidence level for estimating suspiciousness?
#[derive(Debug)]
enum Invariant<'a> {
    Constant(&'a VariableData),
    RangeLeq(RangeBoundary),
    RangeGeq(RangeBoundary),
    RangeBoth(RangeBoundary, RangeBoundary),
    NonZero,
    Ordering(&'a Access, Relation),
    TypeStability(VariableDataType),
}

struct InvariantDetails<'a> {
    pub inv: Invariant<'a>,
    pub access: &'a Access,
    pub confidence: f32,
}

impl<'a> InvariantDetails<'a> {
    pub fn new(inv: Invariant<'a>, access: &'a Access, confidence: f32) -> Self {
        InvariantDetails {
            inv,
            access,
            confidence,
        }
    }
}

#[derive(Debug)]
struct VarHolder<'a>(HashMap<AccessWrapper<'a>, HashMap<&'a VariableData, usize>>);

impl<'a> VarHolder<'a> {
    pub fn new() -> Self {
        VarHolder(HashMap::new())
    }

    pub fn add(&mut self, access: &'a Access, data: &'a VariableData) {
        let mut wrapper = AccessWrapper::new(access);
        *self
            .0
            .entry(wrapper)
            .or_insert(HashMap::new())
            .entry(data)
            .or_insert(0) += 1;
    }

    pub fn detect_invariants<'b>(
        &'b self,
    ) -> HashMap<&'b AccessWrapper<'b>, Vec<InvariantDetails<'b>>> {
        let mut invariants = HashMap::new();

        for (access, data) in self.0.iter() {
            if let Some(invariant) = self.detect_constant(access, data) {
                invariants
                    .entry(access)
                    .or_insert(Vec::new())
                    .push(invariant);
            }

            if let Some(invariant) = self.detect_range(access, data) {
                invariants
                    .entry(access)
                    .or_insert(Vec::new())
                    .push(invariant);
            }

            if let Some(invariant) = self.detect_non_zero(access, data) {
                invariants
                    .entry(access)
                    .or_insert(Vec::new())
                    .push(invariant);
            }

            if let Some(invariant) = self.detect_type_stability(access, data) {
                invariants
                    .entry(access)
                    .or_insert(Vec::new())
                    .push(invariant);
            }
        }

        invariants
    }

    fn detect_constant(
        &self,
        access: &'a AccessWrapper<'a>,
        data: &HashMap<&'a VariableData, usize>,
    ) -> Option<InvariantDetails> {
        if data.len() == 1 && *data.keys().next().unwrap() != &VariableData::Unsupported {
            Some(InvariantDetails::new(
                Invariant::Constant(data.keys().next().unwrap()),
                access.as_ref(),
                1.0,
            ))
        } else {
            None
        }
    }

    fn detect_range(
        &self,
        access: &'a AccessWrapper<'a>,
        data: &HashMap<&'a VariableData, usize>,
    ) -> Option<InvariantDetails> {
        if data.len() > 1 {
            if let Some(typ) = self.is_of_same_type(data) {
                let mut min = RangeBoundary::max(&typ)?;
                let mut max = RangeBoundary::min(&typ)?;

                for value in data.keys() {
                    min = min.update_min(*value);
                    max = max.update_max(*value);
                }

                // TODO: Statistical testing of significance.
                Some(InvariantDetails::new(
                    Invariant::RangeBoth(min, max),
                    access.as_ref(),
                    1.0,
                ))
            } else {
                None
            }
        } else {
            None
        }
    }

    fn detect_non_zero(
        &self,
        access: &'a AccessWrapper<'a>,
        data: &HashMap<&'a VariableData, usize>,
    ) -> Option<InvariantDetails> {
        if data.len() > 1 || (data.len() == 1 && *data.values().next().unwrap() > 1) {
            let mut found_numeric = false;

            for value in data.keys() {
                if value.get_type().is_numeric() {
                    found_numeric = true;
                    if value.is_zero() {
                        return None;
                    }
                }
            }

            if found_numeric {
                Some(InvariantDetails::new(
                    Invariant::NonZero,
                    access.as_ref(),
                    1.0,
                ))
            } else {
                None
            }
        } else {
            None
        }
    }

    fn detect_type_stability(
        &self,
        access: &'a AccessWrapper<'a>,
        data: &HashMap<&'a VariableData, usize>,
    ) -> Option<InvariantDetails> {
        if data.len() > 1 {
            if let Some(typ) = self.is_of_same_type(data) {
                Some(InvariantDetails::new(
                    Invariant::TypeStability(typ),
                    access.as_ref(),
                    1.0,
                ))
            } else {
                None
            }
        } else {
            None
        }
    }

    fn is_of_same_type(&self, data: &HashMap<&'a VariableData, usize>) -> Option<VariableDataType> {
        let mut data_iter = data.keys();
        let first = data_iter.next()?;
        let typ = first.get_type();

        for item in data_iter {
            if item.get_type() != typ {
                return None;
            }
        }

        Some(typ)
    }
}
