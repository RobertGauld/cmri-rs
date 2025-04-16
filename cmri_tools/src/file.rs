//! Common methods for saving/loading data.

use std::collections::HashMap;
use anyhow::Context;
use cmri::{Address, NodeSort};

#[derive(Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
struct File {
    #[serde(serialize_with = "File::serialize_nodes", deserialize_with = "File::deserialize_nodes")]
    nodes: Vec<Option<Node>>
}

impl File {
    /// Load a file.
    ///
    /// # Errors
    ///
    /// * If the file can't be read.
    /// * If the JSON can't be parsed.
    /// * If the JSON contains invalid data.
    pub fn load(path: &std::path::Path) -> anyhow::Result<Self> {
        #[allow(clippy::unnecessary_debug_formatting, reason = "False positive: `std::path::Path` doesn't implement `std::fmt::Display`")]
        let json = std::fs::read_to_string(path).context(format!("Failed to read file {path:?}"))?;
        #[allow(clippy::unnecessary_debug_formatting, reason = "False positive: `std::path::Path` doesn't implement `std::fmt::Display`")]
        serde_json::from_str(&json).context(format!("Failed to parse JSON in {path:?}"))
    }

    /// Save the file.
    ///
    /// # Errors
    ///
    /// * If the file can't be written.
    pub fn save(&self, path: &std::path::Path) -> anyhow::Result<()> {
        let json = serde_json::to_string_pretty(self).context("Failed to generate JSON")?;
        #[allow(clippy::unnecessary_debug_formatting, reason = "False positive: `std::path::Path` doesn't implement `std::fmt::Display`")]
        std::fs::write(path, json.as_bytes()).context(format!("Failed to write file {path:?}"))
    }

    #[allow(clippy::missing_errors_doc)]
    fn serialize_nodes<S>(nodes: &[Option<Node>], serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer {
        use serde::ser::SerializeSeq;

        let nodes = nodes.iter().filter_map(Option::as_ref);
        let mut seq = serializer.serialize_seq(nodes.size_hint().1)?;
        for node in nodes {
            seq.serialize_element(node)?;
        }
        seq.end()
    }

    #[allow(clippy::missing_errors_doc)]
    fn deserialize_nodes<'de, D>(deserializer: D) -> Result<Vec<Option<Node>>, D::Error> where D: serde::Deserializer<'de> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = Vec<Option<Node>>;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(formatter, "a sequence of upto 128 nodes")
            }
            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error> where A: serde::de::SeqAccess<'de>, {
                if let Some(len) = seq.size_hint() {
                    if len > 128 {
                        return Err(serde::de::Error::invalid_length(len, &self));
                    }
                }

                let mut nodes = File::initial_vec(128);
                while let Some(node) = seq.next_element::<Node>()? {
                    let index = usize::from(node.address.as_node_address());
                    if nodes[index].is_some() {
                        return Err(serde::de::Error::custom(format!("Duplicate node address {index} in nodes")));
                    }
                    nodes[index] = Some(node);
                }
                Ok(nodes)
            }
        }
        deserializer.deserialize_seq(Visitor)
    }

    fn initial_vec<T>(size: usize) -> Vec<Option<T>> {
        let mut vec = Vec::with_capacity(size);
        for _ in 0..size {
            vec.push(None);
        }
        vec
    }
}

impl Default for File {
    fn default() -> Self {
        Self {
            nodes: Self::initial_vec(128)
        }
    }
}

/// Labels for the input/output bits of a node.
#[derive(Debug, Default, Eq, PartialEq, serde::Serialize, serde::Deserialize, Clone)]
pub struct Labels {
    /// Labels for the input bits.
    #[serde(deserialize_with = "Labels::deserialize_field")]
    pub inputs: HashMap<usize, String>,

    /// Labels for the output bits.
    #[serde(deserialize_with = "Labels::deserialize_field")]
    pub outputs: HashMap<usize, String>
}

impl Labels {
  #[allow(clippy::missing_errors_doc)]
  fn deserialize_field<'de, D>(deserializer: D) -> Result<HashMap<usize, String>, D::Error> where D: serde::Deserializer<'de> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = HashMap<usize, String>;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(formatter, "a map of int (0-2048) to string")
            }
            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error> where A: serde::de::MapAccess<'de>, {
                if let Some(len) = map.size_hint() {
                    if len > 2048 {
                        return Err(serde::de::Error::invalid_length(len, &self));
                    }
                }

                let mut labels = Self::Value::new();
                while let Some((index, label)) = map.next_entry::<usize, String>()? {
                    if index > 2048 {
                        return Err(serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(u64::try_from(index).expect("Already checked it's in range")), &Self));
                    }
                    if labels.insert(index, label).is_some() {
                        return Err(serde::de::Error::custom(format!("Duplicate bit address {index}")));
                    }
                }
                Ok(labels)
            }
        }
        deserializer.deserialize_map(Visitor)
    }
}

/// Details about a CMRInet node.
#[derive(Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Node {
    /// Optional friendly name given to the node (e.g. "Station", "Main loop").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// The node's address.
    pub address: Address,

    /// The type and configuration of the node.
    #[serde(rename = "type")]
    pub sort: NodeSort,

    /// Labels to use for input & output bits
    #[serde(default)]
    pub labels: Labels
}


/// Load a previously saved list of nodes from a file.
///
/// # Errors
///
/// * If the file can't be read.
/// * If the JSON can't be parsed.
/// * If the JSON contains invalid data.
pub fn load_nodes(path: &std::path::Path) -> anyhow::Result<Vec<Option<Node>>> {
    Ok(File::load(path)?.nodes)
}

/// Save a list of `Node`s to a file.
///
/// If the file already exists only the nodes are replaced.
///
/// # Errors
///
/// * If the file can't be written.
pub fn save_nodes(path: &std::path::Path, nodes: Vec<Node>) -> anyhow::Result<()> {
    let mut file = if path.is_file() {
        File::load(path)?
    } else {
        File::default()
    };

    for node in nodes {
        let index = usize::from(node.address.as_node_address());
        file.nodes[index] = Some(node);
    }

    file.save(path)
}


#[allow(clippy::missing_panics_doc, reason="tests")]
#[cfg(test)]
mod tests {
    use rand::Rng;
    use super::*;

    struct TempFile(std::path::PathBuf);
    impl TempFile {
        pub fn new() -> Self {
            let dir = std::env::temp_dir();
            let mut rng = rand::rng();
            let path = loop {
                const SIZE: usize = 16;
                let mut file = String::with_capacity(SIZE);
                for _ in 0..SIZE {
                    file.push(char::from(b'A' + rng.random_range(0..26)));
                }
                let mut path = dir.clone();
                path.push(&file);
                if std::fs::metadata(&path).is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound) {
                    break path
                }
            };
            Self(path)
        }

        pub const fn path(&self) -> &std::path::PathBuf {
            &self.0
        }
    }
    impl Drop for TempFile {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    fn nodes() -> Vec<Node> {
        let mut labels = Labels::default();
        labels.inputs.insert(0, String::from("Node 0 Input 0"));
        labels.outputs.insert(0, String::from("Node 0 Output 0"));
        vec![
            Node {
                name: None,
                address: Address::try_from_node_address(0).unwrap(),
                sort: NodeSort::try_new_smini(0, [0; 6]).unwrap(),
                labels
            },
            Node {
                name: Some(String::from("Named node")),
                address: Address::try_from_node_address(1).unwrap(),
                sort: NodeSort::try_new_smini(0, [3, 6, 12, 24, 48, 96]).unwrap(),
                labels: Labels::default()
            }
        ]
    }

    const fn nodes_json() -> &'static str {
        indoc::indoc!{r#"{
          "nodes": [
            {
              "address": 0,
              "type": {
                "Smini": {
                  "configuration": {
                    "transmit_delay": 0,
                    "oscillating_count": 0,
                    "oscillating_pairs": [
                      0,
                      0,
                      0,
                      0,
                      0,
                      0
                    ]
                  }
                }
              },
              "labels": {
                "inputs": {
                  "0": "Node 0 Input 0"
                },
                "outputs": {
                  "0": "Node 0 Output 0"
                }
              }
            },
            {
              "name": "Named node",
              "address": 1,
              "type": {
                "Smini": {
                  "configuration": {
                    "transmit_delay": 0,
                    "oscillating_count": 6,
                    "oscillating_pairs": [
                      3,
                      6,
                      12,
                      24,
                      48,
                      96
                    ]
                  }
                }
              }
            }
          ]
        }"#}
    }

    mod save_nodes {
        use super::*;

        #[test]
        fn success() {
            let temp_file = TempFile::new();
            let expected = indoc::indoc!{r#"{
              "nodes": [
                {
                  "address": 0,
                  "type": {
                    "Smini": {
                      "configuration": {
                        "transmit_delay": 0,
                        "oscillating_count": 0,
                        "oscillating_pairs": [
                          0,
                          0,
                          0,
                          0,
                          0,
                          0
                        ]
                      }
                    }
                  },
                  "labels": {
                    "inputs": {
                      "0": "Node 0 Input 0"
                    },
                    "outputs": {
                      "0": "Node 0 Output 0"
                    }
                  }
                },
                {
                  "name": "Named node",
                  "address": 1,
                  "type": {
                    "Smini": {
                      "configuration": {
                        "transmit_delay": 0,
                        "oscillating_count": 6,
                        "oscillating_pairs": [
                          3,
                          6,
                          12,
                          24,
                          48,
                          96
                        ]
                      }
                    }
                  },
                  "labels": {
                    "inputs": {},
                    "outputs": {}
                  }
                }
              ]
            }"#};
            assert!(save_nodes(temp_file.path(), nodes()).is_ok());
            assert_eq!(std::fs::read_to_string(temp_file.path()).unwrap(), expected);
        }

//        #[test]
//        fn updates_if_file_exists() {
//            let temp_file = TempFile::new();
//            std::fs::write(temp_file.path(), labels_json()).unwrap();
//            assert!(save_nodes(temp_file.path(), nodes()).is_ok());
//            assert_eq!(std::fs::read_to_string(temp_file.path()).unwrap(), json());
//        }

        #[test]
        fn file_error() {
            let temp_file = TempFile::new();

            std::fs::write(temp_file.path(), nodes_json()).unwrap();
            let permissions = std::fs::metadata(temp_file.path()).unwrap().permissions();
            let mut readonly = permissions.clone();
            readonly.set_readonly(true);
            std::fs::set_permissions(temp_file.path(), readonly).unwrap();

            let error_message = save_nodes(temp_file.path(), nodes()).err().unwrap().root_cause().to_string();
            #[cfg(unix)]
            assert_eq!(&error_message, "Permission denied (os error 13)");
            #[cfg(not(unix))]
            assert_eq!(&error_message, "Access is denied. (os error 5)");

            std::fs::set_permissions(temp_file.path(), permissions).unwrap();
        }
    }

    mod load_nodes {
        use super::*;

        #[test]
        fn success() {
            let temp_file = TempFile::new();
            std::fs::write(temp_file.path(), nodes_json()).unwrap();
            let loaded = load_nodes(temp_file.path()).unwrap().iter_mut().filter_map(Option::take).collect::<Vec<Node>>();
            assert_eq!(loaded, nodes());
        }

        #[test]
        fn file_error() {
            let temp_file = TempFile::new();
            let error_message = load_nodes(temp_file.path()).err().unwrap().root_cause().to_string();
            #[cfg(unix)]
            assert_eq!(&error_message, "No such file or directory (os error 2)");
            #[cfg(not(unix))]
            assert_eq!(&error_message, "The system cannot find the file specified. (os error 2)");
        }

        #[test]
        fn json_error() {
            let temp_file = TempFile::new();
            std::fs::write(temp_file.path(), r#"{"nodes":[{"address":200,"type":null}]}"#).unwrap();
            let error_message = load_nodes(temp_file.path()).err().unwrap().root_cause().to_string();
            assert_eq!(&error_message, "invalid value: integer `200`, expected between 0 and 127 (inclusive) at line 1 column 24");
        }
    }
}
