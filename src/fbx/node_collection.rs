use multimap::MultiMap;
use crate::fbx::node::NodeRecord;
use crate::fbx::node_collection::Error::{NoSuchNode, MultipleValuesExist};

#[derive(Debug)]
pub struct NodeCollection {
    nodes: MultiMap<String, NodeRecord>,
}

pub enum Error {
    MultipleValuesExist,
    NoSuchNode
}

impl NodeCollection {
    pub fn new() -> Self {
        NodeCollection {
            nodes: MultiMap::new()
        }
    }

    pub fn insert(&mut self, node: NodeRecord) {
        self.nodes.insert(node.name.clone(), node);
    }

    pub fn get(&self, name: &str) -> Result<&NodeRecord, Error> {
        match self.nodes.get(name) {
            Some(x) => Ok(x),
            None => Err(NoSuchNode)
        }
    }

    pub fn get_multiple(&self, name: &str) -> Option<&Vec<NodeRecord>> {
        self.nodes.get_vec(name)
    }
}