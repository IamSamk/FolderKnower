//! Deterministic Louvain community detection.
//!
//! Nodes are visited in index order and ties go to the lowest community id, so the
//! same graph always yields the same partition (no RNG involved).

use std::collections::BTreeMap;

/// Returns a community label per node (labels are dense, ordered by first member).
pub fn louvain(n: usize, edges: &[(usize, usize, f64)], resolution: f64) -> Vec<usize> {
    let mut membership: Vec<usize> = (0..n).collect();
    let mut cur_n = n;
    let mut cur_edges: Vec<(usize, usize, f64)> = edges.to_vec();
    for _level in 0..6 {
        let local = one_level(cur_n, &cur_edges, resolution);
        let (relabelled, k) = relabel(&local);
        if k == cur_n {
            break;
        }
        for m in membership.iter_mut() {
            *m = relabelled[*m];
        }
        // Aggregate
        let mut agg: BTreeMap<(usize, usize), f64> = BTreeMap::new();
        for &(a, b, w) in &cur_edges {
            let (ca, cb) = (relabelled[a], relabelled[b]);
            let key = if ca <= cb { (ca, cb) } else { (cb, ca) };
            *agg.entry(key).or_insert(0.0) += w;
        }
        cur_edges = agg.into_iter().map(|((a, b), w)| (a, b, w)).collect();
        cur_n = k;
    }
    relabel(&membership).0
}

fn relabel(labels: &[usize]) -> (Vec<usize>, usize) {
    let mut map = BTreeMap::new();
    let mut out = Vec::with_capacity(labels.len());
    for &l in labels {
        let next = map.len();
        out.push(*map.entry(l).or_insert(next));
    }
    (out, map.len())
}

fn one_level(n: usize, edges: &[(usize, usize, f64)], resolution: f64) -> Vec<usize> {
    let mut adj: Vec<BTreeMap<usize, f64>> = vec![BTreeMap::new(); n];
    let mut k = vec![0.0f64; n];
    let mut m2 = 0.0;
    for &(a, b, w) in edges {
        if a == b {
            *adj[a].entry(a).or_insert(0.0) += w;
            k[a] += 2.0 * w;
        } else {
            *adj[a].entry(b).or_insert(0.0) += w;
            *adj[b].entry(a).or_insert(0.0) += w;
            k[a] += w;
            k[b] += w;
        }
        m2 += 2.0 * w;
    }
    let mut comm: Vec<usize> = (0..n).collect();
    if m2 == 0.0 {
        return comm;
    }
    let mut tot = k.clone();
    for _ in 0..32 {
        let mut moved = false;
        for i in 0..n {
            let old = comm[i];
            tot[old] -= k[i];
            let mut to_comm: BTreeMap<usize, f64> = BTreeMap::new();
            for (&j, &w) in &adj[i] {
                if j != i {
                    *to_comm.entry(comm[j]).or_insert(0.0) += w;
                }
            }
            let gain = |c: usize, w_in: f64| w_in - resolution * tot[c] * k[i] / m2;
            let mut best = old;
            let mut best_gain = gain(old, *to_comm.get(&old).unwrap_or(&0.0));
            for (&c, &w_in) in &to_comm {
                let g = gain(c, w_in);
                if g > best_gain + 1e-12 || ((g - best_gain).abs() <= 1e-12 && c < best) {
                    best = c;
                    best_gain = g;
                }
            }
            tot[best] += k[i];
            if best != old {
                comm[i] = best;
                moved = true;
            }
        }
        if !moved {
            break;
        }
    }
    comm
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_cliques_split() {
        let mut e = Vec::new();
        for a in 0..4 {
            for b in a + 1..4 {
                e.push((a, b, 1.0));
                e.push((a + 4, b + 4, 1.0));
            }
        }
        e.push((0, 4, 0.1));
        let c = louvain(8, &e, 1.0);
        assert_eq!(c[0], c[3]);
        assert_eq!(c[4], c[7]);
        assert_ne!(c[0], c[4]);
        assert_eq!(c, louvain(8, &e, 1.0));
    }
}
