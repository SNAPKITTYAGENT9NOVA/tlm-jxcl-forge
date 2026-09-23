# Minimal dense matrix kernel for the certification invariants.
module Invariants
  class Matrix
    getter rows : Int32, cols : Int32
    getter data : Array(Float64)

    def initialize(@rows, @cols, @data = Array(Float64).new(rows * cols, 0.0))
      raise ArgumentError.new("shape #{rows}x#{cols} does not match #{data.size} entries") unless data.size == rows * cols
    end

    def self.from(rows : Array(Array(Float64))) : Matrix
      cols = rows.first?.try(&.size) || 0
      raise ArgumentError.new("ragged matrix") unless rows.all? { |r| r.size == cols }
      Matrix.new(rows.size, cols, rows.flatten)
    end

    def self.identity(n : Int32) : Matrix
      m = Matrix.new(n, n)
      n.times { |i| m[i, i] = 1.0 }
      m
    end

    def self.diagonal(rows : Int32, cols : Int32, values : Array(Float64)) : Matrix
      m = Matrix.new(rows, cols)
      values.each_with_index { |x, i| m[i, i] = x }
      m
    end

    def [](i : Int32, j : Int32) : Float64
      @data[i * @cols + j]
    end

    def []=(i : Int32, j : Int32, x : Float64)
      @data[i * @cols + j] = x
    end

    def square? : Bool
      @rows == @cols
    end

    def transpose : Matrix
      t = Matrix.new(@cols, @rows)
      @rows.times { |i| @cols.times { |j| t[j, i] = self[i, j] } }
      t
    end

    def *(o : Matrix) : Matrix
      raise ArgumentError.new("inner dimensions #{@cols} != #{o.rows}") unless @cols == o.rows
      r = Matrix.new(@rows, o.cols)
      @rows.times do |i|
        @cols.times do |k|
          a = self[i, k]
          next if a == 0.0
          o.cols.times { |j| r[i, j] += a * o[k, j] }
        end
      end
      r
    end

    def -(o : Matrix) : Matrix
      same_shape!(o)
      Matrix.new(@rows, @cols, @data.map_with_index { |x, i| x - o.data[i] })
    end

    def +(o : Matrix) : Matrix
      same_shape!(o)
      Matrix.new(@rows, @cols, @data.map_with_index { |x, i| x + o.data[i] })
    end

    def frobenius : Float64
      Math.sqrt(@data.sum { |x| x * x })
    end

    def diag : Array(Float64)
      (0...Math.min(@rows, @cols)).map { |i| self[i, i] }
    end

    # Entries strictly above (k=1) or strictly below (k=-1) the diagonal.
    def strict_part(upper : Bool) : Matrix
      m = Matrix.new(@rows, @cols)
      @rows.times { |i| @cols.times { |j| m[i, j] = self[i, j] if (upper ? j > i : j < i) } }
      m
    end

    def off_diagonal : Matrix
      m = Matrix.new(@rows, @cols, @data.dup)
      Math.min(@rows, @cols).times { |i| m[i, i] = 0.0 }
      m
    end

    def ==(o : Matrix) : Bool
      @rows == o.rows && @cols == o.cols && @data == o.data
    end

    private def same_shape!(o : Matrix)
      raise ArgumentError.new("shape mismatch") unless @rows == o.rows && @cols == o.cols
    end
  end

  module Decompose
    record LUResult, p : Matrix, l : Matrix, u : Matrix
    record QRResult, q : Matrix, r : Matrix

    # Gaussian elimination with partial pivoting: P*A = L*U.
    def self.lu(a : Matrix) : LUResult
      raise ArgumentError.new("LU needs a square matrix") unless a.square?
      n = a.rows
      u = Matrix.new(n, n, a.data.dup)
      l = Matrix.identity(n)
      perm = (0...n).to_a
      n.times do |k|
        pivot = (k...n).max_by { |i| u[i, k].abs }
        if pivot != k
          n.times { |j| u[k, j], u[pivot, j] = u[pivot, j], u[k, j] }
          k.times { |j| l[k, j], l[pivot, j] = l[pivot, j], l[k, j] }
          perm[k], perm[pivot] = perm[pivot], perm[k]
        end
        next if u[k, k] == 0.0
        ((k + 1)...n).each do |i|
          f = u[i, k] / u[k, k]
          l[i, k] = f
          (k...n).each { |j| u[i, j] -= f * u[k, j] }
        end
      end
      p = Matrix.new(n, n)
      perm.each_with_index { |src, i| p[i, src] = 1.0 }
      LUResult.new(p, l, u)
    end

    # Householder QR: A = Q*R with Q m-by-m orthogonal.
    def self.qr(a : Matrix) : QRResult
      m, n = a.rows, a.cols
      r = Matrix.new(m, n, a.data.dup)
      q = Matrix.identity(m)
      Math.min(m - 1, n).times do |k|
        x = (k...m).map { |i| r[i, k] }
        norm = Math.sqrt(x.sum { |e| e * e })
        next if norm == 0.0
        x[0] += (x[0] >= 0 ? norm : -norm)
        vnorm2 = x.sum { |e| e * e }
        next if vnorm2 == 0.0
        n.times do |j|
          s = 2.0 * (k...m).sum { |i| x[i - k] * r[i, j] } / vnorm2
          (k...m).each { |i| r[i, j] -= s * x[i - k] }
        end
        m.times do |i|
          s = 2.0 * (k...m).sum { |c| q[i, c] * x[c - k] } / vnorm2
          (k...m).each { |c| q[i, c] -= s * x[c - k] }
        end
      end
      QRResult.new(q, r)
    end

    # Lower Cholesky factor, or nil when A is not symmetric positive definite.
    def self.cholesky(a : Matrix) : Matrix?
      return nil unless a.square?
      n = a.rows
      l = Matrix.new(n, n)
      n.times do |j|
        d = a[j, j] - (0...j).sum { |k| l[j, k] ** 2 }
        return nil unless d > 0.0
        l[j, j] = Math.sqrt(d)
        ((j + 1)...n).each do |i|
          l[i, j] = (a[i, j] - (0...j).sum { |k| l[i, k] * l[j, k] }) / l[j, j]
        end
      end
      l
    end
  end
end
