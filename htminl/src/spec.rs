/*!
# HTML Library: Spec

This file contains several helpers related to attributes, elements, nodes, etc.
*/

use marked::{
	Attribute,
	Element,
	html::{
		t,
		TAG_META,
	},
	LocalName,
	NodeRef,
};



/// Minification-related Attribute Methods.
pub trait MinifyAttribute {
	/// Is Default Type
	///
	/// Default types like "text/javascript" don't need to be included in the
	/// document at all.
	fn is_default_type(&self, tag: &LocalName) -> bool;

	/// As Boolean
	///
	/// Attributes like "hidden", "draggable", etc., that either are or aren't.
	fn is_boolean_value(&self) -> bool;
}

impl MinifyAttribute for Attribute {
	/// Is Default Type
	///
	/// Default types like "text/javascript" don't need to be included in the
	/// document at all.
	fn is_default_type(&self, tag: &LocalName) -> bool {
		if &*self.name.local == "type" {
			match *tag {
				t::SCRIPT => match self.value.to_ascii_lowercase().as_str() {
					"text/javascript"
					| "application/javascript"
					| "application/x-javascript"
					| "text/ecmascript"
					| "application/ecmascript"
					| "text/jscript" => true,
					_ => false,
				},
				t::STYLE => self.value.to_ascii_lowercase() == "text/css",
				_ => false,
			}
		}
		else { false }
	}

	/// As Boolean
	///
	/// Attributes like "hidden", "draggable", etc., that either are or aren't.
	fn is_boolean_value(&self) -> bool {
		match &*self.name.local {
			"allowfullscreen"
			| "async"
			| "autofocus"
			| "autoplay"
			| "checked"
			| "compact"
			| "controls"
			| "declare"
			| "default"
			| "defaultchecked"
			| "defaultmuted"
			| "defaultselected"
			| "defer"
			| "disabled"
			| "enabled"
			| "formnovalidate"
			| "hidden"
			| "indeterminate"
			| "inert"
			| "ismap"
			| "itemscope"
			| "loop"
			| "multiple"
			| "muted"
			| "nohref"
			| "noresize"
			| "noshade"
			| "novalidate"
			| "nowrap"
			| "open"
			| "pauseonexit"
			| "readonly"
			| "required"
			| "reversed"
			| "scoped"
			| "seamless"
			| "selected"
			| "sortable"
			| "truespeed"
			| "typemustmatch"
			| "visible" => true,
			"draggable" => match self.value.to_ascii_lowercase().as_str() {
				"true" | "false" => true,
				_ => false,
			}
			_ => false,
		}
	}
}

/// Minification-related Element Methods.
pub trait MinifyElement {
	/// Is Minifiable
	///
	/// Can inner whitespace be collapsed? Most of the time the answer is yes,
	/// but there are a few cases where it is safer to leave things be.
	fn is_minifiable(&self) -> bool;
}

impl MinifyElement for Element {
	#[must_use]
	/// Is Minifiable
	///
	/// Can inner whitespace be collapsed? Most of the time the answer is yes,
	/// but there are a few cases where it is safer to leave things be.
	fn is_minifiable(&self) -> bool {
		match self.name.local {
			t::CODE
			| t::PRE
			| t::SCRIPT
			| t::STYLE
			| t::SVG
			| t::TEXTAREA => false,
			ref x => TAG_META.contains_key(x),
		}
	}
}

/// Minification-related Node(Ref) Methods.
pub trait MinifyNode {
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

impl MinifyNode for NodeRef<'_> {
	/// Next Element Is
	///
	/// Quick method to see if the next sibling exists and is a certain kind of
	/// element.
	fn next_sibling_is_elem(&self, kind: LocalName) -> bool {
		if let Some(ref x) = self.next_sibling() {
			if let Some(y) = (*x).as_element() {
				return y.is_elem(kind);
			}
		}

		false
	}

	/// Previous Element Is
	///
	/// Quick method to see if the previous sibling exists and is a certain
	/// kind of element.
	fn prev_sibling_is_elem(&self, kind: LocalName) -> bool {
		if let Some(ref x) = self.prev_sibling() {
			if let Some(y) = (*x).as_element() {
				return y.is_elem(kind);
			}
		}

		false
	}
}
