/*!
# HTML Traits: `NodeRef`

This trait exposes a few methods to the `NodeRef` struct.
*/

use marked::{
	LocalName,
	NodeRef,
};



/// Minification-related Node(Ref) Methods.
pub trait MinifyNodeRef {
	/// Next Element Kind.
	///
	/// Get the `LocalName` of the next sibling.
	fn next_sibling_elem_kind(&self) -> Option<LocalName>;

	/// Previous Element Kind.
	///
	/// Get the `LocalName` of the previous sibling.
	fn prev_sibling_elem_kind(&self) -> Option<LocalName>;

	/// Is First Child.
	fn is_first_child(&self) -> bool;

	/// Is Last Child.
	fn is_last_child(&self) -> bool;

	#[must_use]
	/// Sibling Element Kind
	fn sibling_elem_kind(node: &NodeRef) -> Option<LocalName> {
		node.as_element().map(|e| e.name.local.to_owned())
	}

	/// Next Element Is.
	///
	/// Quick method to see if the next sibling exists and is a certain kind of
	/// element.
	fn next_sibling_is_elem(&self, kind: LocalName) -> bool {
		match self.next_sibling_elem_kind() {
			Some(s) => s == kind,
			_ => false,
		}
	}

	/// Previous Element Is.
	///
	/// Quick method to see if the previous sibling exists and is a certain
	/// kind of element.
	fn prev_sibling_is_elem(&self, kind: LocalName) -> bool {
		match self.prev_sibling_elem_kind() {
			Some(s) => s == kind,
			_ => false,
		}
	}

	/// Sibling Element Is.
	///
	/// Quick method to see if either sibling is a certain kind of element.
	fn sibling_is_elem(&self, kind: LocalName) -> bool {
		if let Some(s) = self.prev_sibling_elem_kind() {
			if s == kind { return true; }
		}
		self.next_sibling_is_elem(kind)
	}
}

impl MinifyNodeRef for NodeRef<'_> {
	/// Next Element Kind.
	///
	/// Get the `LocalName` of the next sibling.
	fn next_sibling_elem_kind(&self) -> Option<LocalName> {
		self.next_sibling().as_ref().and_then(Self::sibling_elem_kind)
	}

	/// Previous Element Kind.
	///
	/// Get the `LocalName` of the previous sibling.
	fn prev_sibling_elem_kind(&self) -> Option<LocalName> {
		self.prev_sibling().as_ref().and_then(Self::sibling_elem_kind)
	}

	/// Is First Child.
	fn is_first_child(&self) -> bool {
		self.prev_sibling().is_none()
	}

	/// Is Last Child.
	fn is_last_child(&self) -> bool {
		self.next_sibling().is_none()
	}
}
