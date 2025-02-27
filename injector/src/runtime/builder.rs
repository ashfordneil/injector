use std::{any::TypeId, collections::HashMap};

use super::Injector;
use crate::derive_api::{INJECTION_REGISTRY, InjectMeta};

pub struct InjectorBuilder {
    injector: Injector,
}

impl InjectorBuilder {
    pub fn new(injector: Injector) -> Self {
        InjectorBuilder { injector }
    }

    pub fn build_the_world(self) -> Injector {
        self.build_from_metadata(INJECTION_REGISTRY.iter().map(|create_meta| create_meta()))
    }

    fn build_from_metadata(mut self, metas: impl Iterator<Item = InjectMeta>) -> Injector {
        let metas = metas
            .map(|meta| (meta.this, meta))
            .collect::<HashMap<_, _>>();

        let sorted = Self::topological_sort(metas);
        for meta in sorted {
            self.injector.build_and_store(&meta);
        }

        self.injector
    }

    fn topological_sort(mut graph: HashMap<TypeId, InjectMeta>) -> Vec<InjectMeta> {
        // As we go through, we will pull items out of the graph and push them onto this list
        let mut creation_order = Vec::new();

        // Find a node that currently isn't queued up to be created
        while let Some(&start) = graph.keys().next() {
            // DFS from this node to find all its deps. Add them to the queue in reverse order.
            enum VisitType {
                BeforeChildren(TypeId),
                AfterChildren(InjectMeta),
            }
            let mut dfs_queue = Vec::new();
            dfs_queue.push(VisitType::BeforeChildren(start));

            while let Some(to_visit) = dfs_queue.pop() {
                match to_visit {
                    VisitType::BeforeChildren(this_type) => {
                        let Some(to_visit_meta) = graph.remove(&this_type) else {
                            // If the node has been removed from the graph, then its already queued up to be
                            // created ...or it's not injectable in the first place, which is unfortunate,
                            // and will lead to an error later.
                            continue;
                        };

                        let children = to_visit_meta.dependencies.clone();
                        dfs_queue.push(VisitType::AfterChildren(to_visit_meta));
                        for child in children {
                            dfs_queue.push(VisitType::BeforeChildren(child));
                        }
                    }
                    VisitType::AfterChildren(this_type) => {
                        creation_order.push(this_type);
                    }
                }
            }
        }

        creation_order
    }
}
