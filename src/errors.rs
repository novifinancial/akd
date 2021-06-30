use core::fmt;

use crate::node_state::NodeLabel;

#[derive(Debug)]
pub enum SeemlessError {
    HistoryTreeNodeErr(HistoryTreeNodeError),
    SeemlessDirectoryErr(SeemlessDirectoryError),
    AzksErr(AzksError),
}

impl From<HistoryTreeNodeError> for SeemlessError {
    fn from(error: HistoryTreeNodeError) -> Self {
        Self::HistoryTreeNodeErr(error)
    }
}

impl From<SeemlessDirectoryError> for SeemlessError {
    fn from(error: SeemlessDirectoryError) -> Self {
        Self::SeemlessDirectoryErr(error)
    }
}

impl From<AzksError> for SeemlessError {
    fn from(error: AzksError) -> Self {
        Self::AzksErr(error)
    }
}

#[derive(Debug)]
pub enum HistoryTreeNodeError {
    NoDirectionInSettingChild(u64, u64),
    DirectionIsNone,
    NoChildInTreeAtEpoch(u64, usize),
    NoChildrenInTreeAtEpoch(u64),
    InvalidEpochForUpdatingHash(u64),
    TriedToUpdateParentOfRoot,
    ParentNextEpochInvalid(u64),
    HashUpdateOnlyAllowedAfterNodeInsertion,
    TriedToHashLeafChildren,
    NodeCreatedWithoutEpochs(u64),
    LeafNodeLabelLenLessThanInterior(NodeLabel),
    CompressionError(NodeLabel),
}

impl fmt::Display for HistoryTreeNodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoDirectionInSettingChild(node_label, child_label) => {
                write!(
                    f,
                    "no direction provided to set the child {} of this node {}",
                    node_label, child_label
                )
            }
            Self::NoChildrenInTreeAtEpoch(epoch) => {
                write!(f, "no children at epoch {}", epoch)
            }
            Self::NoChildInTreeAtEpoch(epoch, direction) => {
                write!(f, "no node in direction {} at epoch {}", direction, epoch)
            }
            Self::DirectionIsNone => {
                write!(f, "Direction provided is None")
            }
            Self::InvalidEpochForUpdatingHash(epoch) => {
                write!(f, "Invalid epoch for updating hash {}", epoch)
            }
            Self::TriedToUpdateParentOfRoot => {
                write!(f, "Tried to update parent of root")
            }
            Self::ParentNextEpochInvalid(epoch) => {
                write!(f, "Next epoch of parent is invalid, epoch = {}", epoch)
            }
            Self::HashUpdateOnlyAllowedAfterNodeInsertion => {
                write!(
                    f,
                    "Hash update in parent only allowed after node is inserted"
                )
            }
            Self::TriedToHashLeafChildren => {
                write!(f, "Tried to hash the children of a leaf")
            }
            Self::NodeCreatedWithoutEpochs(label) => {
                write!(f, "A node exists which has no epochs. Nodes should always have epochs, labelled: {}", label)
            }
            Self::LeafNodeLabelLenLessThanInterior(label) => {
                write!(f, "A leaf was inserted with lable length shorter than an interior node, labelled: {:?}", label)
            }
            Self::CompressionError(label) => {
                write!(
                    f,
                    "A node without a child in some direction exists, labelled: {:?}",
                    label
                )
            }
        }
    }
}

#[derive(Debug)]
pub enum AzksError {
    PopFromEmptyPriorityQueue(u64),
}

impl fmt::Display for AzksError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PopFromEmptyPriorityQueue(epoch) => {
                write!(
                    f,
                    "Tried to pop from an empty priority queue at ep {:?}",
                    epoch
                )
            }
        }
    }
}
#[derive(Debug)]
pub enum SeemlessDirectoryError {
    AuditProofStartEpLess(u64, u64),
}

impl fmt::Display for SeemlessDirectoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AuditProofStartEpLess(start, end) => {
                write!(
                    f,
                    "Audit proof requested for epoch {:?} till {:?} and the audit start epoch is greater than or equal to the end.",
                    start,
                    end
                )
            }
        }
    }
}
