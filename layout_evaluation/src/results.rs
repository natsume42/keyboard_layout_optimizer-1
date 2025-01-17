//! The `results` module contains structs representing the results of metric evaluations.

use serde::{Deserialize, Serialize};

/// The `NormalizationType` specifies how the total cost of a metric evaluation shall be normalized.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", content = "value")]
#[serde(rename_all = "snake_case")]
pub enum NormalizationType {
    /// Divide the metric result's cost value by a fixed value.
    Fixed(f64),
    /// Divide the metric result's cost value by the sum of the ngram weights that could be mapped by the layout and a given fixed value.
    WeightFound(f64),
    /// Divide the metric result's cost value by the sum of all ngram weights and a given fixed value.
    WeightAll(f64),
}

/// Specify which data a metric operates on.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum MetricType {
    Layout,
    Unigram,
    Bigram,
    Trigram,
}

/// Describes the result of an individual metric evaluation.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MetricResult {
    /// Name of the metric.
    pub name: String,
    /// Resulting total cost value (not normalized).
    pub cost: f64,
    /// An optional message that may contain additional details.
    pub message: Option<String>,
    /// The weight that shall be used when aggregating all metrics.
    pub weight: f64,
    /// The normalization type to apply.
    pub normalization: NormalizationType,
}

/// Describes the normalized results of an individual metric evaluation
/// taking into account the total found/not found ngram weights.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NormalizedMetricResult {
    pub core: MetricResult,
    pub weighted_cost: f64,
    pub unweighted_cost: f64,
}

/// Describes a list of metric evaluation results of the same `MetricType`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MetricResults {
    /// Type of the metric, i.e. which data the metrics operated on.
    pub metric_type: MetricType,
    /// The total amount of weight (ngram frequencies) from ngrams that could be mapped by the layout.
    pub found_weight: f64,
    /// The total amount of weight (ngram frequencies) from ngrams that contained symbols that coult not be mapped by the layout.
    pub not_found_weight: f64,
    /// A list of the individual metric results.
    pub metric_costs: Vec<NormalizedMetricResult>,
}

impl std::fmt::Display for MetricResults {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{:?} metrics:", self.metric_type)?;

        if self.metric_type != MetricType::Layout {
            writeln!(
                f,
                "  Not found: {:.4}% of {:.4}",
                100.0 * self.not_found_weight / (self.not_found_weight + self.found_weight),
                self.not_found_weight + self.found_weight
            )?;
        }
        for metric_cost in self.metric_costs.iter() {
            writeln!(
                f,
                "  {:>9.4} (weighted: {:>9.4}) {:<35} | {}",
                metric_cost.unweighted_cost,
                metric_cost.weighted_cost,
                metric_cost.core.name,
                metric_cost.core.message.as_ref().unwrap_or(&"".to_string()),
            )?;
        }
        Ok(())
    }
}

impl MetricResults {
    pub fn new(metric_type: MetricType, found_weight: f64, not_found_weight: f64) -> Self {
        Self {
            metric_type,
            found_weight,
            not_found_weight,
            metric_costs: Vec::new(),
        }
    }

    pub fn add_result(&mut self, metric_cost: MetricResult) {
        let weighted_cost = self.compute_metric_cost(&metric_cost, true, true);
        let unweighted_cost = self.compute_metric_cost(&metric_cost, true, false);
        self.metric_costs.push(NormalizedMetricResult {
            core: metric_cost,
            weighted_cost,
            unweighted_cost,
        })
    }

    /// Normalize a metric's cost value with given normalization strategy.
    fn normalize_value(&self, val: f64, normalization_type: &NormalizationType) -> f64 {
        let mut res = match normalization_type {
            NormalizationType::Fixed(t) => val / t,
            NormalizationType::WeightFound(t) => val / (t * self.found_weight),
            NormalizationType::WeightAll(t) => {
                val / (t * self.found_weight + self.not_found_weight)
            }
        };

        // instead of NAN, we prefer having 0.0 cost
        if res.is_nan() {
            res = 0.0
        }

        return res
    }

    /// Helper function for weighting and normalizing individual metric's results.
    fn compute_metric_cost(
        &self,
        metric_cost: &MetricResult,
        normalize: bool,
        weight: bool,
    ) -> f64 {
        let cost = match weight {
            true => metric_cost.weight * metric_cost.cost,
            false => metric_cost.cost,
        };

        match normalize {
            true => self.normalize_value(cost, &metric_cost.normalization),
            false => cost,
        }
    }

    /// Helper function for aggregating all individual metrics' results to a total value.
    fn aggregate_metric_costs(&self, normalize: bool, weight: bool) -> f64 {
        self.metric_costs.iter().fold(0.0, |acc, metric_cost| {
            acc + self.compute_metric_cost(&metric_cost.core, normalize, weight)
        })
    }

    /// Compute the weighted and normalized total cost of all metrics.
    pub fn total_cost(&self) -> f64 {
        self.aggregate_metric_costs(true, true)
    }

    /// Compute the weighted but not normalized total cost of all metrics.
    pub fn unnormalized_total_cost(&self) -> f64 {
        self.aggregate_metric_costs(false, true)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EvaluationResult {
    individual_results: Vec<MetricResults>,
}

impl std::fmt::Display for EvaluationResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.individual_results
            .iter()
            .fold(Ok(()), |acc, results| {
                acc.and_then(|_| writeln!(f, "{}", results))
            })?;

        writeln!(
            f,
            "Cost: {:.4} (optimization score: {})",
            self.total_cost(),
            self.optimization_score()
        )?;

        Ok(())
    }
}

impl EvaluationResult {
    pub fn new(individual_results: Vec<MetricResults>) -> Self {
        Self { individual_results }
    }

    pub fn total_cost(&self) -> f64 {
        let mut cost = 0.0;
        for mc in self
            .individual_results
            .iter()
            .filter(|mc| !mc.metric_costs.is_empty())
        {
            cost += mc.total_cost();
        }

        cost
    }

    pub fn optimization_score(&self) -> usize {
        (1e8 / self.total_cost()) as usize
    }

    pub fn iter(&self) -> std::slice::Iter<'_, MetricResults> {
        self.individual_results.iter()
    }
}
