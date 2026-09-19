//! Iota reduction: the computation rule for recursors over the
//! built-in inductive types (`Nat`, `Bool`, `List`, `Option`,
//! `Either`, `Vector`, `Fin` -- registered by `vf-axioms`).
//!
//! This module knows nothing about any *specific* inductive type. It
//! knows one thing, generically, about all of them: a recursor is a
//! `Const` which, once applied to enough arguments that its last
//! argument (the "major premise") is headed by a known constructor,
//! reduces by picking that constructor's case-handling function
//! (a "minor premise") and applying it to the constructor's own
//! arguments -- passing each recursive argument's own recursively-
//! reduced result alongside it, exactly like `Nat`'s familiar
//! `rec zero := z; rec (succ n) := s n (rec n)`, generalized to any
//! shape of constructor. This is structural/primitive recursion: the
//! recursor can only ever be re-invoked on a strict sub-part of the
//! value it was just given, which is what keeps computation total
//! without this crate having to prove a termination metatheory (see
//! this crate's top-level doc comment on fuel instead).
use crate::{consume, whnf, DeltaContext, ReduceError};
use vf_core::{Symbol, Term, TermArena, TermId};

/// One constructor of a registered inductive type, as far as iota
/// reduction needs to know: for each of the constructor's own
/// explicit arguments (in order), whether that argument is itself a
/// recursive occurrence of the inductive type being defined (e.g.
/// `succ`'s `Nat` argument, `cons`'s `List A` tail) -- a recursive
/// argument gets passed to the case function *and* has the recursor
/// re-applied to it, a non-recursive one (e.g. `cons`'s head element)
/// is passed through as-is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstructorSpec {
    pub recursive_args: Vec<bool>,
}

/// A recursor's reduction shape. Deliberately says nothing about the
/// recursor's *type* (how many uniform parameters, indices, or what
/// the motive's own type looks like) -- only what iota reduction
/// needs: given a fully-applied `Recursor a_0 .. a_{arity-1}` (the
/// major premise is always `a_{arity-1}`, the last argument, by
/// convention), which argument position holds each constructor's case
/// function.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecursorSpec {
    /// Total argument count of a fully-applied call, major premise
    /// included (and always last).
    pub arity: u32,
    /// Parallel to `constructors`: `case_positions[i]` is the 0-based
    /// argument index (among the recursor's own `arity` arguments)
    /// holding constructor `i`'s case function.
    pub case_positions: Vec<u32>,
    /// The constructor names, in the same order as `case_positions`,
    /// so `vf-axioms` can build the constructor-to-recursor lookup
    /// table when it registers this spec.
    pub constructor_names: Vec<Symbol>,
    pub constructors: Vec<ConstructorSpec>,
}

/// Decompose `term` into its application spine: `(head, args)` such
/// that `term = head args[0] args[1] .. args[n-1]` (curried). `head`
/// is whatever the innermost non-`App` node is, unreduced.
fn decompose_app(arena: &TermArena, term: TermId) -> Result<(TermId, Vec<TermId>), ReduceError> {
    let mut args = Vec::new();
    let mut head = term;
    while let Term::App(f, a) = arena.get(head)?.clone() {
        args.push(a);
        head = f;
    }
    args.reverse();
    Ok((head, args))
}

/// If `term` is a fully-applied recursor call whose major premise is
/// headed by a known constructor, perform one iota reduction step and
/// return the result. Returns `Ok(None)` (never a "stuck" error) for
/// every other shape of term: under-applied, headed by something
/// other than a known recursor, or a major premise that doesn't (yet)
/// reduce to a constructor -- all of those are legitimate whnf-already
/// results, not failures.
pub(crate) fn try_iota_reduce<C: DeltaContext>(
    arena: &mut TermArena,
    ctx: &C,
    term: TermId,
    fuel: &mut u64,
) -> Result<Option<TermId>, ReduceError> {
    let (head, args) = decompose_app(arena, term)?;
    let Term::Const(rec_name) = arena.get(head)?.clone() else {
        return Ok(None);
    };
    let Some(spec) = ctx.recursor(rec_name) else {
        return Ok(None);
    };
    let spec = spec.clone();
    if (args.len() as u32) < spec.arity {
        return Ok(None); // not fully applied yet
    }
    let call_args = &args[..spec.arity as usize];
    let extra_args = &args[spec.arity as usize..];
    let major = call_args[spec.arity as usize - 1];

    let major_whnf = whnf(arena, ctx, major, fuel)?;
    let (ctor_head, ctor_args) = decompose_app(arena, major_whnf)?;
    let Term::Const(ctor_sym) = arena.get(ctor_head)?.clone() else {
        return Ok(None); // major premise isn't (yet) constructor-headed
    };
    let Some((owning_recursor, ctor_index)) = ctx.constructor_index(ctor_sym) else {
        return Ok(None);
    };
    if owning_recursor != rec_name {
        // `ctor_sym` belongs to a different recursor's type than the
        // one being applied here -- ill-typed input. vf-kernel's
        // typing judgment is what rejects this; iota reduction simply
        // declines to reduce a term it can't make sense of, rather
        // than guessing.
        return Ok(None);
    }
    let ctor_spec = &spec.constructors[ctor_index];
    if ctor_args.len() != ctor_spec.recursive_args.len() {
        return Ok(None); // malformed constructor application; decline, don't guess
    }

    let mut result = call_args[spec.case_positions[ctor_index] as usize];
    for (i, &is_recursive) in ctor_spec.recursive_args.iter().enumerate() {
        let arg = ctor_args[i];
        result = arena.app(result, arg);
        if is_recursive {
            let mut sub_call = head;
            for (j, &call_arg) in call_args.iter().enumerate() {
                let this_arg = if j + 1 == call_args.len() {
                    arg
                } else {
                    call_arg
                };
                sub_call = arena.app(sub_call, this_arg);
            }
            result = arena.app(result, sub_call);
        }
    }
    for &extra in extra_args {
        result = arena.app(result, extra);
    }
    consume(fuel)?;
    Ok(Some(result))
}

#[cfg(test)]
mod tests {
    use crate::{normalize, whnf, DeltaContext, DEFAULT_FUEL};
    use vf_core::{Binder, Interner, Sort, TermArena, TermId};

    use super::{try_iota_reduce, ConstructorSpec, RecursorSpec};
    use vf_core::Symbol;

    /// A from-scratch, minimal "Nat" for exercising iota reduction in
    /// isolation, with no typing involved at all -- this crate's
    /// [`super::RecursorSpec`] mechanism only ever looks at term shape,
    /// never types, so a standalone reduction test doesn't need
    /// vf-kernel or vf-axioms in the loop.
    struct NatCtx {
        zero: Symbol,
        succ: Symbol,
        rec: Symbol,
        spec: RecursorSpec,
    }

    impl DeltaContext for NatCtx {
        fn unfold(&self, _name: Symbol) -> Option<TermId> {
            None
        }
        fn recursor(&self, name: Symbol) -> Option<&RecursorSpec> {
            (name == self.rec).then_some(&self.spec)
        }
        fn constructor_index(&self, name: Symbol) -> Option<(Symbol, usize)> {
            if name == self.zero {
                Some((self.rec, 0))
            } else if name == self.succ {
                Some((self.rec, 1))
            } else {
                None
            }
        }
    }

    fn setup_nat(interner: &mut Interner) -> NatCtx {
        let zero = interner.intern("zero");
        let succ = interner.intern("succ");
        let rec = interner.intern("Nat_rec");
        // Nat_rec case_z case_s n -- no motive: iota reduction never
        // looks at it, so omitting it keeps this standalone test
        // focused on the reduction mechanism itself.
        let spec = RecursorSpec {
            arity: 3,
            case_positions: vec![0, 1],
            constructor_names: vec![zero, succ],
            constructors: vec![
                ConstructorSpec {
                    recursive_args: vec![],
                }, // zero: no args
                ConstructorSpec {
                    recursive_args: vec![true],
                }, // succ: one recursive Nat arg
            ],
        };
        NatCtx {
            zero,
            succ,
            rec,
            spec,
        }
    }

    fn numeral(arena: &mut TermArena, succ: Symbol, zero: Symbol, n: u32) -> TermId {
        let mut t = arena.const_(zero);
        let succ_const = arena.const_(succ);
        for _ in 0..n {
            t = arena.app(succ_const, t);
        }
        t
    }

    #[test]
    fn iota_reduces_the_base_case_directly() {
        let mut arena = TermArena::new();
        let mut interner = Interner::new();
        let ctx = setup_nat(&mut interner);
        let case_z = arena.sort(Sort::Type(0)); // an arbitrary closed "result" for the base case
        let case_s = arena.sort(Sort::Type(1)); // never invoked in this test
        let zero_term = numeral(&mut arena, ctx.succ, ctx.zero, 0);
        let call = {
            let c = arena.const_(ctx.rec);
            let c = arena.app(c, case_z);
            let c = arena.app(c, case_s);
            arena.app(c, zero_term)
        };
        let mut fuel = DEFAULT_FUEL;
        let result = whnf(&mut arena, &ctx, call, &mut fuel).unwrap();
        assert_eq!(result, case_z);
    }

    #[test]
    fn iota_computes_addition_via_primitive_recursion() {
        // add(m, n) := Nat_rec n (fun _ ih => succ ih) m
        // add(2, 3) should normalize to the literal numeral 5.
        let mut arena = TermArena::new();
        let mut interner = Interner::new();
        let ctx = setup_nat(&mut interner);

        let two = numeral(&mut arena, ctx.succ, ctx.zero, 2);
        let three = numeral(&mut arena, ctx.succ, ctx.zero, 3);
        let five = numeral(&mut arena, ctx.succ, ctx.zero, 5);

        let dummy_ty = arena.sort(Sort::Type(0));
        let succ_const = arena.const_(ctx.succ);
        let ih = arena.bound_var(0);
        let inner_body = arena.app(succ_const, ih);
        let inner_lam = arena.lam(Binder(interner.intern("ih")), dummy_ty, inner_body);
        let case_s = arena.lam(Binder(interner.intern("_pred")), dummy_ty, inner_lam);

        let add = {
            let c = arena.const_(ctx.rec);
            let c = arena.app(c, three); // case_z = n = 3
            let c = arena.app(c, case_s);
            arena.app(c, two) // major premise = m = 2
        };

        let mut fuel = DEFAULT_FUEL;
        let normalized = normalize(&mut arena, &ctx, add, &mut fuel).unwrap();
        assert_eq!(normalized, five);
    }

    #[test]
    fn a_recursor_call_with_too_few_arguments_does_not_reduce() {
        let mut arena = TermArena::new();
        let mut interner = Interner::new();
        let ctx = setup_nat(&mut interner);
        let case_z = arena.sort(Sort::Type(0));
        let partial = {
            let c = arena.const_(ctx.rec);
            arena.app(c, case_z) // only 1 of 3 required args
        };
        let mut fuel = DEFAULT_FUEL;
        assert_eq!(
            try_iota_reduce(&mut arena, &ctx, partial, &mut fuel).unwrap(),
            None
        );
        // whnf must likewise leave it exactly as-is, not panic or hang.
        assert_eq!(whnf(&mut arena, &ctx, partial, &mut fuel).unwrap(), partial);
    }

    #[test]
    fn a_major_premise_that_is_a_free_variable_does_not_reduce() {
        // Nat_rec case_z case_s x, where x is an unresolved free
        // variable -- iota must decline, not guess.
        let mut arena = TermArena::new();
        let mut interner = Interner::new();
        let ctx = setup_nat(&mut interner);
        let case_z = arena.sort(Sort::Type(0));
        let case_s = arena.sort(Sort::Type(1));
        let x = arena.free_var(interner.intern("x"));
        let call = {
            let c = arena.const_(ctx.rec);
            let c = arena.app(c, case_z);
            let c = arena.app(c, case_s);
            arena.app(c, x)
        };
        let mut fuel = DEFAULT_FUEL;
        let result = whnf(&mut arena, &ctx, call, &mut fuel).unwrap();
        assert_eq!(
            result, call,
            "stuck on the free variable, not silently reduced"
        );
    }

    #[test]
    fn a_constructor_from_an_unrelated_recursor_never_triggers_reduction() {
        // Two independent families sharing one context. Applying
        // Bool's `true_` as Nat_rec's major premise is a nonsensical
        // (ill-typed) term, but iota reduction must decline to reduce
        // it rather than misapplying Nat_rec's case functions to it --
        // that's vf-kernel's typing judgment's job to reject, not
        // something this crate resolves by guessing.
        struct TwoFamilies {
            nat: NatCtx,
            bool_rec: Symbol,
            true_: Symbol,
            false_: Symbol,
            bool_spec: RecursorSpec,
        }
        impl DeltaContext for TwoFamilies {
            fn unfold(&self, _name: Symbol) -> Option<TermId> {
                None
            }
            fn recursor(&self, name: Symbol) -> Option<&RecursorSpec> {
                if name == self.nat.rec {
                    Some(&self.nat.spec)
                } else if name == self.bool_rec {
                    Some(&self.bool_spec)
                } else {
                    None
                }
            }
            fn constructor_index(&self, name: Symbol) -> Option<(Symbol, usize)> {
                if name == self.nat.zero {
                    Some((self.nat.rec, 0))
                } else if name == self.nat.succ {
                    Some((self.nat.rec, 1))
                } else if name == self.true_ {
                    Some((self.bool_rec, 0))
                } else if name == self.false_ {
                    Some((self.bool_rec, 1))
                } else {
                    None
                }
            }
        }

        let mut arena = TermArena::new();
        let mut interner = Interner::new();
        let nat = setup_nat(&mut interner);
        let bool_rec = interner.intern("Bool_rec");
        let true_ = interner.intern("true_");
        let false_ = interner.intern("false_");
        let bool_spec = RecursorSpec {
            arity: 3,
            case_positions: vec![0, 1],
            constructor_names: vec![true_, false_],
            constructors: vec![
                ConstructorSpec {
                    recursive_args: vec![],
                },
                ConstructorSpec {
                    recursive_args: vec![],
                },
            ],
        };
        let ctx = TwoFamilies {
            nat,
            bool_rec,
            true_,
            false_,
            bool_spec,
        };

        let case_z = arena.sort(Sort::Type(0));
        let case_s = arena.sort(Sort::Type(1));
        let true_term = arena.const_(true_);
        let call = {
            let c = arena.const_(ctx.nat.rec);
            let c = arena.app(c, case_z);
            let c = arena.app(c, case_s);
            arena.app(c, true_term) // Bool's `true_` where Nat_rec expects a Nat
        };

        let mut fuel = DEFAULT_FUEL;
        let result = whnf(&mut arena, &ctx, call, &mut fuel).unwrap();
        assert_eq!(
            result, call,
            "must stay stuck, not reduce using the wrong family's case functions"
        );
    }
}
