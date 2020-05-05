use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::hash::Hash;
use std::ops::Deref;

use crate::arena::{P, S};
use crate::data::{
    access::{Access, AccessChain},
    types::TestName,
    values::{Value, ValueType},
};

// REFACTOR: Implement multiple detectors with different trade-offs (time and space efficiency).
pub struct Stats {
    samples: HashSet<S<TestName>>,
    data: HashMap<AccessChain, AccessState>,
}

impl Stats {
    pub fn new() -> Self {
        Stats {
            samples: HashSet::new(),
            data: HashMap::new(),
        }
    }

    pub fn learn(&mut self, access: P<Access>, data: P<Value>, test: S<TestName>) {
        self.samples.insert(test);

        let view = AccessChain::from_defs(access.as_ref());
        self.data.entry(view).or_default().learn(data, test);
    }

    pub fn check(&self, data: &P<Value>, access: &P<Access>) -> Vec<InvariantInfo> {
        let view = AccessChain::from_defs(access.as_ref());

        if let Some(state) = self.data.get(&view) {
            state.check(data, access, self)
        } else {
            Vec::new()
        }
    }
}

pub enum Invariant {
    Constant(P<Value>),
    Range(Option<P<Value>>, Option<P<Value>>),
    TypeStable(ValueType),
    // NaN, +Inf, -Inf, NULL
    NonExceptionalValue(ValueType),
}

pub struct InvariantInfo {
    pub inv: Invariant,
    pub access: P<Access>,
    pub confidence: f32,
}

impl InvariantInfo {
    pub fn new(inv: Invariant, access: P<Access>, confidence: f32) -> Self {
        InvariantInfo {
            inv,
            access,
            confidence,
        }
    }

    pub fn explain(&self, data: &P<Value>) -> String {
        match &self.inv {
            Invariant::Constant(cst) => format!(
                "expected to be constantly {}, but is {}",
                cst.as_ref(),
                data.as_ref()
            ),
            Invariant::Range(Some(min), Some(max)) => format!(
                "expected to be in range [{}, {}], but is {}",
                min.as_ref(),
                max.as_ref(),
                data.as_ref()
            ),
            Invariant::Range(Some(min), None) => {
                format!("expected to be ≥{}, but is {}", min.as_ref(), data.as_ref())
            }
            Invariant::Range(None, Some(max)) => {
                format!("expected to be ≤{}, but is {}", max.as_ref(), data.as_ref())
            }
            Invariant::Range(None, None) => panic!("internal error"),
            Invariant::TypeStable(typ) => format!(
                "expected to have a stable type {}, but is of type {}",
                typ,
                data.as_ref().get_type()
            ),
            // TODO: Make better description based on the actual type.
            Invariant::NonExceptionalValue(_) => {
                format!("expected to have a normal value, but is {}", data.as_ref())
            }
        }
    }
}

#[derive(PartialEq, Eq)]
struct ValueOrd(P<Value>);

impl ValueOrd {
    pub fn wrap(value: P<Value>) -> Option<Self> {
        match value.as_ref() {
            Value::Floating(value) if !(***value).is_finite() => None,
            _ => Some(ValueOrd(value)),
        }
    }
}

impl Deref for ValueOrd {
    type Target = P<Value>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PartialOrd for ValueOrd {
    fn partial_cmp(&self, other: &ValueOrd) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ValueOrd {
    fn cmp(&self, other: &ValueOrd) -> Ordering {
        // We checked for problematic cases in `wrap` constructor, we are safe
        // to unwrap here.
        self.0.as_ref().partial_cmp(other.0.as_ref()).unwrap()
    }
}

enum AccessState {
    Empty,
    SingleValue(SingleValueAccessState),
    Range(RangeAccessState),
    None(NoneAccessState),
}

impl Default for AccessState {
    fn default() -> Self {
        AccessState::Empty
    }
}

impl AccessState {
    pub fn new(data: P<Value>, test: S<TestName>) -> Self {
        let data_ref = data.as_ref();

        if data_ref.is_unsupported() {
            AccessState::None(NoneAccessState::typed(
                ValueType::Unsupported,
                vec![test].into_iter().collect(),
            ))
        } else if data_ref.is_exceptional_value() {
            AccessState::None(NoneAccessState::typed_with_reason(
                data_ref.get_type(),
                NoInvariantReason::ExceptionalValue,
                vec![test].into_iter().collect(),
            ))
        } else {
            AccessState::SingleValue(SingleValueAccessState::new(data, test))
        }
    }

    pub fn learn(&mut self, data: P<Value>, test: S<TestName>) {
        let data_ref = data.as_ref();

        // TODO: When creating "none" state, put there all happened violations
        // (ie., both type changed and exceptional value if they happened), not just one.
        match self {
            AccessState::Empty => {
                *self = AccessState::new(data, test);
            }
            AccessState::SingleValue(single_value) => {
                if single_value.data == data {
                    single_value.learn(test);
                } else if single_value.data.as_ref().get_type() != data_ref.get_type() {
                    *self = AccessState::None(NoneAccessState::type_changed(
                        single_value.in_tests.clone(),
                    ))
                } else if data_ref.is_exceptional_value() {
                    *self = AccessState::None(NoneAccessState::typed_with_reason(
                        data_ref.get_type(),
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
                if range.typ != data_ref.get_type() {
                    *self = AccessState::None(NoneAccessState::type_changed(range.in_tests.clone()))
                } else if data_ref.is_exceptional_value() {
                    *self = AccessState::None(NoneAccessState::typed_with_reason(
                        data_ref.get_type(),
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

    pub fn check(&self, data: &P<Value>, access: &P<Access>, stats: &Stats) -> Vec<InvariantInfo> {
        match self {
            AccessState::Empty => Vec::new(),
            AccessState::SingleValue(single_value) => single_value.check(data, access, stats),
            AccessState::Range(range) => range.check(data, access, stats),
            AccessState::None(reasons) => reasons.check(data, access, stats),
        }
    }
}

struct SingleValueAccessState {
    data: P<Value>,
    count: usize,
    in_tests: HashSet<S<TestName>>,
}

impl SingleValueAccessState {
    pub fn new(data: P<Value>, test: S<TestName>) -> Self {
        SingleValueAccessState {
            data,
            count: 1,
            in_tests: vec![test].into_iter().collect(),
        }
    }

    pub fn learn(&mut self, test: S<TestName>) {
        self.count += 1;
        self.in_tests.insert(test);
    }

    pub fn check(&self, data: &P<Value>, access: &P<Access>, stats: &Stats) -> Vec<InvariantInfo> {
        let data_ref = data.as_ref();
        let self_data_ref = self.data.as_ref();

        let mut violations = Vec::new();
        let confidence = (self.in_tests.len() as f32) / (stats.samples.len() as f32);

        if data_ref.is_exceptional_value() {
            violations.push(InvariantInfo::new(
                Invariant::NonExceptionalValue(self_data_ref.get_type()),
                *access,
                confidence,
            ));
        }

        if data_ref.get_type() != self_data_ref.get_type() {
            violations.push(InvariantInfo::new(
                Invariant::TypeStable(self_data_ref.get_type()),
                *access,
                confidence,
            ));
        } else if data != &self.data {
            // Equal types.
            violations.push(InvariantInfo::new(
                Invariant::Constant(self.data),
                *access,
                confidence,
            ));
        }

        violations
    }
}

struct RangeAccessState {
    data: BTreeMap<ValueOrd, usize>,
    typ: ValueType,
    in_tests: HashSet<S<TestName>>,
}

impl RangeAccessState {
    pub fn new(
        mut values: impl Iterator<Item = (P<Value>, usize)>,
        in_tests: HashSet<S<TestName>>,
    ) -> Option<Self> {
        let mut data = BTreeMap::new();

        if let Some(first) = values.next() {
            let typ = first.0.as_ref().get_type();
            let key = ValueOrd::wrap(first.0)?;

            data.insert(key, first.1);

            while let Some(item) = values.next() {
                if item.0.as_ref().get_type() != typ {
                    return None;
                } else {
                    let key = ValueOrd::wrap(item.0)?;
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

    pub fn learn(&mut self, data: P<Value>, test: S<TestName>) {
        self.in_tests.insert(test);

        if data.as_ref().get_type() == self.typ {
            if let Some(key) = ValueOrd::wrap(data) {
                *self.data.entry(key).or_insert(0) += 1;
            }
        }
    }

    pub fn check(&self, data: &P<Value>, access: &P<Access>, stats: &Stats) -> Vec<InvariantInfo> {
        let data_ref = data.as_ref();

        let mut violations = Vec::new();
        let confidence = (self.in_tests.len() as f32) / (stats.samples.len() as f32);

        if data_ref.is_exceptional_value() {
            violations.push(InvariantInfo::new(
                Invariant::NonExceptionalValue(self.typ),
                *access,
                confidence,
            ));
        }

        if data_ref.get_type() != self.typ {
            violations.push(InvariantInfo::new(
                Invariant::TypeStable(self.typ),
                *access,
                confidence,
            ));
        } else {
            // Equal types.
            let min = **self.data.iter().next().unwrap().0;
            let max = **self.data.iter().rev().next().unwrap().0;

            // TODO: Statistical testing.
            if data_ref < min.as_ref() || data_ref > max.as_ref() {
                violations.push(InvariantInfo::new(
                    Invariant::Range(Some(min), Some(max)),
                    *access,
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

struct NoneAccessState {
    typ: Option<ValueType>,
    reasons: HashSet<NoInvariantReason>,
    in_tests: HashSet<S<TestName>>,
}

impl NoneAccessState {
    pub fn typed(typ: ValueType, in_tests: HashSet<S<TestName>>) -> Self {
        NoneAccessState {
            typ: Some(typ),
            reasons: HashSet::new(),
            in_tests,
        }
    }

    pub fn typed_with_reason(
        typ: ValueType,
        reason: NoInvariantReason,
        in_tests: HashSet<S<TestName>>,
    ) -> Self {
        NoneAccessState {
            typ: Some(typ),
            reasons: vec![reason].into_iter().collect(),
            in_tests,
        }
    }

    pub fn type_changed(in_tests: HashSet<S<TestName>>) -> Self {
        NoneAccessState {
            typ: None,
            reasons: HashSet::new(),
            in_tests,
        }
    }

    pub fn learn(&mut self, data: P<Value>, test: S<TestName>) {
        let data_ref = data.as_ref();

        self.in_tests.insert(test);

        if let Some(typ) = self.typ {
            if typ != data_ref.get_type() {
                self.typ = None;
            }
        }

        if data_ref.is_exceptional_value() {
            self.reasons.insert(NoInvariantReason::ExceptionalValue);
        }
    }

    pub fn check(&self, data: &P<Value>, access: &P<Access>, stats: &Stats) -> Vec<InvariantInfo> {
        let data_ref = data.as_ref();

        let mut violations = Vec::new();
        let confidence = (self.in_tests.len() as f32) / (stats.samples.len() as f32);

        if let Some(typ) = self.typ {
            if data_ref.get_type() != typ {
                violations.push(InvariantInfo::new(
                    Invariant::TypeStable(typ),
                    *access,
                    confidence,
                ));
            }

            if data_ref.is_exceptional_value()
                && !self.reasons.contains(&NoInvariantReason::ExceptionalValue)
            {
                violations.push(InvariantInfo::new(
                    Invariant::NonExceptionalValue(typ),
                    *access,
                    confidence,
                ));
            }
        }

        violations
    }
}
