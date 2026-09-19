//! Small term-building helpers shared across this crate's declarations
//! -- the same pattern `vf-axioms::prelude` and `vf-elucidian` use,
//! reimplemented locally rather than factored into a shared crate (see
//! this crate's top-level doc comment on why).
use vf_core::{ArenaError, Binder, Interner, TermArena, TermId};

/// `Pi (hint : domain), codomain`, where `codomain` is built from a
/// fresh placeholder standing in for the bound variable.
pub(crate) fn dep_pi(
    arena: &mut TermArena,
    interner: &mut Interner,
    hint: &str,
    domain: TermId,
    build_codomain: impl FnOnce(&mut TermArena, &mut Interner, TermId) -> Result<TermId, ArenaError>,
) -> Result<TermId, ArenaError> {
    let sym = interner.intern(hint);
    let placeholder = arena.free_var(sym);
    let codomain_open = build_codomain(arena, interner, placeholder)?;
    let codomain_closed = arena.close_at(codomain_open, 0, sym)?;
    Ok(arena.pi(Binder(sym), domain, codomain_closed))
}

/// `Pi (hint : domain), codomain` where `codomain` does not depend on
/// the bound variable at all.
pub(crate) fn arrow(
    arena: &mut TermArena,
    interner: &mut Interner,
    domain: TermId,
    codomain: TermId,
) -> TermId {
    arena.pi(Binder(interner.intern("_")), domain, codomain)
}

/// `fun (hint : domain) => body`, where `body` is built from a fresh
/// placeholder standing in for the bound variable.
pub(crate) fn lam1(
    arena: &mut TermArena,
    interner: &mut Interner,
    hint: &str,
    domain: TermId,
    build_body: impl FnOnce(&mut TermArena, &mut Interner, TermId) -> Result<TermId, ArenaError>,
) -> Result<TermId, ArenaError> {
    let sym = interner.intern(hint);
    let placeholder = arena.free_var(sym);
    let body_open = build_body(arena, interner, placeholder)?;
    let body_closed = arena.close_at(body_open, 0, sym)?;
    Ok(arena.lam(Binder(sym), domain, body_closed))
}

/// `f a b`, avoiding the borrow conflict of nested `arena.app(arena.app(..))`.
pub(crate) fn app2(arena: &mut TermArena, f: TermId, a: TermId, b: TermId) -> TermId {
    let fa = arena.app(f, a);
    arena.app(fa, b)
}

/// `f a b c`.
pub(crate) fn app3(arena: &mut TermArena, f: TermId, a: TermId, b: TermId, c: TermId) -> TermId {
    let fab = app2(arena, f, a, b);
    arena.app(fab, c)
}

/// `f a b c d`.
pub(crate) fn app4(
    arena: &mut TermArena,
    f: TermId,
    a: TermId,
    b: TermId,
    c: TermId,
    d: TermId,
) -> TermId {
    let fabc = app3(arena, f, a, b, c);
    arena.app(fabc, d)
}
