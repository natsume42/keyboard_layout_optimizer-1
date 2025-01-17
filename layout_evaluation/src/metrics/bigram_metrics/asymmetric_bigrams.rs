//! The bigram metric `AsymmetricBigram` metric assigns a cost to each bigram
//! for which the two keys are not symmetrical (thumbs are excluded).
//!
//! *Note*: In contrast to ArneBab's version, thumbs are excluded.

use super::BigramMetric;

use keyboard_layout::{
    key::Finger,
    layout::{LayerKey, Layout},
};

use serde::Deserialize;

#[derive(Clone, Deserialize, Debug)]
pub struct Parameters {}

#[derive(Clone, Debug)]
pub struct AsymmetricBigrams {}

impl AsymmetricBigrams {
    pub fn new(_params: &Parameters) -> Self {
        Self {}
    }
}

impl BigramMetric for AsymmetricBigrams {
    fn name(&self) -> &str {
        "Asymmetric Bigrams"
    }

    #[inline(always)]
    fn individual_cost(
        &self,
        k1: &LayerKey,
        k2: &LayerKey,
        weight: f64,
        total_weight: f64,
        _layout: &Layout,
    ) -> Option<f64> {
        match k1.key.symmetry_index != k2.key.symmetry_index
            && k1.key.finger != Finger::Thumb
            && k2.key.finger != Finger::Thumb
        {
            true => {
                // log the top asymmetric bigram scorers (with weight > 1%)
                if weight > 0.01 * total_weight {
                    log::trace!(
                        "Bigram: {:>3}{:<3}, cost: {:.4}",
                        k1.symbol.escape_debug().to_string(),
                        k2.symbol.escape_debug().to_string(),
                        weight,
                    );
                }

                Some(weight)
            }
            false => Some(0.0),
        }
    }
}
