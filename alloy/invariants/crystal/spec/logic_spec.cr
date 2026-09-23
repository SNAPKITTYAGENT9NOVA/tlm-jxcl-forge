require "./spec_helper"

include Invariants::Logic

private def p
  Atom.new("P")
end

private def q
  Atom.new("Q")
end

private def r
  Atom.new("R")
end

describe "lemmas.als invariants" do
  it "I1 detects dependency cycles and accepts DAGs" do
    a = Lemma.new([] of Prop, p)
    b = Lemma.new([] of Prop, q, [a])
    Invariants::Logic.acyclic?([a, b]).should be_true
    a.dependencies << b
    Invariants::Logic.acyclic?([a, b]).should be_false
  end

  it "I2 a counterexample exists exactly when the lemma is unsound" do
    [Lemma.new([p] of Prop, q), Lemma.new([p] of Prop, p)].each do |l|
      l.sound?.should eq(l.counterexample.nil?)
    end
  end

  it "I3 excluded middle and I4 non-contradiction" do
    Lemma.new([] of Prop, Or.new(p, Not.new(p))).sound?.should be_true
    Lemma.new([And.new(p, Not.new(p))] of Prop, q).contradictory?.should be_true
  end

  it "I5 ex falso: contradictory assumptions make any lemma sound" do
    l = Lemma.new([p, Not.new(p)] of Prop, q)
    l.contradictory?.should be_true
    l.sound?.should be_true
  end

  it "I6 modus ponens and I7 hypothetical syllogism" do
    Lemma.new([Implies.new(p, q), p] of Prop, q).sound?.should be_true
    Lemma.new([Implies.new(p, q), Implies.new(q, r)] of Prop, Implies.new(p, r)).sound?.should be_true
  end

  it "I8 De Morgan and I9 contrapositive" do
    Invariants::Logic.equivalent?(Not.new(And.new(p, q)), Or.new(Not.new(p), Not.new(q))).should be_true
    Invariants::Logic.equivalent?(Implies.new(p, q), Implies.new(Not.new(q), Not.new(p))).should be_true
  end

  it "I10 adding assumptions preserves soundness" do
    base = Lemma.new([Implies.new(p, q), p] of Prop, q)
    Lemma.new(base.assumptions + [r] of Prop, q).sound?.should be_true
  end

  it "C1 not every lemma is sound (original ExFalsoIsSound was false)" do
    Lemma.new([p] of Prop, q).counterexample.should eq({"P" => true, "Q" => false})
  end

  it "C2 affirming the consequent has a counterexample" do
    Lemma.new([Implies.new(p, q), q] of Prop, p).counterexample.should eq({"P" => false, "Q" => true})
  end

  it "C3 an implication is not equivalent to its converse" do
    Invariants::Logic.equivalent?(Implies.new(p, q), Implies.new(q, p)).should be_false
  end

  it "refuses to search beyond its bound instead of truncating" do
    atoms = (1..21).map { |i| Atom.new("A#{i}").as(Prop) }
    expect_raises(ArgumentError) { Lemma.new(atoms, p).counterexample }
  end
end
