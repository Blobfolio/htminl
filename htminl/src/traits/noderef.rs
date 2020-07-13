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
	/// Next Element Is
	///
	/// Quick method to see if the next sibling exists and is a certain kind of
	/// element.
	fn next_sibling_is_elem(&self, kind: LocalName) -> bool;

	/// Previous Element Is
	///
	/// Quick method to see if the previous sibling exists and is a certain
	/// kind of element.
	fn prev_sibling_is_elem(&self, kind: LocalName) -> bool;
}

impl MinifyNodeRef for NodeRef<'_> {
	/// Next Element Is
	///
	/// Quick method to see if the next sibling exists and is a certain kind of
	/// element.
	fn next_sibling_is_elem(&self, kind: LocalName) -> bool {
		if let Some(el) = self.next_sibling().as_deref().and_then(|p| p.as_element()) {
			el.is_elem(kind)
		}
		else { false }
	}

	/// Previous Element Is
	///
	/// Quick method to see if the previous sibling exists and is a certain
	/// kind of element.
	fn prev_sibling_is_elem(&self, kind: LocalName) -> bool {
		if let Some(el) = self.prev_sibling().as_deref().and_then(|p| p.as_element()) {
			el.is_elem(kind)
		}
		else { false }
	}
}
