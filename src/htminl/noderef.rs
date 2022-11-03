/*!
# HTML Traits: `NodeRef`

This trait exposes a few methods to the `NodeRef` struct.
*/

use marked::{
	LocalName,
	NodeRef,
};
use super::{
	element,
	meta::t,
};



#[must_use]
/// Unnecessary Whitespace-Only Text Node Sandwiches
pub(super) fn can_drop_if_whitespace(node: &NodeRef) -> bool {
	// If the parent is a <pre> tag, we can trim between space between the
	// inner code tags, otherwise all whitespace needs to stay where it is.
	if parent_is_elem(node, t::PRE) {
		return sibling_is_elem(node, t::CODE);
	}

	// Otherwise, if we have a drop-capable sibling (and no not droppable ones)
	// we can drop it.
	node.prev_sibling().as_ref().map_or(true, can_drop_whitespace_sandwich) &&
	node.next_sibling().as_ref().map_or(true, can_drop_whitespace_sandwich) &&
	has_sibling(node)
}

#[must_use]
/// Can Drop If Sandwhiched?
pub(super) fn can_drop_whitespace_sandwich(node: &NodeRef) -> bool {
	node.as_element().map_or(false, element::can_drop_whitespace_sandwich)
}

#[must_use]
/// Has Sibling
pub(super) fn has_sibling(node: &NodeRef) -> bool {
	! is_first_child(node) || ! is_last_child(node)
}

#[must_use]
/// Is First Child.
pub(super) fn is_first_child(node: &NodeRef) -> bool {
	node.prev_sibling().is_none()
}

#[must_use]
/// Is Last Child.
pub(super) fn is_last_child(node: &NodeRef) -> bool {
	node.next_sibling().is_none()
}

#[must_use]
/// Next Sibling Is.
pub(super) fn next_sibling_is_elem(node: &NodeRef, kind: LocalName) -> bool {
	node.next_sibling().map_or(false, |n| n.is_elem(kind))
}

#[must_use]
/// Previous Sibling Is.
pub(super) fn prev_sibling_is_elem(node: &NodeRef, kind: LocalName) -> bool {
	node.prev_sibling().map_or(false, |n| n.is_elem(kind))
}

#[must_use]
/// Parent Is.
pub(super) fn parent_is_elem(node: &NodeRef, kind: LocalName) -> bool {
	node.parent().map_or(false, |n| n.is_elem(kind))
}

#[must_use]
/// Sibling Is.
pub(super) fn sibling_is_elem(node: &NodeRef, kind: LocalName) -> bool {
	prev_sibling_is_elem(node, kind.clone()) || next_sibling_is_elem(node, kind)
}
