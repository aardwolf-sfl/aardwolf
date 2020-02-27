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
    pub stmts: HashSet<u64>,
    pub tests: HashSet<&'a TestName>,
}

impl<'a> IrrelevantItems<'a> {
    pub fn new() -> Self {
        IrrelevantItems {
            stmts: HashSet::new(),
            tests: HashSet::new(),
        }
    }

    pub fn mark_stmt(&mut self, stmt: &Statement) {
        self.stmts.insert(stmt.id);
    }

    pub fn mark_test(&mut self, test: &'a TestName) {
        self.tests.insert(test);
    }

    pub fn is_empty(&self) -> bool {
        self.stmts.is_empty() && self.tests.is_empty()
    }
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

    fn run_pre<'a, 'b>(&'b self, _api: &'a Api<'a>) -> IrrelevantItems<'b> {
        IrrelevantItems::new()
    }

    // TODO: Return Iterator instead of allocated array. This will allow to implement a more efficient structure
    //       that lists only N most suspicious elements. (NOTE: lifetime issues might be an obstacle).
    fn run_loc<'a, 'b>(&'b self, _api: &'a Api<'a>) -> Vec<LocalizationItem<'a, 'b>> {
        Vec::new()
    }

    // TODO: Determine real API of this method.
    fn run_post<'a, 'b>(&'b self, _api: &'a Api<'a>) -> Vec<LocalizationItem<'a, 'b>> {
        Vec::new()
    }
}
