// scheduler.rs — CPM engine
// Enterprise CPM Scheduler — Rust rebuild
//
// Implements:
//   • Topological sort (Kahn's algorithm)
//   • Forward pass  (ES / EF)
//   • Backward pass (LS / LF)
//   • Total Float   TF = LS - ES
//   • Free Float    FF = min(succ ES) - EF
//   • Critical path identification
//   • Full support for FS/SS/FF/SF relationship types with lag
//
// All arithmetic in f64 for future fractional-day support.

use std::collections::{HashMap, HashSet, VecDeque};
use crate::activity::{Activity, CpmResult, RelType};

#[derive(Debug, Clone)]
pub enum SchedulerError {
    CycleDetected(Vec<String>),
    UnknownPredecessor { activity: String, pred: String },
    EmptyProject,
}

impl std::fmt::Display for SchedulerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SchedulerError::CycleDetected(ids) => {
                write!(f, "Cycle detected involving: {}", ids.join(", "))
            }
            SchedulerError::UnknownPredecessor { activity, pred } => {
                write!(f, "Activity '{}' references unknown predecessor '{}'", activity, pred)
            }
            SchedulerError::EmptyProject => write!(f, "No activities to schedule"),
        }
    }
}

pub struct SchedulerResult {
    pub activities: Vec<Activity>,
    pub project_duration: f64,
    pub critical_path: Vec<String>,
    pub warnings: Vec<String>,
}

/// Main entry point: run CPM on a list of activities.
/// Returns a NEW list with all CpmResult fields filled in.
pub fn run_cpm(input: &[Activity]) -> Result<SchedulerResult, SchedulerError> {
    if input.is_empty() {
        return Err(SchedulerError::EmptyProject);
    }

    let mut warnings = Vec::new();

    // --- Build lookup & validate ---
    let id_set: HashSet<&str> = input.iter().map(|a| a.id.as_str()).collect();
    for act in input {
        for pred in &act.predecessors {
            if !id_set.contains(pred.activity_id.as_str()) {
                return Err(SchedulerError::UnknownPredecessor {
                    activity: act.id.clone(),
                    pred: pred.activity_id.clone(),
                });
            }
        }
    }

    // --- Topological sort (Kahn's algorithm) ---
    // Build adjacency list and in-degree map
    let mut successors: HashMap<&str, Vec<&str>> = HashMap::new();
    let mut in_degree: HashMap<&str, usize> = input.iter().map(|a| (a.id.as_str(), 0)).collect();

    for act in input {
        for pred in &act.predecessors {
            successors
                .entry(pred.activity_id.as_str())
                .or_default()
                .push(act.id.as_str());
            *in_degree.get_mut(act.id.as_str()).unwrap() += 1;
        }
    }

    let mut queue: VecDeque<&str> = in_degree
        .iter()
        .filter(|(_, &deg)| deg == 0)
        .map(|(&id, _)| id)
        .collect();

    let mut topo_order: Vec<&str> = Vec::with_capacity(input.len());

    while let Some(id) = queue.pop_front() {
        topo_order.push(id);
        if let Some(succs) = successors.get(id) {
            for &succ in succs {
                let deg = in_degree.get_mut(succ).unwrap();
                *deg -= 1;
                if *deg == 0 {
                    queue.push_back(succ);
                }
            }
        }
    }

    if topo_order.len() != input.len() {
        // Cycle — collect involved IDs
        let scheduled: HashSet<&str> = topo_order.iter().copied().collect();
        let cycle_ids: Vec<String> = input
            .iter()
            .filter(|a| !scheduled.contains(a.id.as_str()))
            .map(|a| a.id.clone())
            .collect();
        return Err(SchedulerError::CycleDetected(cycle_ids));
    }

    // --- Clone activities for mutation ---
    let mut acts: HashMap<String, Activity> = input
        .iter()
        .cloned()
        .map(|a| (a.id.clone(), a))
        .collect();

    // Reset all CPM fields
    for act in acts.values_mut() {
        act.cpm = CpmResult::default();
    }

    // --- Forward pass ---
    for &id in &topo_order {
        let act = acts.get(id).unwrap().clone();
        let es = if act.predecessors.is_empty() {
            0.0
        } else {
            act.predecessors
                .iter()
                .map(|pred| {
                    let p = &acts[&pred.activity_id];
                    let lag = pred.lag as f64;
                    match pred.rel_type {
                        RelType::FS => p.cpm.ef + lag,
                        RelType::SS => p.cpm.es + lag,
                        RelType::FF => p.cpm.ef + lag - act.duration,
                        RelType::SF => p.cpm.es + lag - act.duration,
                    }
                })
                .fold(f64::NEG_INFINITY, f64::max)
                .max(0.0)
        };

        let act = acts.get_mut(id).unwrap();
        act.cpm.es = es;
        act.cpm.ef = es + act.duration;
    }

    // --- Project duration = max EF ---
    let project_duration = acts
        .values()
        .map(|a| a.cpm.ef)
        .fold(f64::NEG_INFINITY, f64::max);

    // --- Backward pass ---
    // Initialize all LF to project_duration
    for act in acts.values_mut() {
        act.cpm.lf = project_duration;
        act.cpm.ls = project_duration - act.duration;
    }

    // Build predecessor map: for each activity, who are its successors?
    // Already have `successors` map of &str -> Vec<&str>

    // Traverse in reverse topo order
    for &id in topo_order.iter().rev() {
        let successors_of: Vec<String> = successors
            .get(id)
            .cloned()
            .unwrap_or_default()
            .iter()
            .map(|s| s.to_string())
            .collect();

        if successors_of.is_empty() {
            // End activity — LF = project_duration already set
        } else {
            let lf = successors_of
                .iter()
                .map(|succ_id| {
                    let succ = &acts[succ_id];
                    // Find the specific predecessor entry for `id` in succ
                    let pred_entry = succ
                        .predecessors
                        .iter()
                        .find(|p| p.activity_id == id)
                        .unwrap();
                    let lag = pred_entry.lag as f64;
                    match pred_entry.rel_type {
                        RelType::FS => succ.cpm.ls - lag,
                        RelType::SS => succ.cpm.ls - lag + acts[id].duration,
                        RelType::FF => succ.cpm.lf - lag,
                        RelType::SF => succ.cpm.lf - lag + acts[id].duration,
                    }
                })
                .fold(f64::INFINITY, f64::min);

            let act = acts.get_mut(id).unwrap();
            act.cpm.lf = lf;
            act.cpm.ls = lf - act.duration;
        }
    }

    // --- Total Float & Critical ---
    for act in acts.values_mut() {
        act.cpm.tf = (act.cpm.ls - act.cpm.es).max(0.0);
        // Tiny float tolerance for critical path
        act.cpm.critical = act.cpm.tf < 1e-9;
    }

    // --- Free Float ---
    // FF(i) = min over all successors j of [ ES(j) - EF(i) - lag(i→j) ]
    // For activities with no successors: FF = LF - EF (same as TF for end acts)
    for &id in &topo_order {
        let successors_of: Vec<String> = successors
            .get(id)
            .cloned()
            .unwrap_or_default()
            .iter()
            .map(|s| s.to_string())
            .collect();

        let ef = acts[id].cpm.ef;
        let lf = acts[id].cpm.lf;

        let ff = if successors_of.is_empty() {
            lf - ef // End node
        } else {
            successors_of
                .iter()
                .map(|succ_id| {
                    let succ = &acts[succ_id];
                    let pred_entry = succ
                        .predecessors
                        .iter()
                        .find(|p| p.activity_id == id)
                        .unwrap();
                    let lag = pred_entry.lag as f64;
                    match pred_entry.rel_type {
                        RelType::FS => succ.cpm.es - ef - lag,
                        RelType::SS => succ.cpm.es - acts[id].cpm.es - lag,
                        RelType::FF => succ.cpm.ef - ef - lag,
                        RelType::SF => succ.cpm.ef - acts[id].cpm.es - lag,
                    }
                })
                .fold(f64::INFINITY, f64::min)
                .max(0.0)
        };

        acts.get_mut(id).unwrap().cpm.ff = ff;
    }

    // --- Assemble output in topo order ---
    let activities: Vec<Activity> = topo_order
        .iter()
        .map(|&id| acts.remove(id).unwrap())
        .collect();

    // --- Critical path (ordered chain, TF=0) ---
    let critical_path: Vec<String> = activities
        .iter()
        .filter(|a| a.cpm.critical)
        .map(|a| a.id.clone())
        .collect();

    if critical_path.is_empty() {
        warnings.push("No critical path found — all activities have float.".to_string());
    }

    Ok(SchedulerResult {
        activities,
        project_duration,
        critical_path,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::activity::Activity;

    fn sample_project() -> Vec<Activity> {
        vec![
            Activity::new("A", "Start", 2.0),
            Activity::new("B", "Foundation", 4.0).with_predecessor("A"),
            Activity::new("C", "Structure", 6.0).with_predecessor("B"),
            Activity::new("D", "Electrical", 3.0).with_predecessor("B"),
            Activity::new("E", "Finish", 2.0)
                .with_predecessor("C")
                .with_predecessor("D"),
        ]
    }

    #[test]
    fn test_critical_path_duration() {
        let result = run_cpm(&sample_project()).unwrap();
        assert!((result.project_duration - 14.0).abs() < 1e-9, "Expected 14 days");
    }

    #[test]
    fn test_critical_activities() {
        let result = run_cpm(&sample_project()).unwrap();
        let crit: Vec<&str> = result
            .critical_path
            .iter()
            .map(String::as_str)
            .collect();
        assert!(crit.contains(&"A"));
        assert!(crit.contains(&"B"));
        assert!(crit.contains(&"C"));
        assert!(crit.contains(&"E"));
        assert!(!crit.contains(&"D"), "D should NOT be critical");
    }

    #[test]
    fn test_float_d() {
        let result = run_cpm(&sample_project()).unwrap();
        let d = result.activities.iter().find(|a| a.id == "D").unwrap();
        // D: ES=6 EF=9, LS=9 LF=12  →  TF = LS-ES = 3
        // E starts at ES=12; D finishes at EF=9  →  FF = 12-9 = 3
        assert!((d.cpm.tf - 3.0).abs() < 1e-9, "TF of D should be 3, got {}", d.cpm.tf);
        assert!((d.cpm.ff - 3.0).abs() < 1e-9, "FF of D should be 3, got {}", d.cpm.ff);
    }

    #[test]
    fn test_es_ef_a() {
        let result = run_cpm(&sample_project()).unwrap();
        let a = result.activities.iter().find(|a| a.id == "A").unwrap();
        assert!((a.cpm.es - 0.0).abs() < 1e-9);
        assert!((a.cpm.ef - 2.0).abs() < 1e-9);
    }

    #[test]
    fn test_cycle_detection() {
        let mut acts = vec![
            Activity::new("X", "X", 1.0).with_predecessor("Y"),
            Activity::new("Y", "Y", 1.0).with_predecessor("X"),
        ];
        assert!(matches!(run_cpm(&acts), Err(SchedulerError::CycleDetected(_))));
    }

    #[test]
    fn test_unknown_predecessor() {
        let acts = vec![
            Activity::new("A", "A", 1.0).with_predecessor("GHOST"),
        ];
        assert!(matches!(run_cpm(&acts), Err(SchedulerError::UnknownPredecessor { .. })));
    }
}
