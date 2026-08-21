use super::graph::LinkGraph;
use std::collections::HashMap;
use tracing::{debug, info};

pub struct PageRankCalculator {
    damping_factor: f64,
    iterations: usize,
    convergence_threshold: f64,
}

impl PageRankCalculator {
    pub fn new() -> Self {
        Self {
            damping_factor: 0.85,
            iterations: 30,
            convergence_threshold: 0.0001,
        }
    }

    pub fn calculate(&self, graph: &LinkGraph) -> HashMap<String, f64> {
        let n = graph.node_count() as f64;
        if n == 0.0 {
            return HashMap::new();
        }

        info!("Calculating page rank for {} nodes", n);

        // initialize all pages with equal rank
        let initial_rank = 1.0 / n;

        let mut ranks: HashMap<String, f64> = graph
            .nodes
            .iter()
            .map(|url| (url.clone(), initial_rank))
            .collect();

        // iterative calculation
        for iteration in 0..self.iterations {
            let mut new_ranks = HashMap::new();
            let mut total_diff = 0.0;

            for url in &graph.nodes {
                let mut rank_sum = 0.0;

                // get all pages linking to this page
                if let Some(inbound) = graph.inbounds.get(url) {
                    for source_url in inbound {
                        let source_rank = ranks.get(source_url).unwrap_or(&initial_rank);
                        let source_outbound = graph.outbound_count(source_url) as f64;

                        if source_outbound > 0.0 {
                            rank_sum += source_rank / source_outbound;
                        }
                    }
                }

                // apply damping factor
                let new_rank = (1.0 - self.damping_factor) / n + self.damping_factor * rank_sum;

                // Track convergence
                let old_rank = ranks.get(url).unwrap_or(&initial_rank);
                total_diff += (new_rank - old_rank).abs();

                new_ranks.insert(url.clone(), new_rank);
            }
            ranks = new_ranks;

            debug!("iterations : {} : diff = {:.6}", iteration + 1, total_diff);

            // check convergence
            if total_diff < self.convergence_threshold {
                info!("Pagerank converged at iteration {}", iteration + 1);
                break;
            }
        }

        // Normalize ranks (sum to 1.0)
        let sum: f64 = ranks.values().sum();

        if sum > 0.0 {
            for rank in ranks.values_mut() {
                *rank /= sum;
            }
        }
        ranks
    }

    pub fn get_top_pages(&self, ranks: &HashMap<String, f64>, limit: usize) -> Vec<(String, f64)> {
        let mut ranked: Vec<_> = ranks
            .iter()
            .map(|(url, rank)| (url.clone(), *rank))
            .collect();

        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        ranked.into_iter().take(limit).collect()
    }
}
