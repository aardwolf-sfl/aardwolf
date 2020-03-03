use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::fmt;

use yaml_rust::Yaml;

use crate::api::Api;
use crate::raw::data::{Loc, Statement, TestName};

pub mod invariants;
pub mod prob_graph;
pub mod sbfl;

pub struct IrrelevantItems<'a> {
    // Store relevant items and remove them if they are marked as irrelevant.
    pub stmts: HashSet<u64>,
    pub tests: HashSet<&'a TestName>,
}

impl<'a> IrrelevantItems<'a> {
    pub fn new(api: &'a Api<'a>) -> Self {
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

pub struct Results<'a, 'b> {
    items: Vec<LocalizationItem<'a, 'b>>,
    n_results: usize,
}

impl<'a, 'b> Results<'a, 'b> {
    pub fn new(n_results: usize) -> Self {
        Results {
            items: Vec::with_capacity(n_results),
            n_results,
        }
    }

    pub fn add(&mut self, item: LocalizationItem<'a, 'b>) {
        // TODO: Manage a sorted vector of size n_results with best results encountered so far.
        self.items.push(item);
    }

    pub fn into_vec(mut self) -> Vec<LocalizationItem<'a, 'b>> {
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

pub struct LocalizationItem<'a, 'b> {
    pub loc: Loc,
    pub root_stmt: &'a Statement,
    pub score: f32,
    pub rationale: Rationale,
    pub links: Vec<&'b LocalizationItem<'a, 'b>>,
}

#[derive(Debug)]
pub enum InvalidLocalizationItem {
    InvalidScore(f32),
    EmptyRationale,
}

impl<'a, 'b> LocalizationItem<'a, 'b> {
    pub fn new(
        loc: Loc,
        root_stmt: &'a Statement,
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

    pub fn link(&'b mut self, other: &'a LocalizationItem<'a, 'b>) {
        self.links.push(other);
    }
}

impl<'a, 'b> PartialEq for LocalizationItem<'a, 'b> {
    fn eq(&self, other: &Self) -> bool {
        self.score == other.score
    }
}

impl<'a, 'b> Eq for LocalizationItem<'a, 'b> {}

impl<'a, 'b> Ord for LocalizationItem<'a, 'b> {
    fn cmp(&self, other: &Self) -> Ordering {
        // We check for finiteness of score in the constructor, therefore, we are safe here.
        self.score.partial_cmp(&other.score).unwrap()
    }
}

impl<'a, 'b> PartialOrd for LocalizationItem<'a, 'b> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub type PluginInitError = String;

pub trait AardwolfPlugin {
    fn init<'a>(api: &'a Api<'a>, opts: &HashMap<String, Yaml>) -> Result<Self, PluginInitError>
    where
        Self: Sized;

    fn run_pre<'a, 'b>(&'b self, _api: &'a mut Api<'a>) -> Result<(), PluginError> {
        Ok(())
    }

    fn run_loc<'a, 'b, 'c>(
        &'b self,
        _api: &'a Api<'a>,
        _results: &'c mut Results<'a, 'b>,
    ) -> Result<(), PluginError> {
        Ok(())
    }

    fn run_post<'a, 'b>(
        &'b self,
        _api: &'a Api<'a>,
        _base: HashMap<&'a str, &'b Results<'a, 'b>>,
        _results: &'b mut Results<'a, 'b>,
    ) -> Result<(), PluginError> {
        Ok(())
    }
}
