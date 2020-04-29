use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::ops::Deref;

use crate::data::{
    access::Access,
    types::TestName,
    values::{Value, ValueType},
};

// REFACTOR: Implement multiple detectors with different trade-offs (time and space efficiency).
pub struct Stats<'data> {
    samples: HashSet<&'data TestName>,
    data: HashMap<AccessView<'data>, AccessState<'data>>,
}

impl<'data> Stats<'data> {
    pub fn new() -> Self {
        Stats {
            samples: HashSet::new(),
            data: HashMap::new(),
        }
    }

    pub fn learn(&mut self, access: &'data Access, data: &'data Value, test: &'data TestName) {
        self.samples.insert(test);

        let view = AccessView::new(access);
        self.data.entry(view).or_default().learn(data, test);
    }

    pub fn check(&self, data: &'data Value, access: &'data Access) -> Vec<InvariantInfo<'data>> {
        let view = AccessView::new(access);

        if let Some(state) = self.data.get(&view) {
            state.check(data, access, self)
        } else {
            Vec::new()
        }
    }
}

pub enum Invariant<'data> {
    Constant(&'data Value),
    Range(Option<&'data Value>, Option<&'data Value>),
    TypeStable(ValueType),
    // NaN, +Inf, -Inf, NULL
    NonExceptionalValue(ValueType),
}

pub struct InvariantInfo<'data> {
    pub inv: Invariant<'data>,
    pub access: &'data Access,
    pub confidence: f32,
}

impl<'data> InvariantInfo<'data> {
    pub fn new(inv: Invariant<'data>, access: &'data Access, confidence: f32) -> Self {
        InvariantInfo {
            inv,
            access,
            confidence,
        }
    }

    pub fn explain(&self, data: &'data Value) -> String {
        match &self.inv {
            Invariant::Constant(cst) => {
                format!("expected to be constantly {}, but is {}", cst, data)
            }
            Invariant::Range(Some(min), Some(max)) => format!(
                "expected to be in range [{}, {}], but is {}",
                min, max, data
            ),
            Invariant::Range(Some(min), None) => {
                format!("expected to be ≥{}, but is {}", min, data)
            }
            Invariant::Range(None, Some(max)) => {
                format!("expected to be ≤{}, but is {}", max, data)
            }
            Invariant::Range(None, None) => panic!("internal error"),
            Invariant::TypeStable(typ) => format!(
                "expected to have a stable type {}, but is of type {}",
                typ,
                data.get_type()
            ),
            // TODO: Make better description based on the actual type.
            Invariant::NonExceptionalValue(_) => {
                format!("expected to have a normal value, but is {}", data)
            }
        }
    }
}

#[derive(Debug)]
struct AccessView<'data>(&'data Access);

impl<'data> AccessView<'data> {
    pub fn new(access: &'data Access) -> Self {
        AccessView(access)
    }

    fn view_hash<H: Hasher>(&self, state: &mut H, access: &Access) {
        match access {
            Access::Scalar(scalar) => scalar.hash(state),
            Access::Structural(obj, field) => {
                self.view_hash(state, obj);
                self.view_hash(state, field);
            }
            // Ignore index variables.
            Access::ArrayLike(array, _) => self.view_hash(state, array),
        }
    }

    fn view_eq(&self, lhs: &Access, rhs: &Access) -> bool {
        match (lhs, rhs) {
            (Access::Scalar(scalar_lhs), Access::Scalar(scalar_rhs)) => scalar_lhs == scalar_rhs,
            (Access::Structural(obj_lhs, field_lhs), Access::Structural(obj_rhs, field_rhs)) => {
                self.view_eq(obj_lhs, obj_rhs) && self.view_eq(field_lhs, field_rhs)
            }
            // Ignore index variables.
            (Access::ArrayLike(array_lhs, _), Access::ArrayLike(array_rhs, _)) => {
                self.view_eq(array_lhs, array_rhs)
            }
            _ => false,
        }
    }
}

impl<'data> Hash for AccessView<'data> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.view_hash(state, self.0);
    }
}

impl<'data> PartialEq for AccessView<'data> {
    fn eq(&self, other: &Self) -> bool {
        self.view_eq(self.0, other.0)
    }
}

impl<'data> Eq for AccessView<'data> {}

impl<'data> AsRef<Access> for AccessView<'data> {
    fn as_ref(&self) -> &Access {
        &self.0
    }
}

#[derive(PartialOrd, PartialEq, Eq)]
struct UnsafeOrd<T: PartialOrd + PartialEq + Eq>(T);

impl<T: PartialOrd + PartialEq + Eq> Ord for UnsafeOrd<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.partial_cmp(&other.0).unwrap()
    }
}

impl<'data> UnsafeOrd<&'data Value> {
    pub fn wrap(value: &'data Value) -> Option<Self> {
        match value {
            Value::Floating(value) if !(***value).is_finite() => None,
            other => Some(UnsafeOrd(other)),
        }
    }
}

impl<T: PartialOrd + PartialEq + Eq> Deref for UnsafeOrd<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

enum AccessState<'data> {
    Empty,
    SingleValue(SingleValueAccessState<'data>),
    Range(RangeAccessState<'data>),
    None(NoneAccessState<'data>),
}

impl<'data> Default for AccessState<'data> {
    fn default() -> Self {
        AccessState::Empty
    }
}

impl<'data> AccessState<'data> {
    pub fn new(data: &'data Value, test: &'data TestName) -> Self {
        if data.is_unsupported() {
            AccessState::None(NoneAccessState::typed(
                ValueType::Unsupported,
                vec![test].into_iter().collect(),
            ))
        } else if data.is_exceptional_value() {
            AccessState::None(NoneAccessState::typed_with_reason(
                data.get_type(),
                NoInvariantReason::ExceptionalValue,
                vec![test].into_iter().collect(),
            ))
        } else {
            AccessState::SingleValue(SingleValueAccessState::new(data, test))
        }
    }

    pub fn learn(&mut self, data: &'data Value, test: &'data TestName) {
        // TODO: When creating "none" state, put there all happened violations
        // (ie., both type changed and exceptional value if they happened), not just one.
        match self {
            AccessState::Empty => {
                *self = AccessState::new(data, test);
            }
            AccessState::SingleValue(single_value) => {
                if single_value.data == data {
                    single_value.learn(test);
                } else if single_value.data.get_type() != data.get_type() {
                    *self = AccessState::None(NoneAccessState::type_changed(
                        single_value.in_tests.clone(),
                    ))
                } else if data.is_exceptional_value() {
                    *self = AccessState::None(NoneAccessState::typed_with_reason(
                        data.get_type(),
                        NoInvariantReason::ExceptionalValue,
                        single_value.in_tests.clone(),
                    ))
                } else {
                    *self = AccessState::Range(
                        RangeAccessState::new(
                            vec![(single_value.data, single_value.count), (data, 1)].into_iter(),
                            single_value.in_tests.clone(),
                        )
                        .unwrap(),
                    );
                }
            }
            AccessState::Range(range) => {
                if range.typ != data.get_type() {
                    *self = AccessState::None(NoneAccessState::type_changed(range.in_tests.clone()))
                } else if data.is_exceptional_value() {
                    *self = AccessState::None(NoneAccessState::typed_with_reason(
                        data.get_type(),
                        NoInvariantReason::ExceptionalValue,
                        range.in_tests.clone(),
                    ))
                } else {
                    range.learn(data, test);
                }
            }
            AccessState::None(reasons) => {
                reasons.learn(data, test);
            }
        }
    }

    pub fn check(
        &self,
        data: &'data Value,
        access: &'data Access,
        stats: &Stats<'data>,
    ) -> Vec<InvariantInfo<'data>> {
        match self {
            AccessState::Empty => Vec::new(),
            AccessState::SingleValue(single_value) => single_value.check(data, access, stats),
            AccessState::Range(range) => range.check(data, access, stats),
            AccessState::None(reasons) => reasons.check(data, access, stats),
        }
    }
}

struct SingleValueAccessState<'data> {
    data: &'data Value,
    count: usize,
    in_tests: HashSet<&'data TestName>,
}

impl<'data> SingleValueAccessState<'data> {
    pub fn new(data: &'data Value, test: &'data TestName) -> Self {
        SingleValueAccessState {
            data,
            count: 1,
            in_tests: vec![test].into_iter().collect(),
        }
    }

    pub fn learn(&mut self, test: &'data TestName) {
        self.count += 1;
        self.in_tests.insert(test);
    }

    pub fn check(
        &self,
        data: &'data Value,
        access: &'data Access,
        stats: &Stats<'data>,
    ) -> Vec<InvariantInfo<'data>> {
        let mut violations = Vec::new();
        let confidence = (self.in_tests.len() as f32) / (stats.samples.len() as f32);

        if data.is_exceptional_value() {
            violations.push(InvariantInfo::new(
                Invariant::NonExceptionalValue(self.data.get_type()),
                access,
                confidence,
            ));
        }

        if data.get_type() != self.data.get_type() {
            violations.push(InvariantInfo::new(
                Invariant::TypeStable(self.data.get_type()),
                access,
                confidence,
            ));
        } else if data != self.data {
            // Equal types.
            violations.push(InvariantInfo::new(
                Invariant::Constant(self.data),
                access,
                confidence,
            ));
        }

        violations
    }
}

struct RangeAccessState<'data> {
    data: BTreeMap<UnsafeOrd<&'data Value>, usize>,
    typ: ValueType,
    in_tests: HashSet<&'data TestName>,
}

impl<'data> RangeAccessState<'data> {
    pub fn new(
        mut values: impl Iterator<Item = (&'data Value, usize)>,
        in_tests: HashSet<&'data TestName>,
    ) -> Option<Self> {
        let mut data = BTreeMap::new();

        if let Some(first) = values.next() {
            let typ = first.0.get_type();
            let key = UnsafeOrd::wrap(first.0)?;

            data.insert(key, first.1);

            while let Some(item) = values.next() {
                if item.0.get_type() != typ {
                    return None;
                } else {
                    let key = UnsafeOrd::wrap(item.0)?;
                    *data.entry(key).or_insert(0) += item.1;
                }
            }

            Some(RangeAccessState {
                data,
                typ,
                in_tests,
            })
        } else {
            None
        }
    }

    pub fn learn(&mut self, data: &'data Value, test: &'data TestName) {
        self.in_tests.insert(test);

        if data.get_type() == self.typ {
            if let Some(key) = UnsafeOrd::wrap(data) {
                *self.data.entry(key).or_insert(0) += 1;
            }
        }
    }

    pub fn check(
        &self,
        data: &'data Value,
        access: &'data Access,
        stats: &Stats<'data>,
    ) -> Vec<InvariantInfo<'data>> {
        let mut violations = Vec::new();
        let confidence = (self.in_tests.len() as f32) / (stats.samples.len() as f32);

        if data.is_exceptional_value() {
            violations.push(InvariantInfo::new(
                Invariant::NonExceptionalValue(self.typ),
                access,
                confidence,
            ));
        }

        if data.get_type() != self.typ {
            violations.push(InvariantInfo::new(
                Invariant::TypeStable(self.typ),
                access,
                confidence,
            ));
        } else {
            // Equal types.
            let min = **self.data.iter().next().unwrap().0;
            let max = **self.data.iter().rev().next().unwrap().0;

            // TODO: Statistical testing.
            if data < min || data > max {
                violations.push(InvariantInfo::new(
                    Invariant::Range(Some(min), Some(max)),
                    access,
                    confidence,
                ));
            }
        }

        violations
    }
}

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
enum NoInvariantReason {
    ExceptionalValue,
}

struct NoneAccessState<'data> {
    typ: Option<ValueType>,
    reasons: HashSet<NoInvariantReason>,
    in_tests: HashSet<&'data TestName>,
}

impl<'data> NoneAccessState<'data> {
    pub fn typed(typ: ValueType, in_tests: HashSet<&'data TestName>) -> Self {
        NoneAccessState {
            typ: Some(typ),
            reasons: HashSet::new(),
            in_tests,
        }
    }

    pub fn typed_with_reason(
        typ: ValueType,
        reason: NoInvariantReason,
        in_tests: HashSet<&'data TestName>,
    ) -> Self {
        NoneAccessState {
            typ: Some(typ),
            reasons: vec![reason].into_iter().collect(),
            in_tests,
        }
    }

    pub fn type_changed(in_tests: HashSet<&'data TestName>) -> Self {
        NoneAccessState {
            typ: None,
            reasons: HashSet::new(),
            in_tests,
        }
    }

    pub fn learn(&mut self, data: &'data Value, test: &'data TestName) {
        self.in_tests.insert(test);

        if let Some(typ) = self.typ {
            if typ != data.get_type() {
                self.typ = None;
            }
        }

        if data.is_exceptional_value() {
            self.reasons.insert(NoInvariantReason::ExceptionalValue);
        }
    }

    pub fn check(
        &self,
        data: &'data Value,
        access: &'data Access,
        stats: &Stats<'data>,
    ) -> Vec<InvariantInfo<'data>> {
        let mut violations = Vec::new();
        let confidence = (self.in_tests.len() as f32) / (stats.samples.len() as f32);

        if let Some(typ) = self.typ {
            if data.get_type() != typ {
                violations.push(InvariantInfo::new(
                    Invariant::TypeStable(typ),
                    access,
                    confidence,
                ));
            }

            if data.is_exceptional_value()
                && !self.reasons.contains(&NoInvariantReason::ExceptionalValue)
            {
                violations.push(InvariantInfo::new(
                    Invariant::NonExceptionalValue(typ),
                    access,
                    confidence,
                ));
            }
        }

        violations
    }
}
