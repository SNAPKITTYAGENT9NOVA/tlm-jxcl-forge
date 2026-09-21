//! Rayon-backed parallel element access, gated behind the `parallel`
//! feature (which also turns on `ndarray`'s own `rayon` feature).

use ndarray::parallel::prelude::*;

use crate::tensor::Tensor;

impl<T: Send + Sync + Clone> Tensor<T> {
    /// Apply `f` to every element in place, in parallel.
    pub fn par_mapv_inplace(&mut self, f: impl Fn(T) -> T + Send + Sync) {
        self.data.par_mapv_inplace(f);
    }

    /// Apply `f` to every element, in parallel, producing a new tensor of
    /// the same shape.
    pub fn par_mapv(&self, f: impl Fn(T) -> T + Send + Sync) -> Tensor<T> {
        let mut out = self.data.clone();
        out.par_mapv_inplace(f);
        Tensor::from_array(out)
    }

    /// Run `f` over every top-level (axis-0) sub-view in parallel -- the
    /// natural unit of parallel work for a batch of matrices, images,
    /// samples, etc.
    pub fn par_for_each_outer(&self, f: impl Fn(ndarray::ArrayViewD<'_, T>) + Send + Sync) {
        self.data
            .axis_iter(ndarray::Axis(0))
            .into_par_iter()
            .for_each(f);
    }
}
