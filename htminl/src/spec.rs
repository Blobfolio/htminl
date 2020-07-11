/*!
# `HTMinL`

In-place minification of HTML file(s).

This approach based on `html5minify`.
*/

use marked::{
	Attribute,
	Element,
	html::t,
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
	/// Is Canonical
	///
	/// Is this a known element type (versus some sort of custom thing)?
	fn is_canonical(&self) -> bool;

	/// Is Minifiable
	///
	/// Can inner whitespace be collapsed? Most of the time the answer is yes,
	/// but there are a few cases where it is safer to leave things be.
	fn is_minifiable(&self) -> bool;
}

impl MinifyElement for Element {
	#[allow(clippy::too_many_lines)] // HTML is big, what can I say?
	#[must_use]
	/// Is Canonical
	///
	/// Is this a known element type (versus some sort of custom thing)?
	fn is_canonical(&self) -> bool {
		match self.name.local {
			t::A
			| t::ABBR
			| t::ACRONYM
			| t::ADDRESS
			| t::APPLET
			| t::AREA
			| t::ARTICLE
			| t::ASIDE
			| t::AUDIO
			| t::B
			| t::BASE
			| t::BASEFONT
			| t::BDI
			| t::BDO
			| t::BIG
			| t::BLINK
			| t::BLOCKQUOTE
			| t::BODY
			| t::BR
			| t::BUTTON
			| t::CANVAS
			| t::CAPTION
			| t::CENTER
			| t::CITE
			| t::CODE
			| t::COL
			| t::COLGROUP
			| t::CONTENT
			| t::DATA
			| t::DATALIST
			| t::DD
			| t::DEL
			| t::DETAILS
			| t::DFN
			| t::DIALOG
			| t::DIR
			| t::DIV
			| t::DL
			| t::DT
			| t::EM
			| t::EMBED
			| t::FIELDSET
			| t::FIGCAPTION
			| t::FIGURE
			| t::FONT
			| t::FOOTER
			| t::FORM
			| t::FRAME
			| t::FRAMESET
			| t::H1
			| t::H2
			| t::H3
			| t::H4
			| t::H5
			| t::H6
			| t::HEAD
			| t::HEADER
			| t::HGROUP
			| t::HR
			| t::HTML
			| t::I
			| t::IFRAME
			| t::IMG
			| t::INPUT
			| t::INS
			| t::ISINDEX
			| t::KBD
			| t::LABEL
			| t::LEGEND
			| t::LI
			| t::LINK
			| t::LISTING
			| t::MAIN
			| t::MAP
			| t::MARK
			| t::MENU
			| t::MENUITEM
			| t::META
			| t::METER
			| t::NAV
			| t::NOBR
			| t::NOFRAMES
			| t::NOSCRIPT
			| t::OBJECT
			| t::OL
			| t::OPTGROUP
			| t::OPTION
			| t::OUTPUT
			| t::P
			| t::PARAM
			| t::PICTURE
			| t::PLAINTEXT
			| t::PRE
			| t::PROGRESS
			| t::Q
			| t::RB
			| t::RP
			| t::RT
			| t::RTC
			| t::RUBY
			| t::S
			| t::SAMP
			| t::SCRIPT
			| t::SECTION
			| t::SELECT
			| t::SLOT
			| t::SMALL
			| t::SOURCE
			| t::SPAN
			| t::STRIKE
			| t::STRONG
			| t::STYLE
			| t::SUB
			| t::SUMMARY
			| t::SUP
			| t::SVG
			| t::TABLE
			| t::TBODY
			| t::TD
			| t::TEMPLATE
			| t::TEXTAREA
			| t::TFOOT
			| t::TH
			| t::THEAD
			| t::TIME
			| t::TITLE
			| t::TR
			| t::TT
			| t::U
			| t::UL
			| t::VAR
			| t::VIDEO
			| t::WBR
			| t::XMP => true,
			_ => false,
		}
	}

	#[must_use]
	/// Is Minifiable
	///
	/// Can inner whitespace be collapsed? Most of the time the answer is yes,
	/// but there are a few cases where it is safer to leave things be.
	fn is_minifiable(&self) -> bool {
		(match self.name.local {
			t::CODE
			| t::PRE
			| t::SCRIPT
			| t::STYLE
			| t::TEXTAREA => false,
			_ => true,
		}) && self.is_canonical()
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
