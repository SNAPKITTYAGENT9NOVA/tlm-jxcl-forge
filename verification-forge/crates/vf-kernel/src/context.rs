// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! The kernel's local typing context: what type each in-scope,
//! already-opened local variable has. See `vf-core`'s crate docs for
//! why local variables are tracked this way (locally-nameless) rather
//! than as a de-Bruijn-indexed list.
use vf_core::{Symbol, TermId};

#[derive(Debug, Clone, Default)]
pub struct Context {
    bindings: Vec<(Symbol, TermId)>,
}

impl Context {
    pub fn new() -> Self {
        Context::default()
    }

    /// A new context with `sym : ty` added on top of `self`, shadowing
    /// any existing binding for `sym`.
    pub fn extended(&self, sym: Symbol, ty: TermId) -> Self {
        let mut bindings = self.bindings.clone();
        bindings.push((sym, ty));
        Context { bindings }
    }

    pub fn lookup(&self, sym: Symbol) -> Option<TermId> {
        self.bindings
            .iter()
            .rev()
            .find(|(s, _)| *s == sym)
            .map(|(_, t)| *t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vf_core::{Interner, Sort, TermArena};

    #[test]
    fn lookup_on_an_empty_context_is_none() {
        let mut interner = Interner::new();
        let sym = interner.intern("x");
        assert_eq!(Context::new().lookup(sym), None);
    }

    #[test]
    fn extended_context_finds_the_new_binding() {
        let mut arena = TermArena::new();
        let mut interner = Interner::new();
        let sym = interner.intern("x");
        let ty = arena.sort(Sort::Type(0));
        let ctx = Context::new().extended(sym, ty);
        assert_eq!(ctx.lookup(sym), Some(ty));
    }

    #[test]
    fn a_shadowing_binding_takes_precedence() {
        let mut arena = TermArena::new();
        let mut interner = Interner::new();
        let sym = interner.intern("x");
        let old_ty = arena.sort(Sort::Type(0));
        let new_ty = arena.sort(Sort::Type(1));
        let ctx = Context::new().extended(sym, old_ty).extended(sym, new_ty);
        assert_eq!(ctx.lookup(sym), Some(new_ty));
    }
}
