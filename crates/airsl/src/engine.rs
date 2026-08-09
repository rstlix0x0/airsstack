//! The Lua state, configured and ready to run scripts.
//!
//! This is the crate's entry point. It exists as a distinct type from the builder so that a
//! configured engine is immutable in the ways that matter — the sandbox has been applied and the
//! host modules installed before any caller can hold one, which removes the "did I remember to
//! sandbox it" question from every call site.
//!
//! Responsibilities:
//!
//! - [`Engine`], owning the [`mlua::Lua`] state and the installed [`crate::ModuleSet`].
//! - Evaluating a [`Script`], with and without a typed return value.
//!
//! Non-responsibilities: deciding what to do about a failure. [`Engine::eval`] returns a
//! [`Result`]; [`crate::FailurePolicy`] describes how the caller should treat it.

use mlua::FromLuaMulti;

use crate::builder::{EngineBuilder, Missing};
use crate::error::{Error, Result};
use crate::instruction_budget::{BudgetExhausted, InstructionBudget};
use crate::modules::ModuleSet;
use crate::sandbox::Policy;
use crate::script::Script;
use crate::types::ModuleName;

/// Name of the single global table every host module is installed under.
pub const ROOT_TABLE: &str = "airsstack";

/// A configured Lua state.
///
/// Build one with [`Engine::builder`]. The sandbox policy is required, so an engine cannot be
/// constructed without deciding what scripts are allowed to reach.
///
/// # Examples
///
/// ```
/// use airsl::{Engine, Policy, Script};
///
/// let engine = Engine::builder().policy(Policy::confined()).build()?;
/// let script = Script::from_source("return airsstack.json.encode({ok = true})", "demo")?;
/// assert_eq!(engine.eval_to::<String>(&script)?, r#"{"ok":true}"#);
/// # Ok::<(), airsl::Error>(())
/// ```
pub struct Engine {
    lua: mlua::Lua,
    modules: ModuleSet,
    policy: Policy,
    budget: Option<InstructionBudget>,
}

impl Engine {
    /// Starts configuring an engine.
    #[must_use]
    pub const fn builder() -> EngineBuilder<Missing> {
        EngineBuilder::new()
    }

    /// Wraps an already-configured state. Called by [`EngineBuilder::build`].
    pub(crate) const fn from_parts(
        lua: mlua::Lua,
        modules: ModuleSet,
        policy: Policy,
        budget: Option<InstructionBudget>,
    ) -> Self {
        Self {
            lua,
            modules,
            policy,
            budget,
        }
    }

    /// The policy this engine was built with.
    ///
    /// The policy is fixed at construction and cannot be widened afterwards, so this describes the
    /// engine for as long as it exists.
    #[must_use]
    pub const fn policy(&self) -> &Policy {
        &self.policy
    }

    /// The underlying Lua state, for callers registering their own values after construction.
    #[must_use]
    pub const fn lua(&self) -> &mlua::Lua {
        &self.lua
    }

    /// The names of the installed host modules, in installation order.
    #[must_use]
    pub fn module_names(&self) -> Vec<&ModuleName> {
        self.modules.names()
    }

    /// Runs `script`, discarding whatever it returns.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Lua`] when the chunk fails to compile or raises while running.
    pub fn eval(&self, script: &Script) -> Result<()> {
        self.eval_to::<()>(script)
    }

    /// Runs `script` and converts its return value to `T`.
    ///
    /// The instruction budget is reset first, so every evaluation on a reused engine gets the
    /// whole ceiling rather than what the previous script left of it.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Lua`] when the chunk fails to compile, raises while running, exceeds a
    /// resource ceiling, or returns something that cannot be converted to `T`.
    pub fn eval_to<T: FromLuaMulti>(&self, script: &Script) -> Result<T> {
        if let Some(budget) = self.budget.as_ref() {
            budget.reset();
        }

        self.lua
            .load(script.source())
            .set_name(script.name().as_lua())
            .eval::<T>()
            .map_err(|error| self.classify(script, error))
    }

    /// Names the failure a script produced, separating a resource breach from a script defect.
    ///
    /// Both decisions are made on structure rather than on message text. A script is free to raise
    /// a string that reads exactly like either report, and matching on the text would let it
    /// disguise its own failure as a resource breach or the reverse.
    fn classify(&self, script: &Script, error: mlua::Error) -> Error {
        let chunk = script.name().as_str();

        if let Some(budget) = self.budget.as_ref()
            && (budget.is_exhausted() || error.downcast_ref::<BudgetExhausted>().is_some())
        {
            return Error::InstructionLimit {
                chunk: chunk.to_owned(),
                limit: budget.limit(),
            };
        }

        if let Some(limit) = self.policy.limits().memory()
            && exhausted_memory(&error)
        {
            return Error::MemoryLimit {
                chunk: chunk.to_owned(),
                limit: limit.get(),
                source: Box::new(error),
            };
        }

        Error::lua(chunk, error)
    }
}

/// Whether the VM ran out of memory anywhere in this error's chain.
///
/// The allocator failure is usually wrapped by the callback or context that was running when it
/// happened, so the outermost variant is rarely the informative one.
fn exhausted_memory(error: &mlua::Error) -> bool {
    error.chain().any(|link| {
        matches!(
            link.downcast_ref::<mlua::Error>(),
            Some(mlua::Error::MemoryError(_))
        )
    })
}

impl core::fmt::Debug for Engine {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Engine")
            .field("policy", &self.policy)
            .field("modules", &self.modules)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::unwrap_used,
        reason = "tests unwrap known-valid fixtures; a panic is the intended failure signal"
    )]

    use super::{Engine, ROOT_TABLE};
    use crate::{ExhaustedLimit, InstructionLimit, MemoryLimit, Policy, ResourceLimits, Script};

    /// Fails to compile if `T` is not shareable between threads.
    const fn assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn an_engine_can_be_sent_and_shared_between_threads() {
        assert_send_sync::<Engine>();
    }

    fn engine() -> Engine {
        Engine::builder()
            .policy(Policy::confined())
            .build()
            .unwrap()
    }

    fn script(source: &str) -> Script {
        Script::from_source(source, "test").unwrap()
    }

    #[test]
    fn the_root_table_is_named_after_the_workspace() {
        assert_eq!(ROOT_TABLE, "airsstack");
    }

    #[test]
    fn eval_runs_a_chunk_for_its_effect() {
        assert!(engine().eval(&script("local x = 1")).is_ok());
    }

    #[test]
    fn eval_to_converts_the_return_value() {
        assert_eq!(
            engine().eval_to::<i64>(&script("return 6 * 7")).unwrap(),
            42
        );
    }

    #[test]
    fn a_syntax_error_is_reported_against_the_chunk_name() {
        let err = engine().eval(&script("this is not lua")).unwrap_err();
        assert!(err.to_string().contains("test"), "{err}");
    }

    #[test]
    fn a_runtime_error_is_returned_not_panicked() {
        let err = engine().eval(&script("error('boom')")).unwrap_err();
        assert!(err.to_string().contains("boom"), "{err}");
    }

    #[test]
    fn the_root_table_is_visible_to_scripts() {
        let found = engine()
            .eval_to::<String>(&script("return type(airsstack)"))
            .unwrap();
        assert_eq!(found, "table");
    }

    #[test]
    fn an_endless_loop_is_named_as_an_instruction_breach() {
        let engine = Engine::builder()
            .policy(Policy::confined().with_limits(
                ResourceLimits::none().with_instructions(Some(InstructionLimit::count(100_000))),
            ))
            .build()
            .unwrap();
        let err = engine.eval(&script("while true do end")).unwrap_err();
        assert_eq!(err.exhausted_limit(), Some(ExhaustedLimit::Instructions));
    }

    #[test]
    fn an_unbounded_allocation_is_named_as_a_memory_breach() {
        let engine =
            Engine::builder()
                .policy(Policy::confined().with_limits(
                    ResourceLimits::none().with_memory(Some(MemoryLimit::mebibytes(1))),
                ))
                .build()
                .unwrap();
        let err = engine
            .eval(&script("local t = {} for i = 1, 1e9 do t[i] = i end"))
            .unwrap_err();
        assert_eq!(err.exhausted_limit(), Some(ExhaustedLimit::Memory));
    }

    #[test]
    fn a_script_that_merely_failed_is_not_named_as_a_breach() {
        let engine = engine();
        for source in ["error('boom')", "this is not lua", "error('out of memory')"] {
            let err = engine.eval(&script(source)).unwrap_err();
            assert_eq!(err.exhausted_limit(), None, "{source}");
        }
    }

    #[test]
    fn the_instruction_budget_is_restored_between_scripts_on_one_engine() {
        let engine = Engine::builder()
            .policy(Policy::confined().with_limits(
                ResourceLimits::none().with_instructions(Some(InstructionLimit::count(1_000_000))),
            ))
            .build()
            .unwrap();

        assert!(engine.eval(&script("while true do end")).is_err());
        assert_eq!(engine.eval_to::<i64>(&script("return 7")).unwrap(), 7);
    }

    #[test]
    fn module_names_reports_the_installed_standard_library() {
        let engine = engine();
        let names: Vec<_> = engine
            .module_names()
            .iter()
            .map(ToString::to_string)
            .collect();
        assert!(names.contains(&String::from("json")), "{names:?}");
    }
}
