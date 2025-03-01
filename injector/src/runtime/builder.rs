use std::any::TypeId;

use multimap::MultiMap;

use super::Injector;
use crate::derive_api::{BINDING_REGISTRY, INJECTION_REGISTRY, InjectMeta};

pub struct InjectorBuilder {
    injector: Injector,
}

impl InjectorBuilder {
    pub fn new(injector: Injector) -> Self {
        InjectorBuilder { injector }
    }

    pub fn build_the_world(self) -> Injector {
        let metadata_for_normal_types = INJECTION_REGISTRY.iter().map(|create_meta| create_meta());

        let all_trait_bindings = BINDING_REGISTRY
            .iter()
            .map(|create_binding| create_binding())
            .map(|binding| (binding.trait_object, binding))
            .collect::<MultiMap<_, _>>();

        let metadata_for_bindings = all_trait_bindings.into_iter().flat_map(|(_, bindings)| {
            if bindings.len() > 1 || bindings[0].is_multi_binding {
                if bindings.iter().any(|binding| !binding.is_multi_binding) {
                    panic!("Error registering implementations for {}. Found a mix of #[binding] and #[multi_binding] annotations", bindings[0].name);
                }
            }

            bindings.into_iter().map(|binding| {
                InjectMeta {
                    this: binding.trait_object,
                    name: binding.name,
                    dependencies: vec![binding.impl_type],
                    create: binding.create,
                    is_multi_binding: binding.is_multi_binding,
                }
            })
        });

        self.build_from_metadata(metadata_for_normal_types.chain(metadata_for_bindings))
    }

    fn build_from_metadata(mut self, metas: impl Iterator<Item = InjectMeta>) -> Injector {
        let metas = metas
            .map(|meta| (meta.this, meta))
            .collect::<MultiMap<_, _>>();

        let sorted = Self::topological_sort(metas);
        for meta in sorted {
            self.injector.build_and_store(&meta);
        }

        self.injector
    }

    fn topological_sort(mut graph: MultiMap<TypeId, InjectMeta>) -> Vec<InjectMeta> {
        // As we go through, we will pull items out of the graph and push them onto this list
        let mut creation_order = Vec::new();

        // Find a node that currently isn't queued up to be created
        while let Some(&start) = graph.keys().next() {
            // DFS from this node to find all its deps. Add them to the queue in reverse order.
            enum VisitType {
                BeforeChildren(TypeId),
                AfterChildren(Vec<InjectMeta>),
            }
            let mut dfs_queue = Vec::new();
            dfs_queue.push(VisitType::BeforeChildren(start));

            while let Some(to_visit) = dfs_queue.pop() {
                match to_visit {
                    VisitType::BeforeChildren(this_type) => {
                        let Some(to_visit_metas) = graph.remove(&this_type) else {
                            // If the node has been removed from the graph, then its already queued up to be
                            // created ...or it's not injectable in the first place, which is unfortunate,
                            // and will lead to an error later.
                            continue;
                        };

                        let children = to_visit_metas
                            .iter()
                            .flat_map(|meta| meta.dependencies.iter())
                            .copied()
                            .collect::<Vec<_>>();
                        dfs_queue.push(VisitType::AfterChildren(to_visit_metas));
                        for child in children {
                            dfs_queue.push(VisitType::BeforeChildren(child));
                        }
                    }
                    VisitType::AfterChildren(this_type) => {
                        creation_order.extend(this_type);
                    }
                }
            }
        }

        creation_order
    }
}
