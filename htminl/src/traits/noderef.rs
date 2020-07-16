/*!
# HTML Traits: `NodeRef`

This trait exposes a few methods to the `NodeRef` struct.
*/

use crate::{
	meta::t,
	traits::MinifyElement,
};
use marked::{
	LocalName,
	NodeRef,
};



/// Minification-related Node(Ref) Methods.
pub trait MinifyNodeRef {
	/// Unnecessary Whitespace-Only Text Node Sandwiches
	///
	/// There are a lot of common situations where formatting whitespace would
	/// never play any role in the document layout. This matches those.
	///
	/// The text node itself is not verified by this method; those checks
	/// should be done first.
	fn can_drop_if_whitespace(&self) -> bool;

	/// Can Drop If Sandwhiched?
	fn can_drop_whitespace_sandwhich(&self) -> bool;

	/// Has Sibling
	fn has_sibling(&self) -> bool {
		! self.is_first_child() || ! self.is_last_child()
	}

	/// Is First Child.
	fn is_first_child(&self) -> bool;

	/// Is Last Child.
	fn is_last_child(&self) -> bool;

	/// Next Sibling Is.
	fn next_sibling_is_elem(&self, kind: LocalName) -> bool;

	/// Previous Sibling Is.
	fn prev_sibling_is_elem(&self, kind: LocalName) -> bool;

	/// Parent Is.
	fn parent_is_elem(&self, kind: LocalName) -> bool;

	/// Sibling Is.
	fn sibling_is_elem(&self, kind: LocalName) -> bool {
		self.prev_sibling_is_elem(kind.clone()) || self.next_sibling_is_elem(kind)
	}
}

impl<'a> MinifyNodeRef for NodeRef<'a> {
	/// Unnecessary Whitespace-Only Text Node Sandwiches
	fn can_drop_if_whitespace(&self) -> bool {
		// If the parent is a <pre> tag, we can trim between space between the
		// inner code tags, otherwise all whitespace needs to stay where it is.
		if self.parent_is_elem(t::PRE) {
			return self.sibling_is_elem(t::CODE);
		}

		// Otherwise, if we have a drop-capable sibling (and no not droppable ones)
		// we can drop it.
		self.prev_sibling().map_or(true, |n| n.can_drop_whitespace_sandwhich()) &&
		self.next_sibling().map_or(true, |n| n.can_drop_whitespace_sandwhich()) &&
		self.has_sibling()
	}

	/// Can Drop If Sandwhiched?
	fn can_drop_whitespace_sandwhich(&self) -> bool {
		self.as_element().map_or(false, |e| e.can_drop_whitespace_sandwhich())
	}

	/// Is First Child.
	fn is_first_child(&self) -> bool {
		self.prev_sibling().is_none()
	}

	/// Is Last Child.
	fn is_last_child(&self) -> bool {
		self.next_sibling().is_none()
	}

	/// Next Sibling Is.
	fn next_sibling_is_elem(&self, kind: LocalName) -> bool {
		self.next_sibling().map_or(false, |n| n.is_elem(kind))
	}

	/// Previous Sibling Is.
	fn prev_sibling_is_elem(&self, kind: LocalName) -> bool {
		self.prev_sibling().map_or(false, |n| n.is_elem(kind))
	}

	/// Parent Is.
	fn parent_is_elem(&self, kind: LocalName) -> bool {
		self.parent().map_or(false, |n| n.is_elem(kind))
	}
}
