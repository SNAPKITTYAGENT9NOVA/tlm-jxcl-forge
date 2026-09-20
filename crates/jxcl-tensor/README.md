# jxcl-tensor: High-Performance Tensor Library for JXCL

A production-grade tensor library built on `ndarray`, providing ergonomic N-dimensional array operations with zero-cost abstractions, broadcasting, linear algebra, and reduction operations.

## Features

- **Arbitrary rank tensors**: 1-D vectors, 2-D matrices, 3-D+ tensors with unified API
- **Zero-copy views**: Work with `ArrayView` and `ArrayViewMut` for efficient memory usage
- **Broadcasting**: NumPy-style automatic broadcasting for compatible shapes
- **Element-wise operations**: Add, subtract, multiply, divide with automatic broadcast
- **Reductions**: sum, mean, max, min, argmax, argmin along axes
- **Linear algebra**: matmul, transpose, reshape, determinant, trace
- **Shape manipulation**: reshape, squeeze, expand_dims, flatten, permute_axes
- **Memory control**: C-order (row-major) and F-order (column-major) layout awareness
- **Type-safe**: Fully idiomatic Rust with `Result<T>` error handling
- **Extensible**: Clean trait-based design for custom operations

## Quick Start

```rust
use jxcl_tensor::prelude::*;

// Create tensors
let a = Tensor::<f32>::zeros(&[2, 3]);
let b = Tensor::<f32>::ones(&[2, 3]);

// Element-wise operations with broadcasting
let c = a.add(&b)?;  // Shape [2, 3]

// Reductions
let sum = c.sum()?;           // Scalar f32
let summed = c.sum_axis(0)?;  // Shape [3]
let max = c.max()?;           // Scalar f32

// Linear algebra
let eye = Tensor::<f32>::eye(3)?;
let transposed = c.transpose();

// Shape manipulation
let reshaped = c.reshape(&[6])?;
let flattened = c.flatten();
```

## Architecture

### Module Structure

- **`tensor.rs`**: Core `Tensor<T>` type and basic operations (creation, shape manipulation, transpose, permute)
- **`ops.rs`**: Broadcasting-aware arithmetic (add, sub, mul, div) and element-wise operations
- **`reduce.rs`**: Reduction operations (sum, mean, max, min, argmax along axes)
- **`linalg.rs`**: Linear algebra (matmul, trace, determinant, Frobenius norm)
- **`error.rs`**: Error types with rich context for debugging
- **`prelude.rs`**: Convenient re-exports

### Design Principles

1. **Zero-copy by default**: Operations use views and work with borrowed data when possible
2. **NumPy semantics**: Broadcasting rules and axis conventions match NumPy for familiarity
3. **Fail-safe**: All operations return `Result<T>` for proper error handling
4. **Idiomatic Rust**: Leverages type system, traits, and ownership for memory safety
5. **Extensible**: Trait-based design allows custom operations without modifying core library

## Broadcasting Rules

The library follows NumPy's broadcasting convention:

1. Dimensions of size 1 are stretched to match the other operand
2. Missing dimensions are treated as size 1
3. Incompatible dimensions raise an error

Examples:
- `[2, 3] + [3]` → `[2, 3]` (dimension 2 broadcast)
- `[2, 3] + [1, 3]` → `[2, 3]` (dimension 1 broadcast)
- `[2, 3] + [3, 4]` → Error (incompatible)

## Performance Notes

### Memory Layout

- **C-order (row-major)**: Default, most efficient for row operations
- **F-order (column-major)**: Better for column operations; use `is_f_contiguous()`
- Check with `is_c_contiguous()`, `is_f_contiguous()`, or `is_contiguous()`

### Allocation Avoidance

```rust
// Good: zero-copy operations
let reshaped = t.reshape(&[6])?;  // Reuses data if layout permits

// Careful: creates owned copy
let transposed = t.transpose();   // Creates new owned array
```

### Parallelization

Use the `parallel` feature for `rayon`-based parallelization:

```toml
[dependencies]
jxcl-tensor = { version = "0.1", features = ["parallel"] }
```

BLAS acceleration is available via the `blas` feature (future enhancement).

## Interoperability

The library is designed to integrate with the broader Rust numerical ecosystem:

- **Direct ndarray access**: Use `.as_array()` and `.into_array()` to convert
- **Future BLAS backends**: Matmul and other operations can dispatch to BLAS libraries
- **External libraries**: Export tensors to `nalgebra`, `candle`, `burn`, etc.

## Limitations & Future Work

### Current Limitations

- Determinant only for 1×1 and 2×2 matrices (no full LU decomposition yet)
- No BLAS/LAPACK backends yet (pure Rust implementations)
- No automatic differentiation hooks yet
- Batched operations (4D+ matmul) not yet supported

### Roadmap

- [ ] Full LU, QR, SVD decompositions
- [ ] BLAS/LAPACK bindings (optional feature)
- [ ] Eigenvalue decomposition
- [ ] Rayon parallelization for reductions
- [ ] Automatic differentiation API
- [ ] GPU backends (CUDA/Metal) via optional features
- [ ] Einsum and other einops-style operations
- [ ] Tensor contractions with Index notation

## Safety & Correctness

### Panic Safety

- All bounds checks are explicit; out-of-bounds access returns `Result::Err`
- No unsafe code in public API (`#![forbid(unsafe_code)]`)
- Memory safety guaranteed by Rust's ownership system

### Numerical Stability

- Reductions use careful accumulation to minimize rounding errors
- Floating-point comparisons use `PartialOrd` with respect to user's tolerance
- Document numerical gotchas in operation-specific docstrings

## Examples

### Matrix Multiplication

```rust
use jxcl_tensor::prelude::*;

let a = Tensor::<f32>::new(ndarray::Array::from_shape_vec(
    ndarray::IxDyn(&[2, 3]),
    vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
)?);

let b = Tensor::<f32>::new(ndarray::Array::from_shape_vec(
    ndarray::IxDyn(&[3, 2]),
    vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
)?);

let c = a.matmul(&b)?;
assert_eq!(c.shape(), &[2, 2]);
```

### Broadcasting Addition

```rust
let a = Tensor::<f32>::zeros(&[2, 3]);
let b = Tensor::<f32>::ones(&[1, 3]);
let c = a.add(&b)?;  // Broadcasts b to [2, 3]
assert_eq!(c.shape(), &[2, 3]);
```

### Reductions

```rust
let t = Tensor::<i32>::new(ndarray::Array::from_shape_vec(
    ndarray::IxDyn(&[2, 3]),
    vec![1, 2, 3, 4, 5, 6],
)?);

let sum = t.sum()?;              // 21
let summed_axis = t.sum_axis(0)?; // [5, 7, 9]
let max = t.max()?;               // 6
let argmax = t.argmax()?;         // 5
```

## License

Licensed under AGPL-3.0-only OR LicenseRef-Commercial. See LICENSE-AGPL and LICENSE-COMMERCIAL.

## Contributing

This library is part of the JXCL project. For contributions, see the main repository guidelines.
