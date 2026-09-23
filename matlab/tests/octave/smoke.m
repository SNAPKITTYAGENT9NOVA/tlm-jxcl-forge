function smoke()
  % Each row: label, certifier call, expected `certified`.
  randn("seed", 7);
  tol = 1e-10;
  cases = {
    "lu square",    @() clu.certifyDecomposition(randn(5)),                 true;
    "lu tall",      @() clu.certifyDecomposition(randn(8, 4)),              true;
    "lu wide",      @() clu.certifyDecomposition(randn(4, 8)),              true;
    "lu zero",      @() clu.certifyDecomposition(zeros(3)),                 true;
    "qr square",    @() cqr.certifyDecomposition(randn(5)),                 true;
    "qr tall",      @() cqr.certifyDecomposition(randn(8, 4)),              true;
    "qr wide",      @() cqr.certifyDecomposition(randn(4, 8)),              true;
    "qr zero",      @() cqr.certifyDecomposition(zeros(3)),                 true;
    "svd square",   @() csvd.certifyDecomposition(randn(5)),                true;
    "svd tall",     @() csvd.certifyDecomposition(randn(10, 5)),            true;
    "svd wide",     @() csvd.certifyDecomposition(randn(5, 10)),            true;
    "svd 3x1",      @() csvd.certifyDecomposition(randn(3, 1)),             true;
    "svd 1x3",      @() csvd.certifyDecomposition(randn(1, 3)),             true;
    "svd zero",     @() csvd.certifyDecomposition(zeros(3)),                true;
    "svd zero 3x2", @() csvd.certifyDecomposition(zeros(3, 2)),             true;
    "chol spd",     @() ccholesky.certifyDecomposition([4 2 0; 2 5 1; 0 1 3]), true;
    "chol zero",    @() ccholesky.certifyDecomposition(zeros(3)),           false;
    "chol indef",   @() ccholesky.certifyDecomposition([1 2; 2 1]),         false;
  };
  factors = {"A", "L", "U", "P", "Q", "R", "S", "V"};
  failures = 0;
  for k = 1:rows(cases)
    try
      c = cases{k, 2}();
      errs = struct2cell(rmfield(c, intersect(fieldnames(c), factors)));
      hasNaN = any(cellfun(@(x) isnumeric(x) && any(isnan(x(:))), errs));
      ok = (c.certified == cases{k, 3}) && ~hasNaN;
      msg = sprintf("certified=%d expected=%d nan=%d", c.certified, cases{k, 3}, hasNaN);
    catch err
      ok = false;
      msg = ["ERROR: " err.message];
    end
    status = "ok  ";
    if ~ok
      status = "FAIL";
    end
    printf("%s %-13s %s\n", status, cases{k, 1}, msg);
    failures += ~ok;
  end
  printf("%d failures\n", failures);
  exit(failures > 0);
end
