use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::fmt;

use yaml_rust::Yaml;

use crate::api::Api;
use crate::raw::data::{Loc, Statement, TestName};

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

// TODO: Add root_stmt: &'a Statemen in which a fault localization should indicate a statement
//       that should be mainly blamed in the code snippet in loc (which can be more than one statement).
pub struct LocalizationItem<'a> {
    pub loc: Loc,
    pub score: f32,
    pub rationale: Rationale,
    pub links: Vec<&'a LocalizationItem<'a>>,
}

#[derive(Debug)]
pub enum InvalidLocalizationItem {
    InvalidScore(f32),
    EmptyRationale,
}

impl<'a> LocalizationItem<'a> {
    pub fn new(
        loc: Loc,
        score: f32,
        rationale: Rationale,
    ) -> Result<Self, InvalidLocalizationItem> {
        // The check whether the score is finite is important for total order of items.
        match (score.is_finite(), rationale.is_empty()) {
            (false, _) => Err(InvalidLocalizationItem::InvalidScore(score)),
            (_, true) => Err(InvalidLocalizationItem::EmptyRationale),
            _ => Ok(LocalizationItem {
                loc,
                score,
                rationale,
                links: Vec::new(),
            }),
        }
    }

    pub fn link(&'a mut self, other: &'a LocalizationItem<'a>) {
        self.links.push(other);
    }
}

impl<'a> PartialEq for LocalizationItem<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.score == other.score
    }
}

impl<'a> Eq for LocalizationItem<'a> {}

impl<'a> Ord for LocalizationItem<'a> {
    fn cmp(&self, other: &Self) -> Ordering {
        // We check for finiteness of score in the constructor, therefore, we are safe here.
        self.score.partial_cmp(&other.score).unwrap()
    }
}

impl<'a> PartialOrd for LocalizationItem<'a> {
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
    //       that lists only N most suspicious elements.
    fn run_loc<'a, 'b>(&'b self, _api: &'a Api<'a>) -> Vec<LocalizationItem<'b>> {
        Vec::new()
    }

    // TODO: Determine real API of this method.
    fn run_post<'a, 'b>(&'b self, _api: &'a Api<'a>) -> Vec<LocalizationItem<'b>> {
        Vec::new()
    }
}
