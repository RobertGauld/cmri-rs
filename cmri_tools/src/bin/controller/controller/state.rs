use cmri_tools::file;
use super::Node;

pub struct State {
    pub(crate) nodes: [Option<Node>; 128],
    pub(crate) egui_ctx: egui::Context
}

impl State {
    /// Reset the state back to default.
    pub fn reset(&mut self) {
        for node in &mut self.nodes {
            *node = None;
        }
    }

    pub fn load_nodes(&mut self, mut nodes: Vec<Option<file::Node>>) {
        for node in nodes.iter_mut().filter_map(Option::take) {
            let index = node.address.as_node_address() as usize;
            match self.nodes[index].as_mut() {
                None => {
                    self.nodes[index] = Some(node.into());
                },
                Some(a) => {
                    if node.name.is_some() {
                        a.name = node.name;
                    }
                }
            }
        }
    }

    pub fn available_node_addresses(&self) -> Vec<u8> {
        (0..128).filter(|i| self.nodes[*i as usize].is_none()).collect()
    }
}

impl std::fmt::Debug for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("State")
         .field("nodes", &self.nodes)
         .finish_non_exhaustive()
    }
}

impl Default for State {
    fn default() -> Self {
        Self {
            nodes: std::array::from_fn(|_| None),
            egui_ctx: egui::Context::default()
        }
    }
}


#[allow(clippy::missing_panics_doc, reason="tests")]
#[cfg(test)]
mod tests {
    use cmri::{Address, NodeSort, packet::Data};
    use super::*;

    #[test]
    fn default() {
        let default = State::default();
        assert_eq!(default.nodes.len(), 128);
        for node in &default.nodes {
            assert!(node.is_none());
        }
    }

    #[test]
    fn reset() {
        let mut state = State::default();
        state.nodes[5] = Some(Node {
            address: Address::try_from_node_address(0).unwrap(),
            name: None,
            sort: NodeSort::try_new_smini(0, [0; 6]).unwrap(),
            labels: file::Labels::default(),
            to_initialise: false,
            inputs: Data::default(),
            outputs: Data::default()
        });

        state.reset();
        assert_eq!(state.nodes, State::default().nodes);
    }

    #[test]
    fn load_nodes() {
        let sort = cmri::NodeSort::try_new_smini(0, [0; 6]).unwrap();
        let nodes = vec![
            Some(file::Node {
                address: Address::try_from_node_address(10).unwrap(),
                name: Some(String::from("Test node 1")),
                sort: cmri::NodeSort::try_new_smini(0, [0; 6]).unwrap(),
                labels: file::Labels::default()
            }),
            Some(file::Node {
                address: Address::try_from_node_address(20).unwrap(),
                name: Some(String::from("Test node 2")),
                sort: cmri::NodeSort::try_new_smini(0, [3; 6]).unwrap(), // Must be different to sort variable
                labels: file::Labels::default()
            })
        ];
        let mut state = State::default();
        state.nodes[10] = Some(Node {
            address: Address::try_from_node_address(10).unwrap(),
            name: None,
            sort,
            labels: file::Labels::default(),
            to_initialise: true,
            inputs: Data::try_from(&[1]).unwrap(),
            outputs: Data::try_from(&[2]).unwrap()
        });
        state.nodes[20] = Some(Node {
            address: Address::try_from_node_address(10).unwrap(),
            name: Some(String::from("will change")),
            sort,
            labels: file::Labels::default(),
            to_initialise: true,
            inputs: Data::new(3),
            outputs: Data::new(6)
        });
        state.load_nodes(nodes);

        assert_eq!(state.nodes[10].as_mut().unwrap().name, Some(String::from("Test node 1")));
        assert_eq!(state.nodes[10].as_mut().unwrap().sort, sort);
        assert_eq!(state.nodes[10].as_mut().unwrap().inputs.as_slice(), [1].as_slice());  // Should be untouched
        assert_eq!(state.nodes[10].as_mut().unwrap().outputs.as_slice(), [2].as_slice()); // Should be untouched

        assert_eq!(state.nodes[20].as_mut().unwrap().name, Some(String::from("Test node 2")));
        assert_eq!(state.nodes[20].as_mut().unwrap().sort, sort); // Should not be replaced as it was present
    }
}
