use super::traits::Projectable;
use crate::id::LensId;
use linkme::distributed_slice;

#[distributed_slice]
pub static LENS_REGISTRY: [fn() -> &'static dyn Projectable] = [..];

pub struct LensRegistry;

impl LensRegistry {
    pub fn registered_lenses() -> Vec<&'static dyn Projectable> {
        LENS_REGISTRY.iter().map(|f| f()).collect()
    }

    pub fn get_by_id(lens_id: &LensId) -> Option<&'static dyn Projectable> {
        LENS_REGISTRY
            .iter()
            .find(|f| f().id() == lens_id)
            .map(|f| f())
    }

    pub fn get_by_aggregate(aggregate_id: &str) -> Vec<&'static dyn Projectable> {
        LENS_REGISTRY
            .iter()
            .filter(|f| f().id().as_str().starts_with(aggregate_id))
            .map(|f| f())
            .collect()
    }

    pub fn validate_dependencies() -> Result<(), DependencyCycleError> {
        let graph = Self::dependency_graph();
        if let Some(cycle) = find_cycle(&graph) {
            return Err(DependencyCycleError { cycle });
        }
        Ok(())
    }

    fn dependency_graph() -> Vec<(LensId, Vec<LensId>)> {
        let mut graph = Vec::new();
        for lens in Self::registered_lenses() {
            let id = lens.id().clone();
            let deps = lens.depends_on().to_vec();
            graph.push((id, deps));
        }
        graph
    }
}

fn find_cycle(graph: &[(LensId, Vec<LensId>)]) -> Option<Vec<LensId>> {
    let mut visited = std::collections::HashSet::new();
    let mut rec_stack = std::collections::HashSet::new();
    let mut cycle = Vec::new();

    for (node, _) in graph {
        if !visited.contains(node)
            && dfs_cycle(graph, node, &mut visited, &mut rec_stack, &mut cycle)
        {
            return Some(cycle);
        }
    }
    None
}

fn dfs_cycle(
    graph: &[(LensId, Vec<LensId>)],
    node: &LensId,
    visited: &mut std::collections::HashSet<LensId>,
    rec_stack: &mut std::collections::HashSet<LensId>,
    cycle: &mut Vec<LensId>,
) -> bool {
    if !visited.contains(node) {
        visited.insert(node.clone());
        rec_stack.insert(node.clone());
        cycle.push(node.clone());

        if let Some((_, deps)) = graph.iter().find(|(n, _)| n == node) {
            for dep in deps {
                if !visited.contains(dep) && dfs_cycle(graph, dep, visited, rec_stack, cycle) {
                    return true;
                } else if rec_stack.contains(dep) {
                    if let Some(idx) = cycle.iter().position(|n| n == dep) {
                        cycle.drain(..idx);
                    }
                    return true;
                }
            }
        }
    }

    if let Some(idx) = cycle.iter().position(|n| n == node) {
        cycle.drain(idx..);
    }
    rec_stack.remove(node);
    false
}

#[derive(Debug, Clone)]
pub struct DependencyCycleError {
    pub cycle: Vec<LensId>,
}

impl std::error::Error for DependencyCycleError {}

impl std::fmt::Display for DependencyCycleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Lens dependency cycle detected: {:?}", self.cycle)
    }
}
