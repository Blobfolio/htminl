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
