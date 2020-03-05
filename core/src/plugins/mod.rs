use std::collections::{HashMap, HashSet};
use std::fmt;

use yaml_rust::Yaml;

use crate::api::Api;
use crate::raw::data::{Loc, Statement, TestName};

pub mod collect_bb;
pub mod invariants;
pub mod prob_graph;
pub mod sbfl;

pub struct IrrelevantItems<'data> {
    // Store relevant items and remove them if they are marked as irrelevant.
    pub stmts: HashSet<u64>,
    pub tests: HashSet<&'data TestName>,
}

impl<'data> IrrelevantItems<'data> {
    pub fn new(api: &'data Api<'data>) -> Self {
        // By default, all items are relevant.
        IrrelevantItems {
            stmts: api.get_stmts().iter_ids().copied().collect(),
            tests: api.get_tests().iter_names().collect(),
        }
    }

    pub fn mark_stmt(&mut self, stmt: &Statement) {
        self.stmts.remove(&stmt.id);
    }

    pub fn mark_test(&mut self, test: &TestName) {
        self.tests.remove(test);
    }

    pub fn is_stmt_relevant(&self, stmt: &Statement) -> bool {
        self.stmts.contains(&stmt.id)
    }

    pub fn is_test_relevant(&self, test: &TestName) -> bool {
        self.tests.contains(test)
    }
}

#[derive(Clone)]
pub struct Results<'data> {
    items: Vec<LocalizationItem<'data>>,
    n_results: usize,
    max_score: f32,
}

impl<'data> Results<'data> {
    pub fn new(n_results: usize) -> Self {
        Results {
            items: Vec::with_capacity(n_results),
            n_results,
            max_score: 0.0,
        }
    }

    pub fn add(&mut self, item: LocalizationItem<'data>) {
        if item.score > self.max_score {
            self.max_score = item.score;
        }

        // Use stable algorithm when sorting the items to not break plugins
        // which sort the results using another criterion.
        // Also, we can safely unwrap the result of partial_cmp,
        // because score is checked for finiteness in LocalizationItem constructor.

        if self.items.len() < self.n_results {
            self.items.push(item);
            self.items
                .sort_by(|lhs, rhs| rhs.score.partial_cmp(&lhs.score).unwrap());
        } else {
            match self.items.last() {
                None => self.items.push(item),
                Some(worst) => {
                    if item.score > worst.score {
                        self.items.pop();
                        self.items.push(item);
                        self.items
                            .sort_by(|lhs, rhs| rhs.score.partial_cmp(&lhs.score).unwrap());
                    }
                }
            }
        }
    }

    pub fn any(&self) -> bool {
        !self.items.is_empty()
    }

    pub fn normalize(self) -> Self {
        let max_score = self.max_score;
        let items = self
            .items
            .into_iter()
            .map(|item| item.normalize(max_score))
            .collect();

        Results { items, ..self }
    }

    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &LocalizationItem<'data>> {
        self.items.iter()
    }
}

impl<'data> IntoIterator for Results<'data> {
    type Item = LocalizationItem<'data>;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.into_iter()
    }
}

#[derive(Debug)]
pub enum MissingApi {
    Cfg,
    DefUse,
    Spectra,
    Stmts,
    Tests,
    Vars,
}

#[derive(Debug)]
pub enum PluginError {
    Inner(String),
    MissingApi(MissingApi),
}

#[derive(Clone, PartialEq, Eq)]
pub enum RationaleChunk {
    Text(String),
    Anchor(Loc),
}

#[derive(Clone, PartialEq, Eq)]
pub struct Rationale(Vec<RationaleChunk>);

impl Rationale {
    pub fn new() -> Self {
        Rationale(Vec::new())
    }

    pub fn add_text<T: Into<String>>(&mut self, text: T) -> &mut Self {
        self.0.push(RationaleChunk::Text(text.into()));
        self
    }

    pub fn add_anchor(&mut self, anchor: Loc) -> &mut Self {
        self.0.push(RationaleChunk::Anchor(anchor));
        self
    }

    pub fn newline(&mut self) -> &mut Self {
        self.0.push(RationaleChunk::Text(String::from("\n")));
        self
    }

    pub fn join(&self, other: &Self) -> Self {
        let chunks = self.0.iter().chain(other.0.iter()).cloned().collect();
        Rationale(chunks)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn chunks(&self) -> &Vec<RationaleChunk> {
        &self.0
    }
}

impl From<String> for Rationale {
    fn from(value: String) -> Self {
        let mut result = Rationale::new();
        result.add_text(value);
        result
    }
}

impl From<&str> for Rationale {
    fn from(value: &str) -> Self {
        let mut result = Rationale::new();
        result.add_text(String::from(value));
        result
    }
}

impl fmt::Debug for Rationale {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for chunk in self.0.iter() {
            match chunk {
                RationaleChunk::Text(text) => write!(f, "{}", text)?,
                RationaleChunk::Anchor(anchor) => write!(f, "{:?}", anchor)?,
            }
        }

        Ok(())
    }
}

#[derive(Clone, PartialEq)]
pub struct LocalizationItem<'data> {
    pub loc: Loc,
    pub root_stmt: &'data Statement,
    pub score: f32,
    pub rationale: Rationale,
}

#[derive(Debug)]
pub enum InvalidLocalizationItem {
    InvalidScore(f32),
    EmptyRationale,
}

impl<'data> LocalizationItem<'data> {
    pub fn new(
        loc: Loc,
        root_stmt: &'data Statement,
        score: f32,
        rationale: Rationale,
    ) -> Result<Self, InvalidLocalizationItem> {
        // The check whether the score is finite is important for total order of items.
        match (score.is_finite(), rationale.is_empty()) {
            (false, _) => Err(InvalidLocalizationItem::InvalidScore(score)),
            (_, true) => Err(InvalidLocalizationItem::EmptyRationale),
            _ => Ok(LocalizationItem {
                loc,
                root_stmt,
                score,
                rationale,
            }),
        }
    }

    pub fn normalize(self, max_score: f32) -> Self {
        LocalizationItem {
            score: self.score / max_score,
            ..self
        }
    }
}

pub type PluginInitError = String;

pub trait AardwolfPlugin {
    fn init<'data>(
        api: &'data Api<'data>,
        opts: &HashMap<String, Yaml>,
    ) -> Result<Self, PluginInitError>
    where
        Self: Sized;

    // TODO: Make general structure Preprocessing instead of IrrelevantItems.
    fn run_pre<'data, 'out>(
        &'out self,
        _api: &'data Api<'data>,
        _irrelevant: &'out mut IrrelevantItems<'data>,
    ) -> Result<(), PluginError> {
        Ok(())
    }

    fn run_loc<'data, 'param>(
        &self,
        _api: &'data Api<'data>,
        _results: &'param mut Results<'data>,
        _irrelevant: &'param IrrelevantItems<'data>,
    ) -> Result<(), PluginError> {
        Ok(())
    }

    fn run_post<'data, 'param>(
        &self,
        _api: &'data Api<'data>,
        _base: &'param HashMap<&'param str, &'param Results<'data>>,
        _results: &'param mut Results<'data>,
    ) -> Result<(), PluginError> {
        Ok(())
    }
}
