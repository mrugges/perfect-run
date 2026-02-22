use crate::Error;
use bg3_lib::lsf_reader::{
    LSFReader, Node, NodeAttributeValue, RegionArena, Resource,
};
use bg3_lib::package::Package;
use bg3_lib::package_reader::PackageReader;

/// Load and parse an LSF file from within an LSV package.
/// `file_name` should be e.g. "meta.lsf" or "globals.lsf".
pub fn load_lsf(
    reader: &mut PackageReader,
    package: &Package,
    file_name: &str,
) -> Result<Resource, Error> {
    let pfi = package
        .files
        .iter()
        .find(|f| {
            f.name
                .to_string_lossy()
                .to_lowercase()
                .contains(&file_name.to_lowercase())
        })
        .ok_or_else(|| Error::FileNotFound(file_name.to_string()))?;

    let mut lsf_reader = LSFReader::new();
    lsf_reader.read(reader, pfi).map_err(Error::Package)
}

/// Load globals.lsf using the convenience method on PackageReader.
pub fn load_globals(
    reader: &mut PackageReader,
    package: &Package,
) -> Result<Resource, Error> {
    reader.load_globals(package).map_err(Error::Package)
}

/// Get a string attribute value from a node, if it exists.
pub fn get_string_attr(node: &Node, name: &str) -> Option<String> {
    node.attributes.get(name).and_then(|attr| match &attr.value {
        NodeAttributeValue::String(s) => Some(s.clone()),
        _ => None,
    })
}

/// Get a translated string attribute value from a node.
pub fn get_translated_string_attr(node: &Node, name: &str) -> Option<String> {
    node.attributes.get(name).and_then(|attr| match &attr.value {
        NodeAttributeValue::TranslatedString(ts) => {
            let s = ts.to_string();
            if s == "Option::None" || s.is_empty() {
                None
            } else {
                Some(s)
            }
        }
        NodeAttributeValue::String(s) => Some(s.clone()),
        _ => None,
    })
}

/// Get an integer attribute value from a node.
pub fn get_int_attr(node: &Node, name: &str) -> Option<i32> {
    node.attributes.get(name).and_then(|attr| match &attr.value {
        NodeAttributeValue::Int(v) => Some(*v),
        NodeAttributeValue::Short(v) => Some(*v as i32),
        NodeAttributeValue::Byte(v) => Some(*v as i32),
        NodeAttributeValue::I8(v) => Some(*v as i32),
        _ => None,
    })
}

/// Get an unsigned integer attribute value from a node.
pub fn get_uint_attr(node: &Node, name: &str) -> Option<u32> {
    node.attributes.get(name).and_then(|attr| match &attr.value {
        NodeAttributeValue::UInt(v) => Some(*v),
        NodeAttributeValue::UShort(v) => Some(*v as u32),
        NodeAttributeValue::Byte(v) => Some(*v as u32),
        _ => None,
    })
}

/// Get a UUID attribute value from a node.
pub fn get_uuid_attr(node: &Node, name: &str) -> Option<String> {
    node.attributes.get(name).and_then(|attr| match &attr.value {
        NodeAttributeValue::Uuid(u) => Some(u.to_string()),
        NodeAttributeValue::String(s) => Some(s.clone()),
        _ => None,
    })
}

/// Get a u64 attribute value from a node.
pub fn get_uint64_attr(node: &Node, name: &str) -> Option<u64> {
    node.attributes.get(name).and_then(|attr| match &attr.value {
        NodeAttributeValue::UInt64(v) => Some(*v),
        NodeAttributeValue::UInt(v) => Some(*v as u64),
        _ => None,
    })
}

/// Get bytes attribute value from a node.
pub fn get_bytes_attr(node: &Node, name: &str) -> Option<Vec<u8>> {
    node.attributes.get(name).and_then(|attr| match &attr.value {
        NodeAttributeValue::Bytes(b) => Some(b.clone()),
        _ => None,
    })
}

/// Get a bool attribute value from a node.
pub fn get_bool_attr(node: &Node, name: &str) -> Option<bool> {
    node.attributes.get(name).and_then(|attr| match &attr.value {
        NodeAttributeValue::Bool(v) => Some(*v),
        _ => None,
    })
}

/// Recursively dump the node tree for debugging/exploration.
/// Returns a formatted string showing the tree structure.
pub fn dump_tree(arena: &RegionArena, max_depth: usize) -> String {
    let mut output = String::new();
    for node in arena.get_region_nodes() {
        dump_node_recursive(arena, node, 0, max_depth, &mut output);
    }
    output
}

fn dump_node_recursive(
    arena: &RegionArena,
    node: &Node,
    depth: usize,
    max_depth: usize,
    output: &mut String,
) {
    if depth > max_depth {
        return;
    }

    let indent = "  ".repeat(depth);
    let kind = match &node.kind {
        bg3_lib::lsf_reader::NodeKind::Region { name } => format!("[Region: {}]", name),
        bg3_lib::lsf_reader::NodeKind::Node => String::new(),
    };

    output.push_str(&format!("{}{} {}\n", indent, node.name, kind));

    // Print attributes
    for (attr_name, attr) in &node.attributes {
        let value_str = format_attribute_value(&attr.value);
        output.push_str(&format!("{}  @{}: {:?} = {}\n", indent, attr_name, attr.ty, value_str));
    }

    // Recurse into children
    for child_indices in node.children.values() {
        for &idx in child_indices {
            if let Some(child) = arena.get_node(idx) {
                dump_node_recursive(arena, child, depth + 1, max_depth, output);
            }
        }
    }
}

/// Format an attribute value for display, truncating large byte arrays.
pub fn format_attribute_value(value: &NodeAttributeValue) -> String {
    match value {
        NodeAttributeValue::String(s) => format!("\"{}\"", s),
        NodeAttributeValue::TranslatedString(ts) => format!("t\"{}\"", ts),
        NodeAttributeValue::Bytes(b) if b.len() > 32 => {
            format!("[{} bytes: {:02x?}...]", b.len(), &b[..16])
        }
        NodeAttributeValue::Bytes(b) => format!("[{} bytes: {:02x?}]", b.len(), b),
        NodeAttributeValue::Uuid(u) => u.to_string(),
        NodeAttributeValue::Bool(v) => v.to_string(),
        NodeAttributeValue::Int(v) => v.to_string(),
        NodeAttributeValue::UInt(v) => v.to_string(),
        NodeAttributeValue::Short(v) => v.to_string(),
        NodeAttributeValue::UShort(v) => v.to_string(),
        NodeAttributeValue::Byte(v) => v.to_string(),
        NodeAttributeValue::I8(v) => v.to_string(),
        NodeAttributeValue::Float(v) => v.to_string(),
        NodeAttributeValue::Double(v) => v.to_string(),
        NodeAttributeValue::Int64(v) => v.to_string(),
        NodeAttributeValue::UInt64(v) => v.to_string(),
        NodeAttributeValue::Vec3(v) => format!("({}, {}, {})", v[0], v[1], v[2]),
        NodeAttributeValue::Vec4(v) => format!("({}, {}, {}, {})", v[0], v[1], v[2], v[3]),
        other => format!("{:?}", other),
    }
}

/// Find all child nodes with a given name across the entire arena.
pub fn find_nodes_by_name<'a>(arena: &'a RegionArena, name: &str) -> Vec<&'a Node> {
    arena
        .node_instances
        .iter()
        .filter(|n| n.name == name)
        .collect()
}

/// Walk the tree to find nodes matching a path like "Globals/Party/Characters".
pub fn find_nodes_by_path<'a>(arena: &'a RegionArena, path: &[&str]) -> Vec<&'a Node> {
    if path.is_empty() {
        return vec![];
    }

    // Start from regions matching the first path element
    let mut current_indices: Vec<usize> = arena
        .regions_indices
        .iter()
        .filter(|(name, _)| name.as_str() == path[0])
        .map(|(_, &idx)| idx)
        .collect();

    // If no region matches, try all nodes
    if current_indices.is_empty() {
        current_indices = arena
            .node_instances
            .iter()
            .enumerate()
            .filter(|(_, n)| n.name == path[0])
            .map(|(i, _)| i)
            .collect();
    }

    for &segment in &path[1..] {
        let mut next_indices = Vec::new();
        for &idx in &current_indices {
            if let Some(node) = arena.get_node(idx) {
                if let Some(children) = node.children.get(segment) {
                    next_indices.extend(children);
                }
            }
        }
        current_indices = next_indices;
    }

    current_indices
        .iter()
        .filter_map(|&idx| arena.get_node(idx))
        .collect()
}
