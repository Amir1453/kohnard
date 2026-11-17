// The following code was directly taken and/or inspired by rustc.

/// Globals for the compiler session. Contains the symbol interner.
pub struct SessionGlobals {
    symbol_interner: crate::frontend::SymbolInterner,
    type_interner: crate::types::TypeInterner,
}

impl SessionGlobals {
    fn new() -> SessionGlobals {
        SessionGlobals {
            symbol_interner: crate::frontend::SymbolInterner::new(),
            type_interner: crate::types::TypeInterner::new(),
        }
    }

    /// Returns a reference to the symbol interner.
    #[must_use]
    #[inline(always)]
    pub const fn get_symbol_interner(&self) -> &crate::frontend::SymbolInterner {
        &self.symbol_interner
    }

    /// Returns a reference to the type interner.
    #[must_use]
    #[inline(always)]
    pub const fn get_type_interner(&self) -> &crate::types::TypeInterner {
        &self.type_interner
    }
}

scoped_tls::scoped_thread_local!(static SESSION_GLOBALS: SessionGlobals);

/// Executes a closure f, where the SessionGlobals will be available.
#[inline]
pub fn with_session_globals<R, F>(f: F) -> R
where
    F: FnOnce(&SessionGlobals) -> R,
{
    SESSION_GLOBALS.with(f)
}

/// Creates the SessionGlobals and executes a closure f.
/// Inside of f, the SessionGlobals will be available.
pub fn create_session_globals_then<R>(f: impl FnOnce() -> R) -> R {
    assert!(
        !SESSION_GLOBALS.is_set(),
        "SESSION_GLOBALS should never be overwritten! \
         Use another thread if you need another SessionGlobals"
    );
    let session_globals = SessionGlobals::new();
    SESSION_GLOBALS.set(&session_globals, f)
}
