# Crystal mirror of the DULA result classifier and recursion bound
# (tools/emacs/dula-lean-alloy.el, alloy/invariants/dula.als).
module Invariants::Dula
  enum Status
    Pending
    CounterexampleFound
    NoCounterexampleFound
  end

  RECURSION_LIMIT = 32

  class RecursionLimitExceeded < Exception; end

  # Alloy prints one line per command such as
  #   "00. check Name   0   UNSAT"  or  "00. run Name  0  1/1  SAT".
  # Only a whole-word SAT/UNSAT token counts; "UNSAT" must never be read as
  # "SAT", and the phrase "no counterexample found" must never be read as
  # a counterexample. Anything unrecognised stays Pending.
  def self.classify(output : String) : Status
    return Status::NoCounterexampleFound if output =~ /(^|\W)UNSAT(\W|$)/ || output =~ /no (instance|counterexample) found/i
    return Status::CounterexampleFound if output =~ /(^|\W)SAT(\W|$)/ || output =~ /counterexample found/i
    Status::Pending
  end

  # The pre-fix Emacs Lisp classifier, kept as a documented counter-example:
  # it matches substrings, and checks the counterexample patterns first.
  def self.legacy_classify(output : String) : Status
    if output.includes?("Counterexample") || output.includes?("SAT") ||
       output.includes?("Instance") || output.includes?("found")
      Status::CounterexampleFound
    elsif output.includes?("UNSAT") || output.includes?("No instance") ||
          output.downcase.includes?("no counterexample")
      Status::NoCounterexampleFound
    else
      Status::Pending
    end
  end

  class Assertion
    getter name : String
    property parents : Array(Assertion)
    property depth : Int32 = 0

    def initialize(@name, @parents = [] of Assertion); end
  end

  # Visits parents before the assertion itself, like dula-recursive-assert,
  # raising once depth exceeds the limit. Returns the visit order.
  def self.recursive_visit(a : Assertion, depth = 0, limit = RECURSION_LIMIT,
                           order = [] of {String, Int32}) : Array({String, Int32})
    raise RecursionLimitExceeded.new("depth #{depth} > #{limit} at #{a.name}") if depth > limit
    a.depth = depth
    a.parents.each { |p| recursive_visit(p, depth + 1, limit, order) }
    order << {a.name, depth}
    order
  end
end
