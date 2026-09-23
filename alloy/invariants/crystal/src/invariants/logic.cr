# Propositional semantics of alloy/invariants/core.als, with an exhaustive
# bounded counterexample search over every valuation of the atoms.
module Invariants::Logic
  alias Valuation = Hash(String, Bool)

  abstract class Prop
    abstract def holds?(v : Valuation) : Bool
    abstract def atoms : Set(String)
  end

  class Atom < Prop
    getter name : String

    def initialize(@name); end

    def holds?(v : Valuation) : Bool
      v.fetch(@name) { raise ArgumentError.new("unassigned atom #{@name}") }
    end

    def atoms : Set(String)
      Set{@name}
    end
  end

  class Not < Prop
    getter operand : Prop

    def initialize(@operand); end

    def holds?(v : Valuation) : Bool
      !@operand.holds?(v)
    end

    def atoms : Set(String)
      @operand.atoms
    end
  end

  abstract class Binary < Prop
    getter left : Prop, right : Prop

    def initialize(@left, @right); end

    def atoms : Set(String)
      @left.atoms | @right.atoms
    end
  end

  class And < Binary
    def holds?(v : Valuation) : Bool
      @left.holds?(v) && @right.holds?(v)
    end
  end

  class Or < Binary
    def holds?(v : Valuation) : Bool
      @left.holds?(v) || @right.holds?(v)
    end
  end

  class Implies < Binary
    def holds?(v : Valuation) : Bool
      !@left.holds?(v) || @right.holds?(v)
    end
  end

  # Enumerates the whole state space (2^n valuations). Bounded: refuses
  # more atoms than `max_atoms` rather than silently truncating.
  def self.valuations(atoms : Set(String), max_atoms = 20) : Array(Valuation)
    names = atoms.to_a.sort
    raise ArgumentError.new("#{names.size} atoms exceeds bound #{max_atoms}") if names.size > max_atoms
    (0...(1 << names.size)).map do |bits|
      v = Valuation.new
      names.each_with_index { |n, i| v[n] = bits.bit(i) == 1 }
      v
    end
  end

  class Lemma
    getter assumptions : Array(Prop), conclusion : Prop, dependencies : Array(Lemma)

    def initialize(@assumptions, @conclusion, @dependencies = [] of Lemma); end

    def atoms : Set(String)
      @assumptions.reduce(@conclusion.atoms) { |acc, p| acc | p.atoms }
    end

    def assumptions_hold?(v : Valuation) : Bool
      @assumptions.all?(&.holds?(v))
    end

    def violates?(v : Valuation) : Bool
      assumptions_hold?(v) && !@conclusion.holds?(v)
    end

    # The counter-algorithm: first genuine counterexample, or nil.
    def counterexample : Valuation?
      Logic.valuations(atoms).find { |v| violates?(v) }
    end

    def sound? : Bool
      counterexample.nil?
    end

    def contradictory? : Bool
      Logic.valuations(atoms).none? { |v| assumptions_hold?(v) }
    end
  end

  # Invariant I1: dependency graph is a DAG (DFS with colouring).
  def self.acyclic?(lemmas : Array(Lemma)) : Bool
    state = {} of UInt64 => Symbol
    visit = uninitialized Proc(Lemma, Bool)
    visit = ->(l : Lemma) do
      case state[l.object_id]?
      when :done   then true
      when :active then false
      else
        state[l.object_id] = :active
        ok = l.dependencies.all? { |d| visit.call(d) }
        state[l.object_id] = :done
        ok
      end
    end
    lemmas.all? { |l| visit.call(l) }
  end

  def self.equivalent?(a : Prop, b : Prop) : Bool
    valuations(a.atoms | b.atoms).all? { |v| a.holds?(v) == b.holds?(v) }
  end
end
