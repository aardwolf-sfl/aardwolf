use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::fmt;

use yaml_rust::Yaml;

use crate::api::Api;
use crate::raw::data::{Loc, Statement, TestName};

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

pub struct Results<'data, 'out> {
    items: Vec<LocalizationItem<'data, 'out>>,
    n_results: usize,
}

impl<'data, 'out> Results<'data, 'out> {
    pub fn new(n_results: usize) -> Self {
        Results {
            items: Vec::with_capacity(n_results),
            n_results,
        }
    }

    pub fn add(&mut self, item: LocalizationItem<'data, 'out>) {
        // TODO: Manage a sorted vector of size n_results with best results encountered so far.
        self.items.push(item);
    }

    pub fn into_vec(mut self) -> Vec<LocalizationItem<'data, 'out>> {
        // Use stable sort to not break plugins which sort the results using another criterion.
        self.items.sort_by(|lhs, rhs| rhs.cmp(lhs));

        self.items
            .into_iter()
            // TODO: Will not be necessary when the optimization mentioned in `add` method is implemented.
            .take(self.n_results)
            .collect()
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

#[derive(Clone)]
enum RationaleChunk {
    Text(String),
    Anchor(Loc),
}

#[derive(Clone)]
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

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
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

pub struct LocalizationItem<'data, 'out> {
    pub loc: Loc,
    pub root_stmt: &'data Statement,
    pub score: f32,
    pub rationale: Rationale,
    pub links: Vec<&'out LocalizationItem<'data, 'out>>,
}

#[derive(Debug)]
pub enum InvalidLocalizationItem {
    InvalidScore(f32),
    EmptyRationale,
}

impl<'data, 'out> LocalizationItem<'data, 'out> {
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
                links: Vec::new(),
            }),
        }
    }

    pub fn link(&'out mut self, other: &'data LocalizationItem<'data, 'out>) {
        self.links.push(other);
    }
}

impl<'data, 'out> PartialEq for LocalizationItem<'data, 'out> {
    fn eq(&self, other: &Self) -> bool {
        self.score == other.score
    }
}

impl<'data, 'out> Eq for LocalizationItem<'data, 'out> {}

impl<'data, 'out> Ord for LocalizationItem<'data, 'out> {
    fn cmp(&self, other: &Self) -> Ordering {
        // We check for finiteness of score in the constructor, therefore, we are safe here.
        self.score.partial_cmp(&other.score).unwrap()
    }
}

impl<'data, 'out> PartialOrd for LocalizationItem<'data, 'out> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub type PluginInitError = String;

pub trait AardwolfPlugin {
    fn init<'data>(api: &'data Api<'data>, opts: &HashMap<String, Yaml>) -> Result<Self, PluginInitError>
    where
        Self: Sized;

    fn run_pre<'data, 'out>(&'out self, _api: &'data mut Api<'data>) -> Result<(), PluginError> {
        Ok(())
    }

    fn run_loc<'data, 'out, 'param>(
        &'out self,
        _api: &'data Api<'data>,
        _results: &'param mut Results<'data, 'out>,
    ) -> Result<(), PluginError> {
        Ok(())
    }

    fn run_post<'data, 'out>(
        &'out self,
        _api: &'data Api<'data>,
        _base: HashMap<&'data str, &'out Results<'data, 'out>>,
        _results: &'out mut Results<'data, 'out>,
    ) -> Result<(), PluginError> {
        Ok(())
    }
}
