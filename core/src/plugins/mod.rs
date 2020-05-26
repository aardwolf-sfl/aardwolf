//! Aardwolf plugins.
//!
//! There are currently three fault localization plugins and two auxiliary
//! plugins.
//!
//! * [`SBFL`] -- Simple yet power technique for fault localization based on
//!   coverage program spectrum.
//! * [`Probabilistic dependence`] -- Fault localization technique based on
//!   control- and data dependencies and estimation of probabilities of certain
//!   program states.
//! * [`Likely invariants`] -- Fault localization based on inferred program
//!   invariants and checking for their violations.
//! * [`Basic Block collection`] -- This plugin collects multiple statements in
//!   a basic block which have the same suspiciousness score. This particularly
//!   useful for SBFL.
//! * [`Irrelevant statements`] -- Marks statements as irrelevant if they were
//!   not executed in any failing test cases.
//!
//! [`SBFL`]: sbfl/index.html
//! [`Probabilistic dependence`]: prob_graph/index.html
//! [`Likely invariants`]: invariants/index.html
//! [`Basic Block collection`]: collect_bb/index.html
//! [`Irrelevant statements`]: irrelevant/index.html

use std::collections::{HashMap, HashSet};
use std::fmt;

use yaml_rust::Yaml;

use crate::api::Api;
use crate::arena::{P, S};
use crate::data::{
    statement::{Loc, Statement},
    types::{StmtId, TestName},
};
use crate::queries::{QueryInitError, Stmts, Tests};

pub mod collect_bb;
pub mod invariants;
pub mod irrelevant;
pub mod prob_graph;
pub mod sbfl;

/// This structure records what statements and tests are considered as irrelevant.
pub struct IrrelevantItems {
    // Store relevant items and remove them if they are marked as irrelevant.
    pub stmts: HashSet<StmtId>,
    pub tests: HashSet<S<TestName>>,
}

impl IrrelevantItems {
    /// Initializes the data structure.
    pub fn new(api: &Api) -> Self {
        // By default, all items are relevant.
        IrrelevantItems {
            stmts: api.query::<Stmts>().unwrap().iter_ids().copied().collect(),
            tests: api
                .query::<Tests>()
                .unwrap()
                .iter_names()
                .copied()
                .collect(),
        }
    }

    /// Marks given statement as irrelevant.
    pub fn mark_stmt(&mut self, stmt: &Statement) {
        self.stmts.remove(&stmt.id);
    }

    /// Marks given test case as irrelevant.
    pub fn mark_test(&mut self, test: &S<TestName>) {
        self.tests.remove(test);
    }

    /// Gets whether given statement is marked as irrelevant.
    pub fn is_stmt_relevant(&self, stmt: &Statement) -> bool {
        self.stmts.contains(&stmt.id)
    }

    pub fn is_test_relevant(&self, test: &S<TestName>) -> bool {
        self.tests.contains(test)
    }
}

/// Collection that holds the localization results.
///
/// It is optimized in such way that it stores only the most suspicious elements
/// of size equal to the number of results limit set by the user. It also checks
/// if a identical item is already in the collection and if so, it keeps just
/// the more suspicious one.
#[derive(Clone)]
pub struct Results {
    // TODO: Make a specialized data structure which combines HashMap and BinaryHeap
    //       (maybe custom implementation of binary heap is necessary).
    // TODO: Make two variants of the results (enum)
    //         - first, which just blindly adds all new items up to set limit and keeps them sorted,
    //         - second, which will also check if an existing item is added again and keeps only the most suspicious.
    //       Plugins can then switch between these variants using a method (they get mutable reference).
    items: HashMap<Loc, LocalizationItem>,
    n_results: usize,
    max_score: f32,
}

impl Results {
    /// Initializes the results data structure. The number of stored items is
    /// limited by `n_results` argument. If it is set to 0, then all added items
    /// are kept.
    pub fn new(n_results: usize) -> Self {
        Results {
            items: HashMap::with_capacity(n_results),
            n_results,
            max_score: 0.0,
        }
    }

    /// Adds the item into the collection while keeping just limited number of
    /// the most suspicious items.
    pub fn add(&mut self, item: LocalizationItem) {
        if item.score > self.max_score {
            self.max_score = item.score;
        }

        if self.n_results == 0 || self.items.len() < self.n_results {
            // Check if there exists an item with the same location.
            if let Some(original) = self.items.get(&item.loc) {
                // If so, add new item only if it has higher suspiciousness.
                if item.score > original.score {
                    self.items.insert(item.loc, item);
                }
            } else {
                // If not, just add the item.
                self.items.insert(item.loc, item);
            }
        } else {
            // Check if there exists an item with the same location.
            if let Some(original) = self.items.get(&item.loc) {
                // If so, add new item only if it has higher suspiciousness.
                if item.score > original.score {
                    self.items.insert(item.loc, item);
                }
            } else {
                // If not, replace it with the worst one.
                let (loc, worst) = self
                    .items
                    .iter()
                    .min_by(|(_, lhs), (_, rhs)| lhs.score.partial_cmp(&rhs.score).unwrap())
                    .map(|(loc, worst)| (*loc, worst.score))
                    .unwrap();

                if item.score > worst {
                    self.items.remove(&loc);
                    self.items.insert(item.loc, item);
                }
            }
        }
    }

    /// Checks if there are any items in the collection.
    pub fn any(&self) -> bool {
        !self.items.is_empty()
    }

    /// Normalized the results to be in [0, 1] range.
    pub fn normalize(self) -> NormalizedResults {
        let max_score = self.max_score;
        let mut items = self
            .items
            .into_iter()
            .map(|(_, item)| item.normalize(max_score))
            .collect::<Vec<_>>();

        // Use stable algorithm when sorting the items to not break plugins
        // which sort the results using another criterion.
        // Also, we can safely unwrap the result of partial_cmp,
        // because score is checked for finiteness in LocalizationItem constructor.
        items.sort_by(|lhs, rhs| rhs.score.partial_cmp(&lhs.score).unwrap());

        NormalizedResults { items }
    }

    /// Returns an iterator over the items.
    pub fn iter(&self) -> impl Iterator<Item = &LocalizationItem> {
        self.items.values()
    }
}

/// A collection of normalized results. The items are considered as frozen and
/// the data structure does not allow to modify itself in any way.
pub struct NormalizedResults {
    items: Vec<LocalizationItem>,
}

impl NormalizedResults {
    /// Returns an iterator over the items.
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &LocalizationItem> {
        self.items.iter()
    }
}

impl IntoIterator for NormalizedResults {
    type Item = LocalizationItem;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.into_iter()
    }
}

#[derive(Debug)]
pub enum PluginError {
    Inner(String),
    QueryError(QueryInitError),
}

impl From<QueryInitError> for PluginError {
    fn from(err: QueryInitError) -> Self {
        PluginError::QueryError(err)
    }
}

impl From<()> for PluginError {
    fn from(_err: ()) -> Self {
        // We assume that when the error type is unit type, then such error is
        // never returned.
        unreachable!()
    }
}

impl fmt::Display for PluginError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PluginError::Inner(error) => write!(f, "plugin error: {}", error),
            PluginError::QueryError(_) => write!(f, "plugin error when requesting data"),
        }
    }
}

/// A fragment of a rationale. At the moment, it can be either text or location.
/// These fragments are then concatenated into single string while formatting
/// the locations in a way most suitable for UI used.
#[derive(Clone, PartialEq, Eq)]
pub enum RationaleChunk {
    Text(String),
    Anchor(Loc),
}

/// Represents a rationale for a hypothesis. It consists of text and source code
/// locations.
#[derive(Clone, PartialEq, Eq)]
pub struct Rationale(Vec<RationaleChunk>);

impl Rationale {
    /// Creates an empty rationale.
    pub fn new() -> Self {
        Rationale(Vec::new())
    }

    /// Adds given text to the rationale.
    pub fn add_text<T: Into<String>>(&mut self, text: T) -> &mut Self {
        self.0.push(RationaleChunk::Text(text.into()));
        self
    }

    /// Adds given location to the rationale.
    pub fn add_anchor(&mut self, anchor: Loc) -> &mut Self {
        self.0.push(RationaleChunk::Anchor(anchor));
        self
    }

    /// Adds newline character to the rationale.
    pub fn newline(&mut self) -> &mut Self {
        self.0.push(RationaleChunk::Text(String::from("\n")));
        self
    }

    /// Adds two newline characters to the rationale.
    pub fn paragraph(&mut self) -> &mut Self {
        self.0.push(RationaleChunk::Text(String::from("\n\n")));
        self
    }

    /// Joins two rationales producing a new one.
    pub fn join(&self, other: &Self) -> Self {
        let chunks = self.0.iter().chain(other.0.iter()).cloned().collect();
        Rationale(chunks)
    }

    /// Checks if the rationale is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns an immutable reference to the fragments from which the rationale
    /// is composed.
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

/// An item produced by a fault localization technique. It holds all information
/// that is then presented to the user.
#[derive(Clone, PartialEq)]
pub struct LocalizationItem {
    /// Location of the element. It can be a code snippet of whatever size.
    /// Fault localization techniques should work on the statement granularity
    /// and leave the merging to bigger blocks to other plugins.
    pub loc: Loc,
    /// Root statement that was the cause to blame to code snippet. Might be
    /// useful for some plugins.
    pub root_stmt: P<Statement>,
    /// The suspiciousness score. The higher the score is, the higher the item
    /// is displayed to the user. The absolute value is not that important as it
    /// is normalized into [0, 1] range later anyway.
    pub score: f32,
    /// Rationale for the suspiciousness of the item.
    pub rationale: Rationale,
}

#[derive(Debug)]
pub enum InvalidLocalizationItem {
    InvalidScore(f32),
    EmptyRationale,
}

impl LocalizationItem {
    /// Initializes new result item. It checks whether all required information
    /// is provided, i.e., if the score has finite value (not an infinity or
    /// NaN) and if the rationale is not empty.
    pub fn new(
        loc: Loc,
        root_stmt: P<Statement>,
        score: f32,
        rationale: Rationale,
    ) -> Result<Self, InvalidLocalizationItem> {
        // The check whether the score is finite is important for total order of items.
        // TODO: Check that the score is a non-negative number.
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

    /// Normalizes the score to be in range [0, 1]. Given `max_score` value must
    /// be the greatest score encountered in a single results collection.
    pub fn normalize(self, max_score: f32) -> Self {
        LocalizationItem {
            score: self.score / max_score,
            ..self
        }
    }
}

pub type PluginInitError = String;

// TODO: General pipeline:
// ```
// fn run(
//     &self,
//     api: &Api,
//     preprocessing: &mut Preprocessing,
//     results: &mut Results,
//     finished: &HashMap<&str, &NormalizedResults>,
//     metadata: &mut Metadata
// ) -> Result<(), PluginError>
// ```

/// All plugins in Aardwolf implement this trait.
pub trait AardwolfPlugin {
    /// Initializes the plugin given the API and options.
    fn init(api: &Api, opts: &HashMap<String, Yaml>) -> Result<Self, PluginInitError>
    where
        Self: Sized;

    // TODO: Make general structure Preprocessing instead of IrrelevantItems.
    //  Runs te preprocessing stage in the pipeline. The default implementation
    //  does not nothing.
    fn run_pre(&self, _api: &Api, _irrelevant: &mut IrrelevantItems) -> Result<(), PluginError> {
        Ok(())
    }

    /// Runs the localization stage in the pipeline by filling the results
    /// collection. The default implementation does not nothing.
    fn run_loc(
        &self,
        _api: &Api,
        _results: &mut Results,
        _irrelevant: &IrrelevantItems,
    ) -> Result<(), PluginError> {
        Ok(())
    }

    /// Runs the postprocessing stage in the pipeline filling the results based
    /// on the results computed in the previous stage. The default
    /// implementation does not nothing.
    fn run_post(
        &self,
        _api: &Api,
        _base: &HashMap<&str, &NormalizedResults>,
        _results: &mut Results,
    ) -> Result<(), PluginError> {
        Ok(())
    }
}
