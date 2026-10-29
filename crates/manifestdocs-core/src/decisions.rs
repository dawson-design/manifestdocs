//! `decisions`: a tree's records, grouped by the node whose `decisions/` folder holds them.

use std::cmp::Reverse;
use std::iter;

use crate::layout::records_folder;
use crate::paths::TreePath;
use crate::record::ParsedRecord;
use crate::tree::{Entry, Tree};

/// One node's records, newest first, with records that have frontmatter errors last.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Group<'a> {
    pub node: &'a Entry,
    pub records: Vec<&'a ParsedRecord>,
}

/// The records of every node in outline order, or of `node` and then its ancestors, nearest
/// first. Nodes with no records are left out.
pub fn grouped<'a>(
    tree: &'a Tree,
    records: &'a [ParsedRecord],
    node: Option<&'a Entry>,
) -> Vec<Group<'a>> {
    let nodes: Vec<&Entry> = match node {
        Some(entry) => iter::successors(Some(entry), |e| tree.parent(&e.file)).collect(),
        None => {
            let outline = tree.outline(&TreePath::default(), None);
            outline.into_iter().map(|(_, entry)| entry).collect()
        }
    };
    let groups = nodes.into_iter().map(|node| {
        let owned = records.iter().filter(|record| {
            let owner = owner(tree, &record.file);
            owner.is_some_and(|owner| owner.file == node.file)
        });
        Group {
            node,
            records: newest_first(owned),
        }
    });
    groups.filter(|group| !group.records.is_empty()).collect()
}

/// The nearest node at or above the folder that holds `file`'s `decisions/` folder.
pub fn owner<'a>(tree: &'a Tree, file: &TreePath) -> Option<&'a Entry> {
    let mut folder = records_folder(file)?.parent();
    while let Some(current) = folder {
        if let Some(entry) = tree.entry_at(&current) {
            return Some(entry);
        }
        folder = current.parent();
    }
    None
}

fn newest_first<'a>(records: impl Iterator<Item = &'a ParsedRecord>) -> Vec<&'a ParsedRecord> {
    let mut records: Vec<&ParsedRecord> = records.collect();
    records.sort_by_key(|parsed| {
        let date = parsed.record.as_ref().map(|record| &record.date);
        (date.is_none(), Reverse(date), &parsed.file)
    });
    records
}
