//! A single-turn eval case: a name, a prompt, and a set of scorers.

use crate::agent::evals::score::Scorer;
use crate::agent::types::Prompt;

/// One single-turn eval case.
///
/// Name and prompt are both required, so they are constructor arguments; scorers
/// are added with [`Case::scorer`] and run in the order added.
pub struct Case {
    name: String,
    prompt: Prompt,
    scorers: Vec<Box<dyn Scorer>>,
}

impl Case {
    /// Build a case with the given name and prompt.
    pub fn new(name: impl Into<String>, prompt: impl Into<Prompt>) -> Self {
        Self {
            name: name.into(),
            prompt: prompt.into(),
            scorers: Vec::new(),
        }
    }

    /// Add a scorer. Chainable; scorers run in the order added.
    #[must_use]
    pub fn scorer(mut self, scorer: impl Scorer + 'static) -> Self {
        self.scorers.push(Box::new(scorer));
        self
    }

    /// The case name.
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    /// The case prompt.
    pub(crate) const fn prompt(&self) -> &Prompt {
        &self.prompt
    }

    /// The scorers, in run order.
    pub(crate) fn scorers(&self) -> &[Box<dyn Scorer>] {
        &self.scorers
    }
}

#[cfg(test)]
mod tests {
    use super::Case;
    use crate::agent::evals::scorers::{contains, no_error};

    #[test]
    fn new_case_has_no_scorers() {
        let case = Case::new("empty", "hi");
        assert_eq!(case.name(), "empty");
        assert_eq!(case.prompt().as_str(), "hi");
        assert!(case.scorers().is_empty());
    }

    #[test]
    fn scorer_appends_in_order() {
        let case = Case::new("c", "hi")
            .scorer(contains("a"))
            .scorer(no_error());
        assert_eq!(case.scorers().len(), 2);
        assert_eq!(case.scorers()[0].label(), "contains");
        assert_eq!(case.scorers()[1].label(), "no_error");
    }
}
