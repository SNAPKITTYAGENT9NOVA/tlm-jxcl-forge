require "./spec_helper"

include Invariants

UNSAT_LINE = "00. check NoCertifiedViolation     0       UNSAT"
SAT_LINE   = "03. run   ChainExceedsLimit        0    1/1     SAT"

describe "dula.als invariants" do
  it "R2 status is a function of the Alloy outcome" do
    Dula.classify(UNSAT_LINE).should eq(Dula::Status::NoCounterexampleFound)
    Dula.classify(SAT_LINE).should eq(Dula::Status::CounterexampleFound)
    Dula.classify("No counterexample found.").should eq(Dula::Status::NoCounterexampleFound)
    Dula.classify("Counterexample found. Assertion is invalid.").should eq(Dula::Status::CounterexampleFound)
  end

  it "R3 a failed run is never evidence" do
    Dula.classify("Syntax error at line 3").should eq(Dula::Status::Pending)
    Dula.classify("").should eq(Dula::Status::Pending)
  end

  it "C: the legacy substring classifier reads every UNSAT as a counterexample" do
    Dula.legacy_classify(UNSAT_LINE).should eq(Dula::Status::CounterexampleFound)
    Dula.legacy_classify("No counterexample found.").should eq(Dula::Status::CounterexampleFound)
  end

  it "R1 depth grows by one per parent hop and starts at zero" do
    root = Dula::Assertion.new("root")
    mid = Dula::Assertion.new("mid", [root])
    leaf = Dula::Assertion.new("leaf", [mid])
    Dula.recursive_visit(leaf).should eq([{"root", 2}, {"mid", 1}, {"leaf", 0}])
  end

  it "C1 a parent cycle is stopped by the recursion limit" do
    a = Dula::Assertion.new("a")
    b = Dula::Assertion.new("b", [a])
    a.parents = [b]
    expect_raises(Dula::RecursionLimitExceeded) { Dula.recursive_visit(a, limit: 4) }
  end
end
