require "./spec_helper"

include Invariants

F     = ->(s : Int32, x : Int32) { s + x }
VALID = ->(s : Int32) { s >= 0 }
ADM   = ->(x : Int32) { x >= 0 }
INV   = ->(s : Int32) { s >= 0 }

describe "trace.als invariants" do
  it "T1/T2 a certified trace keeps the invariant at every step" do
    t = Trace.trace(F, VALID, ADM, INV, 0, [1, 2, 1])
    t.certified?.should be_true
    t.steps.all?(&.target_invariant).should be_true
    t.final.should eq(4)
  end

  it "T2 trace fails iff some step fails" do
    t = Trace.trace(F, VALID, ADM, INV, 0, [1, -5, 10])
    t.certified?.should be_false
    t.first_failure.not_nil!.input.should eq(-5)
  end

  it "T3 an inductive invariant certifies every trace from it" do
    Trace.closure_counterexample(F, VALID, ADM, INV, (-3..10), (-2..5)).should be_nil
    [[0, 5], [3, 3, 3], [] of Int32].each do |inputs|
      Trace.trace(F, VALID, ADM, INV, 2, inputs).certified?.should be_true
    end
  end

  it "C1 checking only the final state is insufficient" do
    t = Trace.trace(F, VALID, ->(_x : Int32) { true }, INV, 0, [-5, 10])
    INV.call(t.final).should be_true
    t.certified?.should be_false
  end

  it "C2 closure without Inv-implies-Valid is found by the counter-search" do
    strict_valid = ->(s : Int32) { s > 0 }
    Trace.closure_counterexample(F, strict_valid, ADM, INV, (0..3), (0..2)).should eq({0, 0})
  end
end
