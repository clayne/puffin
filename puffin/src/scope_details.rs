use crate::ScopeId;
use std::{borrow::Cow, collections::HashMap, sync::Arc};

#[derive(Default, Clone)]
struct Inner {
    // Store a both-way map, memory wise this can be a bit redundant but allows for faster access of information by external libs.
    pub(crate) scope_id_to_details: HashMap<ScopeId, Arc<ScopeDetails>>,
    pub(crate) type_to_scope_id: HashMap<Cow<'static, str>, ScopeId>,
}

/// A collection of scope details containing more information about a recorded profile scope.
#[derive(Default, Clone)]
pub struct ScopeCollection(Inner);

impl ScopeCollection {
    /// Fetches scope details by scope id.
    #[inline]
    pub fn fetch_by_id(&self, scope_id: &ScopeId) -> Option<&Arc<ScopeDetails>> {
        self.0.scope_id_to_details.get(scope_id)
    }

    /// Fetches scope details by scope name.
    #[inline]
    pub fn fetch_by_name(&self, scope_name: &str) -> Option<&ScopeId> {
        self.0.type_to_scope_id.get(scope_name)
    }

    /// Insert a scope into the collection.
    /// This method asserts the scope id is set which only puffin should do.
    /// Custom sinks might use this method to store new scope details received from puffin.
    pub fn insert(&mut self, scope_details: Arc<ScopeDetails>) -> Arc<ScopeDetails> {
        let scope_id = scope_details
            .scope_id
            .expect("`ScopeDetails` missing `ScopeId`");

        self.0
            .type_to_scope_id
            .insert(scope_details.name().clone(), scope_id);
        self.0
            .scope_id_to_details
            .entry(scope_id)
            .or_insert(scope_details)
            .clone()
    }

    /// Fetches all registered scopes and their ids.
    /// Useful for fetching scope id by it's scope name.
    /// For profiler scopes and user scopes this is the manual provided name.
    /// For function profiler scopes this is the function name.
    #[inline]
    pub fn scopes_by_name(&self) -> &HashMap<Cow<'static, str>, ScopeId> {
        &self.0.type_to_scope_id
    }

    /// Fetches all registered scopes.
    /// Useful for fetching scope details by a scope id.
    #[inline]
    pub fn scopes_by_id(&self) -> &HashMap<ScopeId, Arc<ScopeDetails>> {
        &self.0.scope_id_to_details
    }
}

/// Scopes are identified by user-provided name while functions are identified by the function name.
#[derive(Debug, Clone, PartialEq, Hash, PartialOrd, Ord, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub enum ScopeType {
    /// The scope is a function profile scope generated by `puffin::profile_function!`.
    Function,
    /// The named scope is a profile scope inside a function generated by `puffin::profile_scope!` or registered manually.
    /// It is identified by a unique name.
    Named,
}

impl ScopeType {
    /// Returns a string representation of this scope type.
    pub fn type_str(&self) -> &'static str {
        match self {
            ScopeType::Function => "function scope",
            ScopeType::Named => "named",
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Hash, PartialOrd, Ord, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
/// Detailed information about a scope.
pub struct ScopeDetails {
    /// Unique scope identifier.
    /// Always initialized once registered.
    /// It is `None` when an external library has yet to register this scope.
    pub(crate) scope_id: Option<ScopeId>,

    /// A name for a profile scope, a function profile scope does not have a custom provided name.
    pub scope_name: Option<Cow<'static, str>>,

    /// The function name of the function in which this scope is contained.
    /// The name might be slightly modified to represent a short descriptive representation.
    pub function_name: Cow<'static, str>,

    /// The file path in which this scope is contained.
    /// The path might be slightly modified to represent a short descriptive representation.
    pub file_path: Cow<'static, str>,

    /// The exact line number at which this scope is located.
    pub line_nr: u32,
}

impl ScopeDetails {
    /// Creates a new user scope with a unique name.
    pub fn from_scope_name<T>(scope_name: T) -> Self
    where
        T: Into<Cow<'static, str>>,
    {
        Self {
            scope_id: None,
            scope_name: Some(scope_name.into()),
            function_name: Default::default(),
            file_path: Default::default(),
            line_nr: Default::default(),
        }
    }

    /// Creates a new user scope with a unique id allocated by puffin.
    /// This function should not be exposed as only puffin should allocate ids for scopes.
    pub(crate) fn from_scope_id(scope_id: ScopeId) -> Self {
        Self {
            scope_id: Some(scope_id),
            scope_name: None,
            function_name: Default::default(),
            file_path: Default::default(),
            line_nr: Default::default(),
        }
    }

    /// Scope in a function.
    #[inline]
    pub fn with_function_name<T>(mut self, name: T) -> Self
    where
        T: Into<Cow<'static, str>>,
    {
        self.function_name = name.into();
        self
    }

    /// Scope in a file.
    #[inline]
    pub fn with_file<T>(mut self, file: T) -> Self
    where
        T: Into<Cow<'static, str>>,
    {
        self.file_path = file.into();
        self
    }

    /// Scope at a line number.
    #[inline]
    pub fn with_line_nr(mut self, line_nr: u32) -> Self {
        self.line_nr = line_nr;
        self
    }

    /// Returns the scope name if this is a profile scope or else the function name.
    pub fn name(&self) -> &Cow<'static, str> {
        self.scope_name.as_ref().map_or(&self.function_name, |x| x)
    }

    /// Returns what type of scope this is.
    pub fn scope_type(&self) -> ScopeType {
        // scope name is only set for named scopes.
        if self.scope_name.is_some() {
            ScopeType::Named
        } else {
            ScopeType::Function
        }
    }

    /// Returns the exact location of the profile scope formatted as `file:line_nr`
    #[inline]
    pub fn location(&self) -> String {
        if self.line_nr != 0 {
            format!("{}:{}", self.file_path, self.line_nr)
        } else {
            format!("{}", self.file_path)
        }
    }

    // This function should not be exposed as only puffin should allocate ids.
    #[inline]
    pub(crate) fn with_scope_id(mut self, scope_id: ScopeId) -> Self {
        self.scope_id = Some(scope_id);
        self
    }

    // This function should not be exposed as users are supposed to provide scope name in constructor.
    #[inline]
    pub(crate) fn with_scope_name<T>(mut self, scope_name: T) -> Self
    where
        T: Into<Cow<'static, str>>,
    {
        self.scope_name = Some(scope_name.into());
        self
    }
}
