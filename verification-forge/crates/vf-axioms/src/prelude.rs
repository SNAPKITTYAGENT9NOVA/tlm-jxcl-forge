// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! The standard prelude: `Nat`, `Bool`, `List`, `Option`, `Either`,
//! `Vector`, and `Fin`, registered as [`crate::Registry::declare_builtin`]/
//! [`crate::Registry::declare_recursor`] entries.
//!
//! There is no general "inductive type declaration" mechanism here --
//! no positivity checker, no automatic derivation of a recursor from a
//! constructor list. Each type's former, constructors, and recursor
//! are individually hand-built `Pi`-chains (checked by the kernel like
//! any other declaration) paired with a hand-written
//! [`vf_reducer::RecursorSpec`] describing its computation rule. That is
//! a real, deliberate scope limit for this small kernel -- the fixed
//! set of seven types below is what "inductive types" means in
//! verification-forge, not a framework for declaring arbitrary new
//! ones. Extending the list means writing another function like the
//! ones here, not user-facing surface syntax.
use crate::error::RegistryError;
use crate::registry::Registry;
use vf_core::{ArenaError, Binder, Interner, Sort, Symbol, TermArena, TermId};
use vf_reducer::{ConstructorSpec, RecursorSpec};

/// Build `Pi (hint : domain), codomain`, where `codomain` is produced
/// by `build_codomain` from a fresh placeholder standing in for the
/// bound variable -- see `vf-core::Interner::fresh`'s doc comment for
/// why a `$`-prefixed fresh symbol can never collide with anything a
/// user could have written, though here we use a readable `hint`
/// instead purely for `Debug` output, since binder names never affect
/// term identity (see `vf-core::term::Binder`'s doc comment).
fn dep_pi(
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
/// the bound variable at all -- the common non-dependent-arrow case,
/// which needs no placeholder/close_at bookkeeping since there is
/// nothing to close.
fn arrow(
    arena: &mut TermArena,
    interner: &mut Interner,
    hint: &str,
    domain: TermId,
    codomain: TermId,
) -> TermId {
    arena.pi(Binder(interner.intern(hint)), domain, codomain)
}

/// `f a b` (curried application to two arguments). Plain nested
/// `arena.app(arena.app(f, a), b)` doesn't borrow-check (two
/// simultaneous `&mut TermArena` borrows), so every multi-argument
/// application in this module goes through one of these instead.
fn app2(arena: &mut TermArena, f: TermId, a: TermId, b: TermId) -> TermId {
    let fa = arena.app(f, a);
    arena.app(fa, b)
}

/// `f a b c`.
fn app3(arena: &mut TermArena, f: TermId, a: TermId, b: TermId, c: TermId) -> TermId {
    let fab = app2(arena, f, a, b);
    arena.app(fab, c)
}

/// `f a b c d`.
fn app4(arena: &mut TermArena, f: TermId, a: TermId, b: TermId, c: TermId, d: TermId) -> TermId {
    let fabc = app3(arena, f, a, b, c);
    arena.app(fabc, d)
}

/// Every symbol the prelude registers, grouped by type. Each type's
/// symbols are exactly: the type former, its constructors (in
/// declaration/recursor-case order), and its recursor.
#[derive(Debug, Clone, Copy)]
pub struct Prelude {
    pub refl: Symbol,
    pub ap: Symbol,

    pub nat: Symbol,
    pub zero: Symbol,
    pub succ: Symbol,
    pub nat_rec: Symbol,
    /// Same computation, but eliminating into `Prop`; see `Bool_ind`'s
    /// doc comment for why both exist.
    pub nat_ind: Symbol,

    pub bool_: Symbol,
    pub true_: Symbol,
    pub false_: Symbol,
    pub bool_rec: Symbol,
    /// Same computation, but eliminating into `Prop`; see this
    /// function's doc comment on why both exist.
    pub bool_ind: Symbol,

    pub list: Symbol,
    pub nil: Symbol,
    pub cons: Symbol,
    pub list_rec: Symbol,

    pub option: Symbol,
    pub none: Symbol,
    pub some: Symbol,
    pub option_rec: Symbol,

    pub either: Symbol,
    pub inl: Symbol,
    pub inr: Symbol,
    pub either_rec: Symbol,

    pub vector: Symbol,
    pub vnil: Symbol,
    pub vcons: Symbol,
    pub vector_rec: Symbol,

    pub fin: Symbol,
    pub fin_zero: Symbol,
    pub fin_succ: Symbol,
    pub fin_rec: Symbol,
}

/// Register the full standard prelude into `registry`, in dependency
/// order (each type's constructors after its former, its recursor
/// last). Fails closed exactly like any other declaration: if any
/// piece is rejected by the kernel, no partial prelude is left behind
/// half-registered silently succeeding for later use -- the error
/// propagates and registration as a whole did not happen.
#[allow(clippy::too_many_lines)]
pub fn register_prelude(
    registry: &mut Registry,
    arena: &mut TermArena,
    interner: &mut Interner,
    fuel: &mut u64,
) -> Result<Prelude, RegistryError> {
    let type0 = arena.sort(Sort::Type(0));

    // ---- Eq (the equality proposition's introduction rule) ----
    // `Term::Eq` (vf-core) is only the proposition *former* -- nothing
    // in the kernel so far can actually construct a proof of one.
    // `refl` is that missing piece: the standard (Lean/Coq-style)
    // primitive that every `x : A` trivially equals itself. Like a
    // constructor, it's opaque/irreducible and its soundness rests on
    // the type theory's metatheory, not on anything the kernel can
    // bootstrap-check -- exactly the same trust position as `zero`
    // or `nil`.
    let refl = interner.intern("refl");
    let refl_ty = dep_pi(arena, interner, "A", type0, |arena, interner, a| {
        dep_pi(arena, interner, "x", a, move |arena, _interner, x| {
            Ok(arena.eq(a, x, x))
        })
    })?;
    registry.declare_builtin(arena, interner, refl, refl_ty, fuel)?;

    // ---- ap (congruence: Eq is a congruence for function application) ----
    // Without a full equality eliminator (the "J rule"), from which
    // congruence/symmetry/transitivity would all be *derivable*, this
    // kernel needs the specific fact it actually uses -- that
    // `f x = f y` whenever `x = y` -- as its own primitive. Standard
    // (`Coq`'s `f_equal`, `Lean`'s `congrArg`) and, like `refl` itself,
    // opaque and never reduced: its soundness rests on the same
    // metatheory `refl`'s does. Adding a general `Eq` eliminator is
    // future work, not something this specific proof needs.
    let ap = interner.intern("ap");
    let ap_ty = dep_pi(arena, interner, "A", type0, |arena, interner, a| {
        dep_pi(arena, interner, "B", type0, move |arena, interner, b| {
            let a_to_b = arrow(arena, interner, "_", a, b);
            dep_pi(arena, interner, "f", a_to_b, move |arena, interner, f| {
                dep_pi(arena, interner, "x", a, move |arena, interner, x| {
                    dep_pi(arena, interner, "y", a, move |arena, interner, y| {
                        let eq_xy = arena.eq(a, x, y);
                        dep_pi(arena, interner, "_", eq_xy, move |arena, _interner, _h| {
                            let fx = arena.app(f, x);
                            let fy = arena.app(f, y);
                            Ok(arena.eq(b, fx, fy))
                        })
                    })
                })
            })
        })
    })?;
    registry.declare_builtin(arena, interner, ap, ap_ty, fuel)?;

    // ---- Nat ----
    let nat = interner.intern("Nat");
    registry.declare_builtin(arena, interner, nat, type0, fuel)?;
    let nat_c = arena.const_(nat);

    let zero = interner.intern("zero");
    registry.declare_builtin(arena, interner, zero, nat_c, fuel)?;
    let zero_c = arena.const_(zero);

    let succ = interner.intern("succ");
    let succ_ty = arrow(arena, interner, "_", nat_c, nat_c);
    registry.declare_builtin(arena, interner, succ, succ_ty, fuel)?;
    let succ_c = arena.const_(succ);

    let nat_rec = interner.intern("Nat_rec");
    let nat_motive_ty = arrow(arena, interner, "_", nat_c, type0);
    let nat_rec_ty = dep_pi(arena, interner, "P", nat_motive_ty, |arena, interner, p| {
        let p_zero = arena.app(p, zero_c);
        dep_pi(arena, interner, "pz", p_zero, |arena, interner, _pz| {
            let step_ty = dep_pi(arena, interner, "n", nat_c, |arena, interner, n| {
                let p_n = arena.app(p, n);
                let succ_n = arena.app(succ_c, n);
                let p_succ_n = arena.app(p, succ_n);
                dep_pi(arena, interner, "ih", p_n, move |_arena, _interner, _ih| {
                    Ok(p_succ_n)
                })
            })?;
            dep_pi(
                arena,
                interner,
                "ps",
                step_ty,
                move |arena, interner, _ps| {
                    dep_pi(arena, interner, "n", nat_c, move |arena, _interner, n| {
                        Ok(arena.app(p, n))
                    })
                },
            )
        })
    })?;
    let nat_rec_spec = RecursorSpec {
        arity: 4,
        case_positions: vec![1, 2],
        constructor_names: vec![zero, succ],
        constructors: vec![
            ConstructorSpec {
                skip: 0,
                recursive_args: vec![],
            },
            ConstructorSpec {
                skip: 0,
                recursive_args: vec![true],
            },
        ],
    };
    registry.declare_recursor(
        arena,
        interner,
        nat_rec,
        nat_rec_ty,
        nat_rec_spec.clone(),
        fuel,
    )?;

    // ---- Nat_ind: the same eliminator, but into Prop instead of Type 0 ----
    // See `Bool_ind`'s doc comment below for why this kernel needs a
    // separate eliminator per target sort rather than one
    // universe-polymorphic recursor.
    let nat_ind = interner.intern("Nat_ind");
    let prop = arena.sort(Sort::Prop);
    let nat_ind_motive_ty = arrow(arena, interner, "_", nat_c, prop);
    let nat_ind_ty = dep_pi(
        arena,
        interner,
        "P",
        nat_ind_motive_ty,
        |arena, interner, p| {
            let p_zero = arena.app(p, zero_c);
            dep_pi(arena, interner, "pz", p_zero, |arena, interner, _pz| {
                let step_ty = dep_pi(arena, interner, "n", nat_c, |arena, interner, n| {
                    let p_n = arena.app(p, n);
                    let succ_n = arena.app(succ_c, n);
                    let p_succ_n = arena.app(p, succ_n);
                    dep_pi(arena, interner, "ih", p_n, move |_arena, _interner, _ih| {
                        Ok(p_succ_n)
                    })
                })?;
                dep_pi(
                    arena,
                    interner,
                    "ps",
                    step_ty,
                    move |arena, interner, _ps| {
                        dep_pi(arena, interner, "n", nat_c, move |arena, _interner, n| {
                            Ok(arena.app(p, n))
                        })
                    },
                )
            })
        },
    )?;
    registry.declare_recursor(arena, interner, nat_ind, nat_ind_ty, nat_rec_spec, fuel)?;

    // ---- Bool ----
    let bool_ = interner.intern("Bool");
    registry.declare_builtin(arena, interner, bool_, type0, fuel)?;
    let bool_c = arena.const_(bool_);

    let true_ = interner.intern("true_");
    registry.declare_builtin(arena, interner, true_, bool_c, fuel)?;
    let true_c = arena.const_(true_);

    let false_ = interner.intern("false_");
    registry.declare_builtin(arena, interner, false_, bool_c, fuel)?;
    let false_c = arena.const_(false_);

    let bool_rec = interner.intern("Bool_rec");
    let bool_motive_ty = arrow(arena, interner, "_", bool_c, type0);
    let bool_rec_ty = dep_pi(
        arena,
        interner,
        "P",
        bool_motive_ty,
        |arena, interner, p| {
            let p_true = arena.app(p, true_c);
            dep_pi(
                arena,
                interner,
                "pt",
                p_true,
                move |arena, interner, _pt| {
                    let p_false = arena.app(p, false_c);
                    dep_pi(
                        arena,
                        interner,
                        "pf",
                        p_false,
                        move |arena, interner, _pf| {
                            dep_pi(arena, interner, "b", bool_c, move |arena, _interner, b| {
                                Ok(arena.app(p, b))
                            })
                        },
                    )
                },
            )
        },
    )?;
    let bool_rec_spec = RecursorSpec {
        arity: 4,
        case_positions: vec![1, 2],
        constructor_names: vec![true_, false_],
        constructors: vec![
            ConstructorSpec {
                skip: 0,
                recursive_args: vec![],
            },
            ConstructorSpec {
                skip: 0,
                recursive_args: vec![],
            },
        ],
    };
    registry.declare_recursor(
        arena,
        interner,
        bool_rec,
        bool_rec_ty,
        bool_rec_spec.clone(),
        fuel,
    )?;

    // ---- Bool_ind: the same eliminator, but into Prop instead of Type 0 ----
    // This kernel has no universe polymorphism (see vf-core's crate
    // docs), so one recursor's type cannot be generic over which sort
    // its motive returns into: `Bool_rec` above can only produce
    // *data* (a `Bool -> Type 0` motive), never prove a *proposition*
    // (a `Bool -> Prop` motive) about an abstract `Bool`, since `Prop`
    // and `Type 0` are unrelated sorts here (no cumulativity between
    // them). `Bool_ind` is the same computation rule (identical
    // RecursorSpec -- iota reduction only cares about term shape, not
    // which sort a motive targets) registered again under its own
    // symbol with a Prop-valued motive, purely so it can be used to
    // prove things by case-splitting on a `Bool`. See `vf-elucidian`
    // for a real use of this (the `reflect` involution proof).
    let bool_ind = interner.intern("Bool_ind");
    let prop = arena.sort(Sort::Prop);
    let bool_ind_motive_ty = arrow(arena, interner, "_", bool_c, prop);
    let bool_ind_ty = dep_pi(
        arena,
        interner,
        "P",
        bool_ind_motive_ty,
        |arena, interner, p| {
            let p_true = arena.app(p, true_c);
            dep_pi(
                arena,
                interner,
                "pt",
                p_true,
                move |arena, interner, _pt| {
                    let p_false = arena.app(p, false_c);
                    dep_pi(
                        arena,
                        interner,
                        "pf",
                        p_false,
                        move |arena, interner, _pf| {
                            dep_pi(arena, interner, "b", bool_c, move |arena, _interner, b| {
                                Ok(arena.app(p, b))
                            })
                        },
                    )
                },
            )
        },
    )?;
    registry.declare_recursor(arena, interner, bool_ind, bool_ind_ty, bool_rec_spec, fuel)?;

    // ---- List ----
    let list = interner.intern("List");
    let list_ty = arrow(arena, interner, "_", type0, type0);
    registry.declare_builtin(arena, interner, list, list_ty, fuel)?;
    let list_c = arena.const_(list);

    let nil = interner.intern("nil");
    let nil_ty = dep_pi(arena, interner, "A", type0, move |arena, _interner, a| {
        Ok(arena.app(list_c, a))
    })?;
    registry.declare_builtin(arena, interner, nil, nil_ty, fuel)?;
    let nil_c = arena.const_(nil);

    let cons = interner.intern("cons");
    let cons_ty = dep_pi(arena, interner, "A", type0, move |arena, interner, a| {
        let list_a = arena.app(list_c, a);
        let inner = arrow(arena, interner, "_", list_a, list_a);
        Ok(arrow(arena, interner, "_", a, inner))
    })?;
    registry.declare_builtin(arena, interner, cons, cons_ty, fuel)?;
    let cons_c = arena.const_(cons);

    let list_rec = interner.intern("List_rec");
    let list_rec_ty = dep_pi(arena, interner, "A", type0, move |arena, interner, a| {
        let list_a = arena.app(list_c, a);
        let nil_a = arena.app(nil_c, a);
        let motive_ty = arrow(arena, interner, "_", list_a, type0);
        dep_pi(
            arena,
            interner,
            "P",
            motive_ty,
            move |arena, interner, p| {
                let p_nil = arena.app(p, nil_a);
                dep_pi(arena, interner, "pn", p_nil, move |arena, interner, _pn| {
                    let step_ty = dep_pi(arena, interner, "x", a, move |arena, interner, x| {
                        dep_pi(arena, interner, "xs", list_a, move |arena, interner, xs| {
                            let p_xs = arena.app(p, xs);
                            let cons_a_x_xs = app3(arena, cons_c, a, x, xs);
                            let p_cons = arena.app(p, cons_a_x_xs);
                            dep_pi(
                                arena,
                                interner,
                                "ih",
                                p_xs,
                                move |_arena, _interner, _ih| Ok(p_cons),
                            )
                        })
                    })?;
                    dep_pi(
                        arena,
                        interner,
                        "pc",
                        step_ty,
                        move |arena, interner, _pc| {
                            dep_pi(arena, interner, "l", list_a, move |arena, _interner, l| {
                                Ok(arena.app(p, l))
                            })
                        },
                    )
                })
            },
        )
    })?;
    let list_rec_spec = RecursorSpec {
        arity: 5, // A, P, pn, pc, l
        case_positions: vec![2, 3],
        constructor_names: vec![nil, cons],
        constructors: vec![
            // nil/cons's own application spine includes the leading
            // type parameter `A` (`nil A`, `cons A x xs`), but `A` is
            // a uniform parameter the recursor's own type only binds
            // once (never re-abstracted per case) -- skip it, don't
            // feed it to pn/pc.
            ConstructorSpec {
                skip: 1,
                recursive_args: vec![],
            }, // nil A
            ConstructorSpec {
                skip: 1,
                recursive_args: vec![false, true],
            }, // cons A x xs: xs rec
        ],
    };
    registry.declare_recursor(arena, interner, list_rec, list_rec_ty, list_rec_spec, fuel)?;

    // ---- Option ----
    let option = interner.intern("Option");
    let option_ty = arrow(arena, interner, "_", type0, type0);
    registry.declare_builtin(arena, interner, option, option_ty, fuel)?;
    let option_c = arena.const_(option);

    let none = interner.intern("none");
    let none_ty = dep_pi(arena, interner, "A", type0, move |arena, _interner, a| {
        Ok(arena.app(option_c, a))
    })?;
    registry.declare_builtin(arena, interner, none, none_ty, fuel)?;
    let none_c = arena.const_(none);

    let some = interner.intern("some");
    let some_ty = dep_pi(arena, interner, "A", type0, move |arena, interner, a| {
        let option_a = arena.app(option_c, a);
        Ok(arrow(arena, interner, "_", a, option_a))
    })?;
    registry.declare_builtin(arena, interner, some, some_ty, fuel)?;
    let some_c = arena.const_(some);

    let option_rec = interner.intern("Option_rec");
    let option_rec_ty = dep_pi(arena, interner, "A", type0, move |arena, interner, a| {
        let option_a = arena.app(option_c, a);
        let none_a = arena.app(none_c, a);
        let motive_ty = arrow(arena, interner, "_", option_a, type0);
        dep_pi(
            arena,
            interner,
            "P",
            motive_ty,
            move |arena, interner, p| {
                let p_none = arena.app(p, none_a);
                dep_pi(
                    arena,
                    interner,
                    "pn",
                    p_none,
                    move |arena, interner, _pn| {
                        let step_ty =
                            dep_pi(arena, interner, "x", a, move |arena, _interner, x| {
                                let some_a_x = app2(arena, some_c, a, x);
                                Ok(arena.app(p, some_a_x))
                            })?;
                        dep_pi(
                            arena,
                            interner,
                            "ps",
                            step_ty,
                            move |arena, interner, _ps| {
                                dep_pi(
                                    arena,
                                    interner,
                                    "o",
                                    option_a,
                                    move |arena, _interner, o| Ok(arena.app(p, o)),
                                )
                            },
                        )
                    },
                )
            },
        )
    })?;
    let option_rec_spec = RecursorSpec {
        arity: 5, // A, P, pn, ps, o(major) -- A IS part of the actual application spine (Option_rec is called as `Option_rec A P pn ps o`), so it counts here just like List_rec's leading A does.
        case_positions: vec![2, 3],
        constructor_names: vec![none, some],
        constructors: vec![
            ConstructorSpec {
                skip: 1,
                recursive_args: vec![],
            }, // none A
            ConstructorSpec {
                skip: 1,
                recursive_args: vec![false],
            }, // some A x
        ],
    };
    registry.declare_recursor(
        arena,
        interner,
        option_rec,
        option_rec_ty,
        option_rec_spec,
        fuel,
    )?;

    // ---- Either ----
    let either = interner.intern("Either");
    let either_ty = dep_pi(arena, interner, "A", type0, move |arena, interner, _a| {
        Ok(arrow(arena, interner, "_", type0, type0))
    })?;
    registry.declare_builtin(arena, interner, either, either_ty, fuel)?;
    let either_c = arena.const_(either);

    let inl = interner.intern("inl");
    let inl_ty = dep_pi(arena, interner, "A", type0, move |arena, interner, a| {
        dep_pi(arena, interner, "B", type0, move |arena, interner, b| {
            let either_ab = app2(arena, either_c, a, b);
            Ok(arrow(arena, interner, "_", a, either_ab))
        })
    })?;
    registry.declare_builtin(arena, interner, inl, inl_ty, fuel)?;
    let inl_c = arena.const_(inl);

    let inr = interner.intern("inr");
    let inr_ty = dep_pi(arena, interner, "A", type0, move |arena, interner, a| {
        dep_pi(arena, interner, "B", type0, move |arena, interner, b| {
            let either_ab = app2(arena, either_c, a, b);
            Ok(arrow(arena, interner, "_", b, either_ab))
        })
    })?;
    registry.declare_builtin(arena, interner, inr, inr_ty, fuel)?;
    let inr_c = arena.const_(inr);

    let either_rec = interner.intern("Either_rec");
    let either_rec_ty = dep_pi(arena, interner, "A", type0, move |arena, interner, a| {
        dep_pi(arena, interner, "B", type0, move |arena, interner, b| {
            let either_ab = app2(arena, either_c, a, b);
            let motive_ty = arrow(arena, interner, "_", either_ab, type0);
            dep_pi(
                arena,
                interner,
                "P",
                motive_ty,
                move |arena, interner, p| {
                    let left_ty = dep_pi(arena, interner, "x", a, move |arena, _interner, x| {
                        let inl_x = app3(arena, inl_c, a, b, x);
                        Ok(arena.app(p, inl_x))
                    })?;
                    dep_pi(
                        arena,
                        interner,
                        "pl",
                        left_ty,
                        move |arena, interner, _pl| {
                            let right_ty =
                                dep_pi(arena, interner, "y", b, move |arena, _interner, y| {
                                    let inr_y = app3(arena, inr_c, a, b, y);
                                    Ok(arena.app(p, inr_y))
                                })?;
                            dep_pi(
                                arena,
                                interner,
                                "pr",
                                right_ty,
                                move |arena, interner, _pr| {
                                    dep_pi(
                                        arena,
                                        interner,
                                        "e",
                                        either_ab,
                                        move |arena, _interner, e| Ok(arena.app(p, e)),
                                    )
                                },
                            )
                        },
                    )
                },
            )
        })
    })?;
    let either_rec_spec = RecursorSpec {
        arity: 6, // A, B, P, pl, pr, e
        case_positions: vec![3, 4],
        constructor_names: vec![inl, inr],
        constructors: vec![
            ConstructorSpec {
                skip: 2,
                recursive_args: vec![false],
            }, // inl A B x
            ConstructorSpec {
                skip: 2,
                recursive_args: vec![false],
            }, // inr A B y
        ],
    };
    registry.declare_recursor(
        arena,
        interner,
        either_rec,
        either_rec_ty,
        either_rec_spec,
        fuel,
    )?;

    // ---- Vector (indexed by length : Nat) ----
    let vector = interner.intern("Vector");
    let vector_ty = dep_pi(arena, interner, "A", type0, move |arena, interner, _a| {
        Ok(arrow(arena, interner, "_", nat_c, type0))
    })?;
    registry.declare_builtin(arena, interner, vector, vector_ty, fuel)?;
    let vector_c = arena.const_(vector);

    let vnil = interner.intern("vnil");
    let vnil_ty = dep_pi(arena, interner, "A", type0, move |arena, _interner, a| {
        Ok(app2(arena, vector_c, a, zero_c))
    })?;
    registry.declare_builtin(arena, interner, vnil, vnil_ty, fuel)?;
    let vnil_c = arena.const_(vnil);

    let vcons = interner.intern("vcons");
    let vcons_ty = dep_pi(arena, interner, "A", type0, move |arena, interner, a| {
        dep_pi(arena, interner, "k", nat_c, move |arena, interner, k| {
            let vec_a_k = app2(arena, vector_c, a, k);
            let succ_k = arena.app(succ_c, k);
            let vec_a_succ_k = app2(arena, vector_c, a, succ_k);
            let tail_to_result = arrow(arena, interner, "_", vec_a_k, vec_a_succ_k);
            let head_to_rest = arrow(arena, interner, "_", a, tail_to_result);
            Ok(head_to_rest)
        })
    })?;
    registry.declare_builtin(arena, interner, vcons, vcons_ty, fuel)?;
    let vcons_c = arena.const_(vcons);

    let vector_rec = interner.intern("Vector_rec");
    let vector_rec_ty = dep_pi(arena, interner, "A", type0, move |arena, interner, a| {
        // P : Pi (k : Nat), Vector A k -> Type 0
        let motive_ty = dep_pi(arena, interner, "k", nat_c, move |arena, interner, k| {
            let vec_a_k = app2(arena, vector_c, a, k);
            Ok(arrow(arena, interner, "_", vec_a_k, type0))
        })?;
        dep_pi(
            arena,
            interner,
            "P",
            motive_ty,
            move |arena, interner, p| {
                let vnil_a = arena.app(vnil_c, a);
                let p_zero_vnil = app2(arena, p, zero_c, vnil_a);
                dep_pi(
                    arena,
                    interner,
                    "pn",
                    p_zero_vnil,
                    move |arena, interner, _pn| {
                        // step : Pi (k:Nat) (x:A) (xs : Vector A k), P k xs -> P (succ k) (vcons A k x xs)
                        let step_ty =
                            dep_pi(arena, interner, "k", nat_c, move |arena, interner, k| {
                                dep_pi(arena, interner, "x", a, move |arena, interner, x| {
                                    let vec_a_k = app2(arena, vector_c, a, k);
                                    dep_pi(
                                        arena,
                                        interner,
                                        "xs",
                                        vec_a_k,
                                        move |arena, interner, xs| {
                                            let p_k_xs = app2(arena, p, k, xs);
                                            let succ_k = arena.app(succ_c, k);
                                            let vcons_a_k_x_xs = app4(arena, vcons_c, a, k, x, xs);
                                            let p_succk_vcons =
                                                app2(arena, p, succ_k, vcons_a_k_x_xs);
                                            dep_pi(
                                                arena,
                                                interner,
                                                "ih",
                                                p_k_xs,
                                                move |_arena, _interner, _ih| Ok(p_succk_vcons),
                                            )
                                        },
                                    )
                                })
                            })?;
                        dep_pi(
                            arena,
                            interner,
                            "ps",
                            step_ty,
                            move |arena, interner, _ps| {
                                dep_pi(arena, interner, "k", nat_c, move |arena, interner, k| {
                                    let vec_a_k = app2(arena, vector_c, a, k);
                                    dep_pi(
                                        arena,
                                        interner,
                                        "v",
                                        vec_a_k,
                                        move |arena, _interner, v| Ok(app2(arena, p, k, v)),
                                    )
                                })
                            },
                        )
                    },
                )
            },
        )
    })?;
    let vector_rec_spec = RecursorSpec {
        arity: 6, // A, P, pn, ps, k, v(major)
        case_positions: vec![2, 3],
        constructor_names: vec![vnil, vcons],
        constructors: vec![
            ConstructorSpec {
                skip: 1,
                recursive_args: vec![],
            }, // vnil A
            ConstructorSpec {
                skip: 1,
                recursive_args: vec![false, false, true],
            }, // vcons A k x xs: xs is recursive (k is an index, not skipped)
        ],
    };
    registry.declare_recursor(
        arena,
        interner,
        vector_rec,
        vector_rec_ty,
        vector_rec_spec,
        fuel,
    )?;

    // ---- Fin (indexed by bound : Nat) ----
    let fin = interner.intern("Fin");
    let fin_ty = arrow(arena, interner, "_", nat_c, type0);
    registry.declare_builtin(arena, interner, fin, fin_ty, fuel)?;
    let fin_c = arena.const_(fin);

    let fin_zero = interner.intern("fin_zero");
    let fin_zero_ty = dep_pi(arena, interner, "n", nat_c, move |arena, _interner, n| {
        let succ_n = arena.app(succ_c, n);
        Ok(arena.app(fin_c, succ_n))
    })?;
    registry.declare_builtin(arena, interner, fin_zero, fin_zero_ty, fuel)?;
    let fin_zero_c = arena.const_(fin_zero);

    let fin_succ = interner.intern("fin_succ");
    let fin_succ_ty = dep_pi(arena, interner, "n", nat_c, move |arena, interner, n| {
        let fin_n = arena.app(fin_c, n);
        let succ_n = arena.app(succ_c, n);
        let fin_succ_n = arena.app(fin_c, succ_n);
        Ok(arrow(arena, interner, "_", fin_n, fin_succ_n))
    })?;
    registry.declare_builtin(arena, interner, fin_succ, fin_succ_ty, fuel)?;
    let fin_succ_c = arena.const_(fin_succ);

    let fin_rec = interner.intern("Fin_rec");
    // P : Pi (n : Nat), Fin n -> Type 0
    let fin_motive_ty = dep_pi(arena, interner, "n", nat_c, move |arena, interner, n| {
        let fin_n = arena.app(fin_c, n);
        Ok(arrow(arena, interner, "_", fin_n, type0))
    })?;
    let fin_rec_ty = dep_pi(
        arena,
        interner,
        "P",
        fin_motive_ty,
        move |arena, interner, p| {
            // base : Pi (n:Nat), P (succ n) (fin_zero n)
            let base_ty = dep_pi(arena, interner, "n", nat_c, move |arena, _interner, n| {
                let succ_n = arena.app(succ_c, n);
                let fin_zero_n = arena.app(fin_zero_c, n);
                Ok(app2(arena, p, succ_n, fin_zero_n))
            })?;
            dep_pi(
                arena,
                interner,
                "pz",
                base_ty,
                move |arena, interner, _pz| {
                    // step : Pi (n:Nat) (i:Fin n), P n i -> P (succ n) (fin_succ n i)
                    let step_ty =
                        dep_pi(arena, interner, "n", nat_c, move |arena, interner, n| {
                            let fin_n = arena.app(fin_c, n);
                            dep_pi(arena, interner, "i", fin_n, move |arena, interner, i| {
                                let p_n_i = app2(arena, p, n, i);
                                let succ_n = arena.app(succ_c, n);
                                let fin_succ_n_i = app2(arena, fin_succ_c, n, i);
                                let p_succn_finsucc = app2(arena, p, succ_n, fin_succ_n_i);
                                dep_pi(
                                    arena,
                                    interner,
                                    "ih",
                                    p_n_i,
                                    move |_arena, _interner, _ih| Ok(p_succn_finsucc),
                                )
                            })
                        })?;
                    dep_pi(
                        arena,
                        interner,
                        "ps",
                        step_ty,
                        move |arena, interner, _ps| {
                            dep_pi(arena, interner, "n", nat_c, move |arena, interner, n| {
                                let fin_n = arena.app(fin_c, n);
                                dep_pi(arena, interner, "i", fin_n, move |arena, _interner, i| {
                                    Ok(app2(arena, p, n, i))
                                })
                            })
                        },
                    )
                },
            )
        },
    )?;
    let fin_rec_spec = RecursorSpec {
        arity: 5, // P, pz, ps, n, i(major)
        case_positions: vec![1, 2],
        constructor_names: vec![fin_zero, fin_succ],
        constructors: vec![
            ConstructorSpec {
                skip: 0,
                recursive_args: vec![false],
            }, // fin_zero n: n non-recursive (it's Nat, not Fin)
            ConstructorSpec {
                skip: 0,
                recursive_args: vec![false, true],
            }, // fin_succ n i: n non-rec, i recursive
        ],
    };
    registry.declare_recursor(arena, interner, fin_rec, fin_rec_ty, fin_rec_spec, fuel)?;

    Ok(Prelude {
        refl,
        ap,
        nat,
        zero,
        succ,
        nat_rec,
        nat_ind,
        bool_,
        true_,
        false_,
        bool_rec,
        bool_ind,
        list,
        nil,
        cons,
        list_rec,
        option,
        none,
        some,
        option_rec,
        either,
        inl,
        inr,
        either_rec,
        vector,
        vnil,
        vcons,
        vector_rec,
        fin,
        fin_zero,
        fin_succ,
        fin_rec,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AxiomPolicy;
    use vf_reducer::DEFAULT_FUEL;

    fn setup() -> (TermArena, Interner, Registry, u64) {
        (
            TermArena::new(),
            Interner::new(),
            Registry::new(AxiomPolicy::NoAxioms),
            DEFAULT_FUEL,
        )
    }

    fn numeral(arena: &mut TermArena, p: &Prelude, n: u32) -> TermId {
        let mut t = arena.const_(p.zero);
        let succ_c = arena.const_(p.succ);
        for _ in 0..n {
            t = arena.app(succ_c, t);
        }
        t
    }

    #[test]
    fn the_whole_prelude_registers_successfully() {
        let (mut arena, mut interner, mut registry, mut fuel) = setup();
        let p = register_prelude(&mut registry, &mut arena, &mut interner, &mut fuel).unwrap();
        for name in [
            p.refl,
            p.ap,
            p.nat,
            p.zero,
            p.succ,
            p.nat_rec,
            p.nat_ind,
            p.bool_,
            p.true_,
            p.false_,
            p.bool_rec,
            p.bool_ind,
            p.list,
            p.nil,
            p.cons,
            p.list_rec,
            p.option,
            p.none,
            p.some,
            p.option_rec,
            p.either,
            p.inl,
            p.inr,
            p.either_rec,
            p.vector,
            p.vnil,
            p.vcons,
            p.vector_rec,
            p.fin,
            p.fin_zero,
            p.fin_succ,
            p.fin_rec,
        ] {
            assert!(registry.is_declared(name));
            assert!(
                !registry.is_axiom(name),
                "a builtin must never be counted as a user axiom"
            );
        }
    }

    #[test]
    fn nat_rec_computes_addition_by_genuine_primitive_recursion() {
        let (mut arena, mut interner, mut registry, mut fuel) = setup();
        let p = register_prelude(&mut registry, &mut arena, &mut interner, &mut fuel).unwrap();

        let two = numeral(&mut arena, &p, 2);
        let three = numeral(&mut arena, &p, 3);
        let five = numeral(&mut arena, &p, 5);

        let nat_c = arena.const_(p.nat);
        let motive = {
            let dummy = Binder(interner.intern("_"));
            arena.lam(dummy, nat_c, nat_c) // fun (_ : Nat) => Nat; body doesn't use the bound var
        };
        let succ_c = arena.const_(p.succ);
        let case_s = {
            let ih = arena.bound_var(0);
            let inner_body = arena.app(succ_c, ih);
            let inner_lam = arena.lam(Binder(interner.intern("ih")), nat_c, inner_body);
            arena.lam(Binder(interner.intern("_pred")), nat_c, inner_lam)
        };
        let rec_c = arena.const_(p.nat_rec);
        // add m n := Nat_rec motive n case_s m
        let add_2_3 = app4(&mut arena, rec_c, motive, three, case_s, two);

        assert!(
            vf_kernel::definitional_equal(&mut arena, &registry, add_2_3, five, &mut fuel).unwrap()
        );
    }

    #[test]
    fn bool_rec_selects_the_matching_case() {
        let (mut arena, mut interner, mut registry, mut fuel) = setup();
        let p = register_prelude(&mut registry, &mut arena, &mut interner, &mut fuel).unwrap();

        let nat_c = arena.const_(p.nat);
        let bool_c = arena.const_(p.bool_);
        let motive = {
            let dummy = Binder(interner.intern("_"));
            arena.lam(dummy, bool_c, nat_c)
        };
        let one = numeral(&mut arena, &p, 1);
        let two = numeral(&mut arena, &p, 2);
        let rec_c = arena.const_(p.bool_rec);
        let true_c = arena.const_(p.true_);
        let false_c = arena.const_(p.false_);

        let if_true = app4(&mut arena, rec_c, motive, one, two, true_c);
        assert!(
            vf_kernel::definitional_equal(&mut arena, &registry, if_true, one, &mut fuel).unwrap()
        );

        let if_false = app4(&mut arena, rec_c, motive, one, two, false_c);
        assert!(
            vf_kernel::definitional_equal(&mut arena, &registry, if_false, two, &mut fuel).unwrap()
        );
    }

    #[test]
    fn list_rec_computes_length() {
        let (mut arena, mut interner, mut registry, mut fuel) = setup();
        let p = register_prelude(&mut registry, &mut arena, &mut interner, &mut fuel).unwrap();

        let nat_c = arena.const_(p.nat);
        let bool_c = arena.const_(p.bool_);
        let list_c = arena.const_(p.list);
        let nil_c = arena.const_(p.nil);
        let cons_c = arena.const_(p.cons);
        let list_rec_c = arena.const_(p.list_rec);
        let succ_c = arena.const_(p.succ);
        let true_c = arena.const_(p.true_);
        let false_c = arena.const_(p.false_);

        // xs := cons Bool true (cons Bool false (nil Bool)) : List Bool, length 2.
        let nil_bool = arena.app(nil_c, bool_c);
        let cons_false = app3(&mut arena, cons_c, bool_c, false_c, nil_bool);
        let xs = app3(&mut arena, cons_c, bool_c, true_c, cons_false);

        // length := List_rec Bool (fun _ => Nat) zero (fun _ _ ih => succ ih) xs
        let list_bool = arena.app(list_c, bool_c);
        let motive = {
            let dummy = Binder(interner.intern("_"));
            arena.lam(dummy, list_bool, nat_c)
        };
        let zero_c = arena.const_(p.zero);
        let case_cons = {
            // fun (_:Bool) (_:List Bool) (ih:Nat) => succ ih
            let ih = arena.bound_var(0);
            let succ_ih = arena.app(succ_c, ih);
            let l3 = arena.lam(Binder(interner.intern("ih")), nat_c, succ_ih);
            let l2 = arena.lam(Binder(interner.intern("_xs")), list_bool, l3);
            arena.lam(Binder(interner.intern("_x")), bool_c, l2)
        };
        let length_xs = {
            let t = arena.app(list_rec_c, bool_c);
            let t = arena.app(t, motive);
            let t = arena.app(t, zero_c);
            let t = arena.app(t, case_cons);
            arena.app(t, xs)
        };
        let two = numeral(&mut arena, &p, 2);
        assert!(
            vf_kernel::definitional_equal(&mut arena, &registry, length_xs, two, &mut fuel)
                .unwrap()
        );
    }

    #[test]
    fn option_rec_selects_the_matching_case() {
        let (mut arena, mut interner, mut registry, mut fuel) = setup();
        let p = register_prelude(&mut registry, &mut arena, &mut interner, &mut fuel).unwrap();

        let nat_c = arena.const_(p.nat);
        let option_c = arena.const_(p.option);
        let option_nat = arena.app(option_c, nat_c);
        let motive = {
            let dummy = Binder(interner.intern("_"));
            arena.lam(dummy, option_nat, nat_c)
        };
        let zero = numeral(&mut arena, &p, 0);
        let seven = numeral(&mut arena, &p, 7);
        let case_some = {
            // fun (x:Nat) => x
            let x = arena.bound_var(0);
            arena.lam(Binder(interner.intern("x")), nat_c, x)
        };
        let rec_c = arena.const_(p.option_rec);
        let none_c = arena.const_(p.none);
        let some_c = arena.const_(p.some);

        let on_none = {
            let t = arena.app(rec_c, nat_c);
            let t = arena.app(t, motive);
            let t = arena.app(t, zero);
            let t = arena.app(t, case_some);
            let none_nat = arena.app(none_c, nat_c);
            arena.app(t, none_nat)
        };
        assert!(
            vf_kernel::definitional_equal(&mut arena, &registry, on_none, zero, &mut fuel).unwrap()
        );

        let some_seven = app2(&mut arena, some_c, nat_c, seven);
        let on_some = {
            let t = arena.app(rec_c, nat_c);
            let t = arena.app(t, motive);
            let t = arena.app(t, zero);
            let t = arena.app(t, case_some);
            arena.app(t, some_seven)
        };
        assert!(
            vf_kernel::definitional_equal(&mut arena, &registry, on_some, seven, &mut fuel)
                .unwrap()
        );
    }

    #[test]
    fn either_rec_selects_the_matching_side() {
        let (mut arena, mut interner, mut registry, mut fuel) = setup();
        let p = register_prelude(&mut registry, &mut arena, &mut interner, &mut fuel).unwrap();

        let nat_c = arena.const_(p.nat);
        let bool_c = arena.const_(p.bool_);
        let either_c = arena.const_(p.either);
        let either_ty = app2(&mut arena, either_c, nat_c, bool_c);
        let motive = {
            let dummy = Binder(interner.intern("_"));
            arena.lam(dummy, either_ty, nat_c)
        };
        let case_left = {
            // fun (x:Nat) => x
            let x = arena.bound_var(0);
            arena.lam(Binder(interner.intern("x")), nat_c, x)
        };
        let zero = numeral(&mut arena, &p, 0);
        let case_right = {
            // fun (_:Bool) => zero
            arena.lam(Binder(interner.intern("_")), bool_c, zero)
        };
        let rec_c = arena.const_(p.either_rec);
        let inl_c = arena.const_(p.inl);
        let inr_c = arena.const_(p.inr);
        let seven = numeral(&mut arena, &p, 7);

        let left_val = app3(&mut arena, inl_c, nat_c, bool_c, seven);
        let on_left = {
            let t = app4(&mut arena, rec_c, nat_c, bool_c, motive, case_left);
            let t = arena.app(t, case_right);
            arena.app(t, left_val)
        };
        assert!(
            vf_kernel::definitional_equal(&mut arena, &registry, on_left, seven, &mut fuel)
                .unwrap()
        );

        let true_c = arena.const_(p.true_);
        let right_val = app3(&mut arena, inr_c, nat_c, bool_c, true_c);
        let on_right = {
            let t = app4(&mut arena, rec_c, nat_c, bool_c, motive, case_left);
            let t = arena.app(t, case_right);
            arena.app(t, right_val)
        };
        assert!(
            vf_kernel::definitional_equal(&mut arena, &registry, on_right, zero, &mut fuel)
                .unwrap()
        );
    }

    #[test]
    fn vector_rec_computes_a_length_via_recursion_on_a_two_element_vector() {
        let (mut arena, mut interner, mut registry, mut fuel) = setup();
        let p = register_prelude(&mut registry, &mut arena, &mut interner, &mut fuel).unwrap();

        let nat_c = arena.const_(p.nat);
        let bool_c = arena.const_(p.bool_);
        let vector_c = arena.const_(p.vector);
        let vnil_c = arena.const_(p.vnil);
        let vcons_c = arena.const_(p.vcons);
        let vector_rec_c = arena.const_(p.vector_rec);
        let succ_c = arena.const_(p.succ);
        let zero_c = arena.const_(p.zero);
        let true_c = arena.const_(p.true_);
        let false_c = arena.const_(p.false_);

        // v := vcons Bool 1 true (vcons Bool 0 false (vnil Bool)) : Vector Bool 2
        let one = numeral(&mut arena, &p, 1);
        let zero = numeral(&mut arena, &p, 0);
        let vnil_bool = arena.app(vnil_c, bool_c);
        let v1 = app4(&mut arena, vcons_c, bool_c, zero, false_c, vnil_bool);
        let v = app4(&mut arena, vcons_c, bool_c, one, true_c, v1);

        // motive : Pi (k:Nat), Vector Bool k -> Type0 := fun k _ => Nat
        let motive = {
            let k_ty = nat_c;
            let bound_k = arena.bound_var(0);
            let vec_bool_k_placeholder = app2(&mut arena, vector_c, bool_c, bound_k);
            let inner = arena.lam(Binder(interner.intern("_v")), vec_bool_k_placeholder, nat_c);
            arena.lam(Binder(interner.intern("k")), k_ty, inner)
        };
        let case_base = zero_c; // pn : P zero (vnil A) := zero
        let case_step = {
            // fun (_k:Nat) (_x:Bool) (_xs:Vector Bool _k) (ih:Nat) => succ ih
            let ih = arena.bound_var(0);
            let succ_ih = arena.app(succ_c, ih);
            let l4 = arena.lam(Binder(interner.intern("ih")), nat_c, succ_ih);
            let vec_placeholder = arena.sort(Sort::Type(0)); // untyped placeholder domain, fine: never inspected by reduction, only by full type-checking (not exercised in this reduction-focused test)
            let l3 = arena.lam(Binder(interner.intern("_xs")), vec_placeholder, l4);
            let l2 = arena.lam(Binder(interner.intern("_x")), bool_c, l3);
            arena.lam(Binder(interner.intern("_k")), nat_c, l2)
        };
        let two = numeral(&mut arena, &p, 2);

        let length_v = {
            let t = arena.app(vector_rec_c, bool_c);
            let t = arena.app(t, motive);
            let t = arena.app(t, case_base);
            let t = arena.app(t, case_step);
            let t = arena.app(t, two); // k = 2
            arena.app(t, v)
        };
        assert!(
            vf_kernel::definitional_equal(&mut arena, &registry, length_v, two, &mut fuel).unwrap()
        );
    }

    #[test]
    fn fin_rec_distinguishes_fin_zero_from_fin_succ() {
        let (mut arena, mut interner, mut registry, mut fuel) = setup();
        let p = register_prelude(&mut registry, &mut arena, &mut interner, &mut fuel).unwrap();

        let nat_c = arena.const_(p.nat);
        let fin_c = arena.const_(p.fin);
        let fin_zero_c = arena.const_(p.fin_zero);
        let fin_succ_c = arena.const_(p.fin_succ);
        let fin_rec_c = arena.const_(p.fin_rec);
        let bool_c = arena.const_(p.bool_);
        let true_c = arena.const_(p.true_);
        let false_c = arena.const_(p.false_);

        // motive : Pi (n:Nat), Fin n -> Type0 := fun _ _ => Bool
        let motive = {
            let bound_n = arena.bound_var(0);
            let fin_n_placeholder = arena.app(fin_c, bound_n);
            let inner = arena.lam(Binder(interner.intern("_i")), fin_n_placeholder, bool_c);
            arena.lam(Binder(interner.intern("n")), nat_c, inner)
        };
        let case_zero = {
            // fun (_n:Nat) => true
            arena.lam(Binder(interner.intern("_n")), nat_c, true_c)
        };
        let case_succ = {
            // fun (_n:Nat) (_i: <placeholder>) (_ih:Bool) => false
            let placeholder_ty = arena.sort(Sort::Type(0));
            let l3 = arena.lam(Binder(interner.intern("_ih")), bool_c, false_c);
            let l2 = arena.lam(Binder(interner.intern("_i")), placeholder_ty, l3);
            arena.lam(Binder(interner.intern("_n")), nat_c, l2)
        };

        let one = numeral(&mut arena, &p, 1);
        let two = numeral(&mut arena, &p, 2);

        // fin_zero 1 : Fin 2
        let fz = arena.app(fin_zero_c, one);
        let on_zero = app4(&mut arena, fin_rec_c, motive, case_zero, case_succ, one);
        let on_zero = arena.app(on_zero, fz);
        assert!(
            vf_kernel::definitional_equal(&mut arena, &registry, on_zero, true_c, &mut fuel)
                .unwrap()
        );

        // fin_succ 0 (fin_zero 0) : Fin 2
        let zero_for_fz0 = arena.const_(p.zero);
        let fz0 = arena.app(fin_zero_c, zero_for_fz0);
        let zero_for_fs = numeral(&mut arena, &p, 0);
        let fs = app2(&mut arena, fin_succ_c, zero_for_fs, fz0);
        let on_succ = app4(&mut arena, fin_rec_c, motive, case_zero, case_succ, one);
        let on_succ = arena.app(on_succ, fs);
        let _ = two;
        assert!(
            vf_kernel::definitional_equal(&mut arena, &registry, on_succ, false_c, &mut fuel)
                .unwrap()
        );
    }
}
