#!/usr/bin/env bash
# Smoke-test the matlab/+{lu,qr,svd,cholesky} certifiers under GNU Octave.
#
# Octave cannot parse MATLAB `arguments` blocks and its builtin lu/qr shadow
# the +lu/+qr packages, so each certifier is copied into a temp directory
# as +c<name> with the arguments block replaced by an equivalent default.
# The certifier bodies themselves run unmodified.
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
src=$(cd "$here/../.." && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
for pkg in lu qr svd cholesky; do
  mkdir -p "$work/+c$pkg"
  python3 - "$src/+$pkg/certifyDecomposition.m" "$work/+c$pkg/certifyDecomposition.m" <<'PY'
import re, sys
s = open(sys.argv[1]).read()
s, n = re.subn(r'\narguments\n.*?\nend\n',
               "\nif nargin < 2\n    tolerance = 1e-10;\nend\n", s, count=1, flags=re.S)
if n != 1:
    sys.exit(f"no arguments block in {sys.argv[1]}")
open(sys.argv[2], 'w').write(s)
PY
done
cp "$here/smoke.m" "$work/"
cd "$work"
octave --no-gui --no-window-system -q --eval smoke
