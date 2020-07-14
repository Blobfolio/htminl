/*!
# HTML Traits: `Attribute`

This trait exposes a few methods to the `Attribute` struct.
*/

use marked::{
	Attribute,
	html::t,
	LocalName,
};



/// Minification-related Attribute Methods.
pub trait MinifyAttribute {
	/// Can Drop Attribute?
	///
	/// Certain attributes, such as `type="text/javascript"` on a `<script>`
	/// element are unnecessary (and actively discouraged), so can be safely
	/// removed from the document.
	fn can_drop(&self, tag: &LocalName) -> bool;

	/// Can Drop Value?
	///
	/// HTML doesn't require explicit empty values, or values on various
	/// "boolean"-like properties such as `autoplay`, `defer`, etc. In such
	/// cases the attribute name is all that matters; the value can be safely
	/// dropped.
	fn can_drop_value(&self) -> bool;

	/// Is Boolean Attribute?
	///
	/// These attributes either are or aren't. Their existence implies "true",
	/// so if they're true they don't need values, and if they're false they
	/// don't need to be at all.
	fn is_boolean(&self) -> bool;
}

impl MinifyAttribute for Attribute {
	/// Can Drop Attribute?
	///
	/// Certain attributes, such as `type="text/javascript"` on a `<script>`
	/// element are unnecessary (and actively discouraged), so can be safely
	/// removed from the document.
	fn can_drop(&self, tag: &LocalName) -> bool {
		match &*self.name.local {
			// Default "type" tags. Technically `<input type="text"/>` is
			// also a default, but because it is frequently matched in CSS
			// rules, we'll leave those be.
			"type" => match *tag {
				t::SCRIPT => match self.value.to_ascii_lowercase().as_str() {
					"text/javascript"
					| "application/javascript"
					| "application/x-javascript"
					| "text/ecmascript"
					| "application/ecmascript"
					| "text/jscript" => true,
					_ => false,
				},
				t::STYLE => self.value.eq_ignore_ascii_case("text/css"),
				_ => false,
			},
			// These tags serve no purpose if they have no values!
			"alt" | "href" | "src" | "srcset" | "target" | "title" => self.value.is_empty(),
			// If this is a falsey boolean attribute, we can get rid of it.
			_ => self.is_boolean() && self.value.eq_ignore_ascii_case("false"),
		}
	}

	/// Can Drop Value?
	///
	/// HTML doesn't require explicit empty values, or values on various
	/// "boolean"-like properties such as `autoplay`, `defer`, etc. In such
	/// cases the attribute name is all that matters; the value can be safely
	/// dropped.
	fn can_drop_value(&self) -> bool {
		self.value.is_empty() ||
		(self.is_boolean() && ! self.value.eq_ignore_ascii_case("false"))
	}

	/// Is Boolean Attribute?
	///
	/// These attributes either are or aren't. Their existence implies "true",
	/// so if they're true they don't need values, and if they're false they
	/// don't need to be at all.
	fn is_boolean(&self) -> bool {
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
			// Draggable has other possible properties, so we'll only count it
			// as a boolean if it is true/false/draggable.
			"draggable" => match self.value.to_ascii_lowercase().as_str() {
				"true" | "false" | "draggable" => true,
				_ => false,
			}
			_ => false,
		}
	}
}
