//! `serde` support, gated behind the `serde` feature (which also turns on
//! `ndarray`'s own `serde` feature). A `Tensor<T>` serializes exactly as
//! its underlying `ndarray::ArrayD<T>` would.

use crate::tensor::Tensor;

impl<T: serde::Serialize> serde::Serialize for Tensor<T> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.data.serialize(serializer)
    }
}

impl<'de, T: serde::Deserialize<'de>> serde::Deserialize<'de> for Tensor<T> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        ndarray::ArrayD::deserialize(deserializer).map(Tensor::from_array)
    }
}
