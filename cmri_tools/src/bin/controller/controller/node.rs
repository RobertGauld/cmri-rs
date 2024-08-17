use cmri::{Address, NodeSort, packet::Data};
use cmri_tools::file;

#[derive(Eq, PartialEq)]
pub struct Node {
    pub(crate) address: Address,
    pub(crate) name: Option<String>,
    pub(crate) sort: NodeSort,
    pub(crate) labels: file::Labels,
    pub(crate) to_initialise: bool,
    pub(crate) inputs: Data,
    pub(crate) outputs: Data,
}

impl Node {
    #[cfg_attr(feature = "experimenter", expect(clippy::large_types_passed_by_value))]
    #[must_use]
    pub fn new(address: Address, sort: NodeSort, name: Option<String>) -> Self {
        Self {
            address,
            name,
            sort,
            labels: file::Labels::default(),
            to_initialise: true,
            inputs: Data::new(sort.configuration().input_bytes() as usize),
            outputs: Data::new(sort.configuration().output_bytes() as usize)
        }
    }
}

impl std::fmt::Debug for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Node")
         .field("name", &self.name)
         .field("address", &self.address)
         .field("sort", &self.sort)
         .field("labels", &self.labels)
         .field("to_initialise", &self.to_initialise)
         .field("inputs", &self.inputs.as_slice())
         .field("outputs", &self.outputs.as_slice())
         .finish()
    }
}

impl From<file::Node> for Node {
    fn from(value: file::Node) -> Self {
        Self {
            address: value.address,
            name: value.name,
            sort: value.sort,
            labels: value.labels,
            to_initialise: true,
            inputs: Data::new(value.sort.configuration().input_bytes() as usize),
            outputs: Data::new(value.sort.configuration().output_bytes() as usize)
        }
    }
}

impl From<&Node> for file::Node {
    fn from(value: &Node) -> Self {
        Self {
            name: value.name.clone(),
            address: value.address,
            sort: value.sort,
            labels: value.labels.clone()
        }
    }
}
