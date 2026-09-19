//! A minimal string interner. Every identifier that appears in a
//! [`crate::Term`] -- a bound-variable name hint, a free/local
//! variable, or a global constant -- is interned into a [`Symbol`]
//! (a cheap, `Copy`, hashable index) rather than carried around as a
//! `String`. This keeps [`crate::Term`] cheap to hash and compare,
//! which is what makes hash-consing in [`crate::TermArena`] worthwhile.

use std::collections::HashMap;

/// An interned identifier. Two `Symbol`s are equal if and only if the
/// strings they were interned from are equal -- comparing symbols
/// never needs to look at the underlying string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Symbol(u32);

/// Interns strings into [`Symbol`]s and back. Not thread-shared: each
/// [`crate::TermArena`] owns one.
#[derive(Debug, Default)]
pub struct Interner {
    strings: Vec<String>,
    lookup: HashMap<String, Symbol>,
}

impl Interner {
    pub fn new() -> Self {
        Interner::default()
    }

    /// Intern `s`, returning the same [`Symbol`] every time this
    /// interner is asked to intern an equal string.
    pub fn intern(&mut self, s: &str) -> Symbol {
        if let Some(sym) = self.lookup.get(s) {
            return *sym;
        }
        let id = self.strings.len() as u32;
        self.strings.push(s.to_string());
        let sym = Symbol(id);
        self.lookup.insert(s.to_string(), sym);
        sym
    }

    /// Look up the string a [`Symbol`] was interned from. Panics only
    /// if given a `Symbol` this interner did not itself produce --
    /// callers within this crate never do that, since `Symbol`s are
    /// only ever constructed by [`Interner::intern`].
    pub fn resolve(&self, sym: Symbol) -> &str {
        &self.strings[sym.0 as usize]
    }

    pub fn len(&self) -> usize {
        self.strings.len()
    }

    pub fn is_empty(&self) -> bool {
        self.strings.is_empty()
    }

    /// Intern a symbol guaranteed to be distinct from every symbol
    /// interned so far *and* from anything `vf-lexer` can ever produce
    /// from source text (a lexed identifier never starts with `$`,
    /// since the lexer only starts one on an ASCII letter or `_`).
    /// `vf-typecheck`/`vf-kernel` use this to open a binder's body
    /// under a genuinely fresh local variable without risking capture
    /// of a user-written name.
    pub fn fresh(&mut self) -> Symbol {
        let candidate = format!("$fresh{}", self.strings.len());
        self.intern(&candidate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interning_the_same_string_twice_returns_the_same_symbol() {
        let mut interner = Interner::new();
        let a = interner.intern("foo");
        let b = interner.intern("foo");
        assert_eq!(a, b);
    }

    #[test]
    fn interning_different_strings_returns_different_symbols() {
        let mut interner = Interner::new();
        let a = interner.intern("foo");
        let b = interner.intern("bar");
        assert_ne!(a, b);
    }

    #[test]
    fn resolve_recovers_the_original_string() {
        let mut interner = Interner::new();
        let sym = interner.intern("hello");
        assert_eq!(interner.resolve(sym), "hello");
    }

    #[test]
    fn empty_interner_reports_empty() {
        let interner = Interner::new();
        assert!(interner.is_empty());
        assert_eq!(interner.len(), 0);
    }

    #[test]
    fn interning_many_distinct_strings_grows_len() {
        let mut interner = Interner::new();
        for i in 0..100 {
            interner.intern(&format!("sym_{i}"));
        }
        assert_eq!(interner.len(), 100);
    }

    #[test]
    fn fresh_symbols_are_pairwise_distinct() {
        let mut interner = Interner::new();
        let a = interner.fresh();
        let b = interner.fresh();
        let c = interner.fresh();
        assert_ne!(a, b);
        assert_ne!(b, c);
        assert_ne!(a, c);
    }

    #[test]
    fn fresh_symbol_cannot_collide_with_a_lexed_identifier() {
        // No token the lexer produces from source text starts with `$`,
        // so a fresh symbol's name (which always does) can never alias
        // a user-written identifier interned separately.
        let mut interner = Interner::new();
        let fresh = interner.fresh();
        assert!(interner.resolve(fresh).starts_with('$'));
    }
}
