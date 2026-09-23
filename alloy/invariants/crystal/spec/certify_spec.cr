require "./spec_helper"

include Invariants
alias Inv = Invariants::Certify::Invariant

private def m(rows) : Matrix
  Matrix.from(rows.map(&.map(&.to_f64)))
end

private def rotation(t : Float64) : Matrix
  Matrix.from([[Math.cos(t), -Math.sin(t)], [Math.sin(t), Math.cos(t)]])
end

A   = [[4, 3, 2], [6, 3, 1], [2, 5, 7]]
SPD = [[4, 2, 0], [2, 5, 1], [0, 1, 3]]

describe "decomposition.als invariants" do
  it "D1 certified LU, QR and Cholesky satisfy every required invariant" do
    a = m(A)
    lu = Decompose.lu(a)
    Certify.lu(a, lu.p, lu.l, lu.u).certified?.should be_true
    qr = Decompose.qr(a)
    Certify.qr(a, qr.q, qr.r).certified?.should be_true
    Certify.cholesky(m(SPD)).certified?.should be_true
  end

  it "D1 certifies an SVD built from orthogonal factors, including rectangular" do
    u, v = rotation(0.3), rotation(1.1)
    s = Matrix.diagonal(2, 2, [3.0, 0.5])
    Certify.svd(u * s * v.transpose, u, s, v).certified?.should be_true
    u3 = Matrix.identity(3)
    s32 = Matrix.diagonal(3, 2, [2.0, 1.0])
    Certify.svd(u3 * s32 * v.transpose, u3, s32, v).certified?.should be_true
  end

  it "D2/D3 every kind requires reconstruction and 3..5 invariants" do
    Certify::REQUIRED.each_value do |req|
      req.should contain(Inv::Reconstruction)
      (3..5).should contain(req.size)
    end
  end

  it "D4 Cholesky rejects symmetric indefinite and non-symmetric input" do
    Certify.cholesky(m([[1, 2], [2, 1]])).certified?.should be_false
    c = Certify.cholesky(m([[4, 1], [0, 3]]))
    c.certified?.should be_false
    c.violated.should contain(Inv::Symmetric)
  end

  it "C1 reconstruction alone is not certification" do
    a = m([[1, 2], [3, 4]])
    q = Matrix.identity(2)
    r = a
    cert = Certify.qr(a, q, r)
    cert.satisfied.should contain(Inv::Reconstruction)
    cert.violated.should eq([Inv::UpperTriangular])
    cert.certified?.should be_false
  end

  it "C2 dropping any one required invariant decertifies" do
    a = m(A)
    lu = Decompose.lu(a)
    good = Certify.lu(a, lu.p, lu.l, lu.u)
    Certify::REQUIRED[Certify::Kind::LU].each do |inv|
      errs = good.errors.dup
      errs[inv] = 1.0
      Certify::Certificate.new(Certify::Kind::LU, errs).certified?.should be_false
    end
  end

  it "hardening: a zero matrix yields finite errors, never NaN" do
    z = Matrix.new(2, 2)
    qr = Decompose.qr(z)
    Certify.qr(z, qr.q, qr.r).errors.values.all?(&.finite?).should be_true
  end

  it "hardening: a certificate missing a required invariant is rejected" do
    expect_raises(ArgumentError) do
      Certify::Certificate.new(Certify::Kind::QR, {Inv::Reconstruction => 0.0})
    end
  end
end
