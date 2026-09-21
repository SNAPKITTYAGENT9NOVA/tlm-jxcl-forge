import Lake
open Lake DSL

package «MathlibMatrixFormalization» where
  version := "0.1.0"
  precompileModules := true

@[default_target]
lean_lib MathlibMatrixFormalization where
  globs := [.submodules "MathlibMatrixFormalization"]

require mathlib from git "https://github.com/leanprover-community/mathlib4.git"
