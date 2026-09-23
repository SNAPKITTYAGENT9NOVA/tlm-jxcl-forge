# Crystal mirror of matlab/+formal/certifyTransition.m / certifyTrace.m and
# alloy/invariants/trace.als.
module Invariants::Trace
  record StepCertificate(S, X), source : S, input : X, target : S,
    source_valid : Bool, input_admissible : Bool, target_invariant : Bool do
    def certified? : Bool
      source_valid && input_admissible && target_invariant
    end
  end

  record TraceCertificate(S, X), steps : Array(StepCertificate(S, X)), final : S do
    def certified? : Bool
      steps.all?(&.certified?)
    end

    def first_failure : StepCertificate(S, X)?
      steps.find { |s| !s.certified? }
    end
  end

  def self.transition(f : Proc(S, X, S), valid : Proc(S, Bool), admissible : Proc(X, Bool),
                      invariant : Proc(S, Bool), s : S, x : X) : StepCertificate(S, X) forall S, X
    target = f.call(s, x)
    StepCertificate(S, X).new(s, x, target, valid.call(s), admissible.call(x), invariant.call(target))
  end

  def self.trace(f : Proc(S, X, S), valid : Proc(S, Bool), admissible : Proc(X, Bool),
                 invariant : Proc(S, Bool), s0 : S, inputs : Array(X)) : TraceCertificate(S, X) forall S, X
    state = s0
    steps = inputs.map do |x|
      cert = transition(f, valid, admissible, invariant, state, x)
      state = cert.target
      cert
    end
    TraceCertificate(S, X).new(steps, state)
  end

  # Bounded check of the inductive-invariant premise used by
  # trace.als/InductiveInvariant: Inv closed under F on admissible inputs
  # and Inv implies Valid, over finite sample domains. Returns the first
  # (state, input) pair breaking closure, or nil.
  def self.closure_counterexample(f : Proc(S, X, S), valid : Proc(S, Bool), admissible : Proc(X, Bool),
                                  invariant : Proc(S, Bool), states : Enumerable(S),
                                  inputs : Enumerable(X)) : Tuple(S, X)? forall S, X
    states.each do |s|
      next unless invariant.call(s)
      return {s, inputs.first} if !valid.call(s) && inputs.any?
      inputs.each do |x|
        next unless admissible.call(x)
        return {s, x} unless invariant.call(f.call(s, x))
      end
    end
    nil
  end
end
