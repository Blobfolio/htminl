/*!
# HTMinL: Questions of Spec.
*/

use html5ever::{
	interface::QualName,
	local_name,
	ns,
	tendril::StrTendril,
};



#[derive(Debug, Clone, Copy)]
/// # Whitespace Options.
pub(crate) struct WhiteSpace(u8);

macro_rules! whitespace {
	( $($nice:ident $k:ident $v:literal,)+ ) => (
		impl WhiteSpace {
			$(
				/// # Flag.
				const $k: u8 = $v;

				#[must_use]
				/// # Getter.
				pub(crate) const fn $nice(self) -> bool {
					Self::$k == self.0 & Self::$k
				}
			)+

			/// # All Flags (Root State).
			pub(crate) const ROOT: Self = Self( $( Self::$k )|+ );
		}
	)
}

whitespace! {
	collapse   COLLAPSE   0b0001, // Collapse whitespace.
	drop_any   DROP_ANY   0b0010, // Drop text nodes period.
	drop_empty DROP_EMPTY 0b0110, // Drop text nodes if whitespace-only.
}

impl WhiteSpace {
	#[must_use]
	/// # New.
	pub(crate) const fn from_element(tag: &QualName) -> Self {
		let mut flags = 0;

		match tag.ns {
			// HTML is the main game, obviously.
			ns!(html) => {
				if can_collapse(tag) { flags |= Self::COLLAPSE; }
				if can_drop_any(tag) { flags |= Self::DROP_ANY; }
				else if can_drop_empty(tag) { flags |= Self::DROP_EMPTY; }
			},
			// We can do a _little_ bit of cleanup for SVGs.
			ns!(svg) => {
				match tag.local {
					local_name!("defs") |
					local_name!("g") |
					local_name!("svg") |
					local_name!("symbol") => flags |= Self::DROP_EMPTY,
					_ => {},
				}
			},
			_ => {},
		}

		Self(flags)
	}

	#[must_use]
	/// # Process.
	pub(crate) fn process(self, raw: &[u8]) -> Option<StrTendril> {
		// Drop it if droppable!
		if self.drop_any() || (self.drop_empty() && is_whitespace(raw)) {
			Some(StrTendril::new())
		}

		// Collapse if collapseable!
		else if
			self.collapse() &&
			let Some(new) = collapse(raw) &&
			new != raw &&
			let Ok(new) = String::from_utf8(new)
		{
			Some(StrTendril::from(new))
		}

		// Leave it be.
		else { None }
	}
}



#[must_use]
/// # Is Void HTML Element?
pub(crate) const fn is_void_html_tag(tag: &QualName) -> bool {
	matches!(tag.ns, ns!(html)) &&
	matches!(
		tag.local,
		local_name!("area") |
		local_name!("base") |
		local_name!("basefont") |
		local_name!("bgsound") |
		local_name!("br") |
		local_name!("col") |
		local_name!("embed") |
		local_name!("frame") |
		local_name!("hr") |
		local_name!("iframe") |
		local_name!("img") |
		local_name!("input") |
		local_name!("keygen") |
		local_name!("link") |
		local_name!("meta") |
		local_name!("param") |
		local_name!("source") |
		local_name!("track") |
		local_name!("wbr")
	)
}



#[expect(clippy::too_many_lines, reason = "There are a lot of tags.")]
#[must_use]
/// # Can Collapse Child Text Whitespace?
///
/// Contiguous whitespace in these elements has no effect, so we can collapse
/// long strings to a single whitespace.
const fn can_collapse(tag: &QualName) -> bool {
	matches!(tag.ns, ns!(html)) &&
	matches!(
		tag.local,
		local_name!("a") |
		local_name!("abbr") |
		local_name!("acronym") |
		local_name!("address") |
		local_name!("applet") |
		local_name!("area") |
		local_name!("article") |
		local_name!("aside") |
		local_name!("audio") |
		local_name!("b") |
		local_name!("base") |
		local_name!("basefont") |
		local_name!("bdi") |
		local_name!("bdo") |
		local_name!("big") |
		local_name!("blink") |
		local_name!("blockquote") |
		local_name!("body") |
		local_name!("br") |
		local_name!("button") |
		local_name!("canvas") |
		local_name!("caption") |
		local_name!("center") |
		local_name!("cite") |
		local_name!("col") |
		local_name!("colgroup") |
		local_name!("content") |
		local_name!("data") |
		local_name!("datalist") |
		local_name!("dd") |
		local_name!("del") |
		local_name!("details") |
		local_name!("dfn") |
		local_name!("dialog") |
		local_name!("dir") |
		local_name!("div") |
		local_name!("dl") |
		local_name!("dt") |
		local_name!("em") |
		local_name!("embed") |
		local_name!("fieldset") |
		local_name!("figcaption") |
		local_name!("figure") |
		local_name!("font") |
		local_name!("footer") |
		local_name!("form") |
		local_name!("frame") |
		local_name!("frameset") |
		local_name!("h1") |
		local_name!("h2") |
		local_name!("h3") |
		local_name!("h4") |
		local_name!("h5") |
		local_name!("h6") |
		local_name!("head") |
		local_name!("header") |
		local_name!("hgroup") |
		local_name!("hr") |
		local_name!("html") |
		local_name!("i") |
		local_name!("iframe") |
		local_name!("img") |
		local_name!("input") |
		local_name!("ins") |
		local_name!("isindex") |
		local_name!("kbd") |
		local_name!("label") |
		local_name!("legend") |
		local_name!("li") |
		local_name!("link") |
		local_name!("listing") |
		local_name!("main") |
		local_name!("map") |
		local_name!("mark") |
		local_name!("menu") |
		local_name!("menuitem") |
		local_name!("meta") |
		local_name!("meter") |
		local_name!("nav") |
		local_name!("nobr") |
		local_name!("noframes") |
		local_name!("noscript") |
		local_name!("object") |
		local_name!("ol") |
		local_name!("optgroup") |
		local_name!("option") |
		local_name!("output") |
		local_name!("p") |
		local_name!("param") |
		local_name!("picture") |
		local_name!("progress") |
		local_name!("q") |
		local_name!("rb") |
		local_name!("rp") |
		local_name!("rt") |
		local_name!("rtc") |
		local_name!("ruby") |
		local_name!("s") |
		local_name!("samp") |
		local_name!("section") |
		local_name!("select") |
		local_name!("slot") |
		local_name!("small") |
		local_name!("source") |
		local_name!("span") |
		local_name!("strike") |
		local_name!("strong") |
		local_name!("sub") |
		local_name!("summary") |
		local_name!("sup") |
		local_name!("table") |
		local_name!("tbody") |
		local_name!("td") |
		local_name!("tfoot") |
		local_name!("th") |
		local_name!("thead") |
		local_name!("time") |
		local_name!("title") |
		local_name!("tr") |
		local_name!("tt") |
		local_name!("u") |
		local_name!("ul") |
		local_name!("var") |
		local_name!("video") |
		local_name!("wbr") |
		local_name!("xmp")
	)
}

#[must_use]
/// # Can Drop Text Nodes (Period)?
///
/// Elements which shouldn't have any text nodes period.
const fn can_drop_any(tag: &QualName) -> bool {
	matches!(tag.ns, ns!(html)) &&
	matches!(
		tag.local,
		local_name!("audio") |
		local_name!("head") |
		local_name!("html") |
		local_name!("optgroup") |
		local_name!("picture") |
		local_name!("select") |
		local_name!("table") |
		local_name!("tbody") |
		local_name!("tfoot") |
		local_name!("tr") |
		local_name!("video")
	)
}

#[must_use]
/// # Can Drop Text Nodes?
///
/// Whitespace-only text nodes in these elements serve no purpose and
/// can be safely removed.
const fn can_drop_empty(tag: &QualName) -> bool {
	can_drop_any(tag) ||
	(
		matches!(tag.ns, ns!(html)) &&
		matches!(
			tag.local,
			local_name!("body") |
			local_name!("option") |
			local_name!("template")
		)
	)
}

#[must_use]
/// # Can Trim Child Text?
///
/// Leading whitespace can be trimmed from the first node and trailing from
/// the last.
pub(crate) const fn can_trim(tag: &QualName) -> bool {
	can_drop_empty(tag) ||
	match tag.ns {
		ns!(html) => matches!(
			tag.local,
			local_name!("script") |
			local_name!("style") |
			local_name!("title")
		),
		ns!(svg) => matches!(
			tag.local,
			local_name!("desc") |
			local_name!("script") |
			local_name!("style") |
			local_name!("title")
		),
		_ => false,
	}
}

#[must_use]
/// Collapse Whitespace.
///
/// HTML rendering largely ignores whitespace, and at any rate treats all
/// types (other than the no-break space `\xA0`) the same way.
///
/// There is some nuance, but for most elements, we can safely convert all
/// contiguous sequences of (ASCII) whitespace to a single horizontal space
/// character.
fn collapse(txt: &[u8]) -> Option<Vec<u8>> {
	// Edge case: single whitespace.
	if txt.len() == 1 && matches!(txt[0], b'\t' | b'\n' | b'\x0C') {
		return Some(vec![b' ']);
	}

	// Find the first non-space whitespace, or pair of (any) whitespaces.
	let pos = txt.windows(2).position(|pair|
		matches!(pair[0], b'\t' | b'\n' | b'\x0C') ||
		(pair[0].is_ascii_whitespace() && pair[1].is_ascii_whitespace())
	)?;

	// Split at that location and start building up a replacement.
	let (a, rest) = txt.split_at(pos);
	let mut new = Vec::with_capacity(txt.len());
	new.extend_from_slice(a);

	let mut in_ws = false;
	for &b in rest {
		match b {
			b'\t' | b'\n' | b'\x0C' | b' ' => if ! in_ws {
				in_ws = true;
				new.push(b' ');
			},
			_ => {
				in_ws = false;
				new.push(b);
			},
		}
	}

	Some(new)
}

#[must_use]
/// Is (Only) Whitespace?
///
/// Returns `true` if the node is empty or contains only whitespace.
///
/// Note that CR is replaced with LF prior to parsing, so there's no need
/// to include `b'\r'` in the matchset.
const fn is_whitespace(mut txt: &[u8]) -> bool {
	while let [b'\t' | b'\n' | b'\x0C' | b' ', rest @ ..] = txt { txt = rest; }
	txt.is_empty()
}



#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn t_can_collapse() {
		// The collapseable list is really big, but should _not_ include
		// these things.
		for i in [
			local_name!("code"),
			local_name!("plaintext"),
			local_name!("pre"),
			local_name!("script"),
			local_name!("style"),
			local_name!("svg"),
			local_name!("textarea"),
		] {
			let name = QualName::new(None, ns!(html), i);
			assert!(! can_collapse(&name));
		}
	}

	#[test]
	fn t_collapse() {
		for (lhs, rhs) in [
			(&b"raw"[..], None),
			(b" ", None),
			(b"  ", Some(vec![b' '])),
			(b"   ", Some(vec![b' '])),
			(b"\n", Some(vec![b' '])),
			(b"hello world", None),
			(b"hello\nworld", Some(b"hello world".to_vec())),
			(b"hello \x0C \t\nworld", Some(b"hello world".to_vec())),
			(b"hello\x0C \t\nworld, hello  moon", Some(b"hello world, hello moon".to_vec())),
		] {
			assert_eq!(collapse(lhs), rhs);
		}
	}

	#[test]
	fn t_is_whitespace() {
		assert!(is_whitespace(b""));
		assert!(is_whitespace(b"  \t\n  \x0C"));
		assert!(! is_whitespace(b"  a "));
	}
}
