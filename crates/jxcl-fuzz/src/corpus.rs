//! Corpus management for fuzzing test cases.
//!
//! A corpus is a collection of test cases (byte sequences) that have been
//! generated or discovered during fuzzing. It can be used to track test cases
//! with metadata, maintain coverage information, or replay known failures.

use std::fmt;

/// A single test case in the fuzzing corpus.
///
/// Represents a byte sequence that has been executed or is intended to be
/// executed by a fuzzing target. Can carry metadata about where it came from
/// (seed, mutation, discovered) and how many times it's been executed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestCase {
    /// The input bytes for this test case.
    pub input: Vec<u8>,
    /// Number of times this test case has been executed.
    executions: usize,
}

impl TestCase {
    /// Create a new test case from an input byte sequence.
    pub fn new(input: Vec<u8>) -> Self {
        TestCase {
            input,
            executions: 0,
        }
    }

    /// Record an execution of this test case.
    pub fn record_execution(&mut self) {
        self.executions += 1;
    }

    /// Get the number of times this test case has been executed.
    pub fn execution_count(&self) -> usize {
        self.executions
    }

    /// Get the length of the input bytes.
    pub fn len(&self) -> usize {
        self.input.len()
    }

    /// Check if the input is empty.
    pub fn is_empty(&self) -> bool {
        self.input.is_empty()
    }
}

impl fmt::Display for TestCase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TestCase(len={}, executions={})",
            self.len(),
            self.executions
        )
    }
}

/// A collection of test cases used during fuzzing.
///
/// Maintains a collection of test cases that have been generated or discovered
/// during fuzzing. Can be used to track all test cases, replay known failures,
/// or maintain a seed corpus for future runs.
#[derive(Debug, Clone)]
pub struct Corpus {
    cases: Vec<TestCase>,
}

impl Corpus {
    /// Create a new empty corpus.
    pub fn new() -> Self {
        Corpus { cases: Vec::new() }
    }

    /// Create a corpus with a pre-allocated capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Corpus {
            cases: Vec::with_capacity(capacity),
        }
    }

    /// Add a test case to the corpus.
    pub fn add(&mut self, case: TestCase) {
        self.cases.push(case);
    }

    /// Get the number of test cases in the corpus.
    pub fn len(&self) -> usize {
        self.cases.len()
    }

    /// Check if the corpus is empty.
    pub fn is_empty(&self) -> bool {
        self.cases.is_empty()
    }

    /// Get a reference to all test cases.
    pub fn all(&self) -> &[TestCase] {
        &self.cases
    }

    /// Get a mutable reference to all test cases.
    pub fn all_mut(&mut self) -> &mut [TestCase] {
        &mut self.cases
    }

    /// Get a reference to a specific test case by index.
    pub fn get(&self, index: usize) -> Option<&TestCase> {
        self.cases.get(index)
    }

    /// Get a mutable reference to a specific test case by index.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut TestCase> {
        self.cases.get_mut(index)
    }

    /// Clear all test cases from the corpus.
    pub fn clear(&mut self) {
        self.cases.clear();
    }

    /// Get the total number of executions across all test cases.
    pub fn total_executions(&self) -> usize {
        self.cases.iter().map(|c| c.execution_count()).count()
    }

    /// Get the total number of bytes across all test cases.
    pub fn total_bytes(&self) -> usize {
        self.cases.iter().map(|c| c.len()).sum()
    }
}

impl Default for Corpus {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for Corpus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Corpus(cases={}, total_bytes={})",
            self.len(),
            self.total_bytes()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_case_creation() {
        let input = vec![0x42, 0x43, 0x44];
        let case = TestCase::new(input.clone());

        assert_eq!(case.input, input);
        assert_eq!(case.len(), 3);
        assert_eq!(case.execution_count(), 0);
    }

    #[test]
    fn test_case_execution_tracking() {
        let mut case = TestCase::new(vec![0xFF]);

        assert_eq!(case.execution_count(), 0);
        case.record_execution();
        assert_eq!(case.execution_count(), 1);
        case.record_execution();
        case.record_execution();
        assert_eq!(case.execution_count(), 3);
    }

    #[test]
    fn test_case_empty_check() {
        let empty = TestCase::new(vec![]);
        let non_empty = TestCase::new(vec![0x42]);

        assert!(empty.is_empty());
        assert!(!non_empty.is_empty());
    }

    #[test]
    fn corpus_creation() {
        let corpus = Corpus::new();
        assert!(corpus.is_empty());
        assert_eq!(corpus.len(), 0);
    }

    #[test]
    fn corpus_add_cases() {
        let mut corpus = Corpus::new();

        for i in 0..5 {
            corpus.add(TestCase::new(vec![i as u8]));
        }

        assert_eq!(corpus.len(), 5);
        assert!(!corpus.is_empty());
    }

    #[test]
    fn corpus_access_cases() {
        let mut corpus = Corpus::new();
        let case1 = TestCase::new(vec![0x11, 0x22]);
        let case2 = TestCase::new(vec![0x33, 0x44, 0x55]);

        corpus.add(case1.clone());
        corpus.add(case2.clone());

        assert_eq!(corpus.get(0).unwrap().input, case1.input);
        assert_eq!(corpus.get(1).unwrap().input, case2.input);
        assert_eq!(corpus.all().len(), 2);
    }

    #[test]
    fn corpus_clear() {
        let mut corpus = Corpus::new();
        corpus.add(TestCase::new(vec![0x42]));
        corpus.add(TestCase::new(vec![0x43]));

        assert_eq!(corpus.len(), 2);
        corpus.clear();
        assert_eq!(corpus.len(), 0);
    }

    #[test]
    fn corpus_total_bytes() {
        let mut corpus = Corpus::new();
        corpus.add(TestCase::new(vec![0; 10]));
        corpus.add(TestCase::new(vec![0; 20]));
        corpus.add(TestCase::new(vec![0; 30]));

        assert_eq!(corpus.total_bytes(), 60);
    }

    #[test]
    fn test_case_display() {
        let case = TestCase::new(vec![0x42; 5]);
        let display_str = format!("{}", case);
        assert!(display_str.contains("len=5"));
    }

    #[test]
    fn corpus_display() {
        let mut corpus = Corpus::new();
        corpus.add(TestCase::new(vec![0; 10]));
        corpus.add(TestCase::new(vec![0; 20]));

        let display_str = format!("{}", corpus);
        assert!(display_str.contains("cases=2"));
        assert!(display_str.contains("total_bytes=30"));
    }
}
