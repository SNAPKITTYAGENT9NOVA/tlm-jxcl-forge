require "./matrix"

# Crystal mirror of matlab/+{lu,qr,svd,cholesky}/certifyDecomposition.m and
# of alloy/invariants/decomposition.als: a certificate is certified exactly
# when every invariant its kind requires is within tolerance.
#
# Hardening relative to the MATLAB originals:
#   - relative errors divide by max(norm, EPS) so a zero matrix gives a
#     finite error instead of NaN (MATLAB: norm(A - Q*R)/norm(A) = 0/0);
#   - SVD orthogonality compares U'U with an identity sized to U's columns,
#     and the off-diagonal test works for rectangular S (the MATLAB code
#     compares a full m-by-m U'U with eye(min(m,n)) and builds a square
#     diag(diag(S)), both of which throw on non-square input).
module Invariants::Certify
  EPS = Float64::EPSILON

  enum Kind
    LU
    QR
    SVD
    Cholesky
  end

  enum Invariant
    Reconstruction
    LowerTriangular
    UpperTriangular
    UnitDiagonal
    Orthogonal
    OrthogonalU
    OrthogonalV
    DiagonalNonneg
    Symmetric
    PositiveDiagonal
    PositiveDefinite
  end

  REQUIRED = {
    Kind::LU       => [Invariant::Reconstruction, Invariant::LowerTriangular, Invariant::UpperTriangular, Invariant::UnitDiagonal],
    Kind::QR       => [Invariant::Reconstruction, Invariant::Orthogonal, Invariant::UpperTriangular],
    Kind::SVD      => [Invariant::Reconstruction, Invariant::OrthogonalU, Invariant::OrthogonalV, Invariant::DiagonalNonneg],
    Kind::Cholesky => [Invariant::Reconstruction, Invariant::LowerTriangular, Invariant::Symmetric, Invariant::PositiveDiagonal, Invariant::PositiveDefinite],
  }

  class Certificate
    getter kind : Kind
    getter errors : Hash(Invariant, Float64)
    getter tolerance : Float64

    def initialize(@kind, @errors, @tolerance = 1e-10)
      raise ArgumentError.new("tolerance must be positive") unless @tolerance > 0
      missing = REQUIRED[@kind] - @errors.keys
      raise ArgumentError.new("#{@kind} certificate missing #{missing}") unless missing.empty?
    end

    def satisfied : Array(Invariant)
      @errors.select { |_, e| e.finite? && e <= @tolerance }.keys
    end

    def violated : Array(Invariant)
      REQUIRED[@kind] - satisfied
    end

    def certified? : Bool
      violated.empty?
    end
  end

  def self.rel(num : Float64, den : Float64) : Float64
    num / Math.max(den, EPS)
  end

  def self.lu(a : Matrix, p : Matrix, l : Matrix, u : Matrix, tol = 1e-10) : Certificate
    d = l.diag
    Certificate.new(Kind::LU, {
      Invariant::Reconstruction  => rel((p * a - l * u).frobenius, a.frobenius),
      Invariant::LowerTriangular => rel(l.strict_part(upper: true).frobenius, l.frobenius),
      Invariant::UpperTriangular => rel(u.strict_part(upper: false).frobenius, u.frobenius),
      Invariant::UnitDiagonal    => rel(Math.sqrt(d.sum { |x| (x - 1) ** 2 }), Math.sqrt(d.sum { |x| x * x })),
    }, tol)
  end

  def self.qr(a : Matrix, q : Matrix, r : Matrix, tol = 1e-10) : Certificate
    i = Matrix.identity(q.cols)
    Certificate.new(Kind::QR, {
      Invariant::Reconstruction  => rel((a - q * r).frobenius, a.frobenius),
      Invariant::Orthogonal      => rel((q.transpose * q - i).frobenius, i.frobenius),
      Invariant::UpperTriangular => rel(r.strict_part(upper: false).frobenius, r.frobenius),
    }, tol)
  end

  def self.svd(a : Matrix, u : Matrix, s : Matrix, v : Matrix, tol = 1e-10) : Certificate
    iu, iv = Matrix.identity(u.cols), Matrix.identity(v.cols)
    sv = s.diag
    diag_err = rel(s.off_diagonal.frobenius, s.frobenius)
    diag_err = Float64::INFINITY if sv.any? { |x| x < -tol }
    Certificate.new(Kind::SVD, {
      Invariant::Reconstruction => rel((a - u * s * v.transpose).frobenius, a.frobenius),
      Invariant::OrthogonalU    => rel((u.transpose * u - iu).frobenius, iu.frobenius),
      Invariant::OrthogonalV    => rel((v.transpose * v - iv).frobenius, iv.frobenius),
      Invariant::DiagonalNonneg => diag_err,
    }, tol)
  end

  def self.cholesky(a : Matrix, tol = 1e-10) : Certificate
    l = Decompose.cholesky(a)
    inf = Float64::INFINITY
    Certificate.new(Kind::Cholesky, {
      Invariant::Reconstruction   => l ? rel((a - l * l.transpose).frobenius, a.frobenius) : inf,
      Invariant::LowerTriangular  => l ? rel(l.strict_part(upper: true).frobenius, l.frobenius) : inf,
      Invariant::Symmetric        => a.square? ? rel((a - a.transpose).frobenius, a.frobenius) : inf,
      Invariant::PositiveDiagonal => l && l.diag.min > 0 ? 0.0 : inf,
      Invariant::PositiveDefinite => l ? 0.0 : inf,
    }, tol)
  end
end
