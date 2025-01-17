//! The `metrics` module provides a trait for bigram metrics.
use keyboard_layout::layout::{LayerKey, Layout};
use priority_queue::DoublePriorityQueue;
use ordered_float::OrderedFloat;

pub mod asymmetric_bigrams;
pub mod finger_repeats;
pub mod finger_repeats_lateral;
pub mod finger_repeats_top_bottom;
pub mod line_changes;
pub mod manual_bigram_penalty;
pub mod movement_pattern;
pub mod no_handswitch_after_unbalancing_key;
pub mod unbalancing_after_neighboring;

const SHOW_WORST: bool = true;
const N_WORST: usize = 3;

/// BigramMetric is a trait for metrics that iterates over weighted bigrams.
pub trait BigramMetric: Send + Sync + BigramMetricClone + std::fmt::Debug {
    /// Return the name of the metric.
    fn name(&self) -> &str;

    /// Compute the cost of one bigram (if that is possible, otherwise, return `None`).
    #[inline(always)]
    fn individual_cost(
        &self,
        _key1: &LayerKey,
        _key2: &LayerKey,
        _weight: f64,
        _total_weight: f64,
        _layout: &Layout,
    ) -> Option<f64> {
        None
    }

    /// Compute the total cost for the metric.
    fn total_cost(
        &self,
        bigrams: &[((&LayerKey, &LayerKey), f64)],
        // total_weight is optional for performance reasons (it can be computed from bigrams).
        total_weight: Option<f64>,
        layout: &Layout,
    ) -> (f64, Option<String>) {
        let total_weight = total_weight.unwrap_or_else(|| bigrams.iter().map(|(_, w)| w).sum());
        let cost_iter = bigrams.iter().filter_map(|(bigram, weight)| {
            let res = self.individual_cost(bigram.0, bigram.1, *weight, total_weight, layout);

            res.map(|c| (bigram, c))
        });

        let (total_cost, msg) = if SHOW_WORST {
            let (total_cost, cost_with_mod, worst) = cost_iter.fold(
                (0.0, 0.0, DoublePriorityQueue::new()),
                |(mut total_cost, mut cost_with_mod, mut worst), (bigram, cost)| {
                    total_cost += cost;

                    if bigram.0.is_modifier || bigram.1.is_modifier {
                        cost_with_mod += cost;
                    };

                    worst.push(
                        (bigram.0.symbol, bigram.1.symbol),
                        OrderedFloat(cost),
                    );
                    if worst.len() > N_WORST {
                        worst.pop_min();
                    }

                    (total_cost, cost_with_mod, worst)
                },
            );

            let mut msgs = Vec::new();

            let worst_msgs: Vec<String> = worst
                .into_sorted_iter()
                .rev()
                .filter(|(_, cost)| cost.into_inner() > 0.0)
                .map(|(bigram, cost)| {
                    format!(
                        "{}{} ({:>5.2}%)",
                        bigram.0.to_string().escape_debug(),
                        bigram.1.to_string().escape_debug(),
                        100.0 * cost.into_inner() / total_cost,
                    )
                })
                .collect();
            if !worst_msgs.is_empty() {
                msgs.push(format!("Worst bigrams: {}", worst_msgs.join(", ")))
            }

            if total_cost > 0.0 {
                msgs.push(format!(
                    "{:>5.2}% of cost involved a modifier",
                    100.0 * cost_with_mod / total_cost,
                ));
            }

            let msg = Some(msgs.join(";  "));

            (total_cost, msg)
        } else {
            let total_cost: f64 = cost_iter.map(|(_, c)| c).sum();

            (total_cost, None)
        };

        (total_cost, msg)
    }
}

impl Clone for Box<dyn BigramMetric> {
    fn clone(&self) -> Box<dyn BigramMetric> {
        self.clone_box()
    }
}

/// Helper trait for realizing clonability for `Box<dyn BigramMetric>`.
pub trait BigramMetricClone {
    fn clone_box(&self) -> Box<dyn BigramMetric>;
}

impl<T> BigramMetricClone for T
where
    T: 'static + BigramMetric + Clone,
{
    fn clone_box(&self) -> Box<dyn BigramMetric> {
        Box::new(self.clone())
    }
}
