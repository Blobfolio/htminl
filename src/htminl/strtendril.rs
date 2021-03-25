/*!
# HTML Traits: `StrTendril`

This trait exposes a few string manipulation methods to the `StrTendril`
struct.
*/

use dactyl::traits::SaturatingFrom;
use tendril::StrTendril;



/// Collapse Whitespace.
///
/// HTML rendering largely ignores whitespace, and at any rate treats all
/// types (other than the no-break space `\xA0`) the same.
///
/// There is some nuance, but for most elements, we can safely convert all
/// contiguous sequences of (ASCII) whitespace to a single horizontal space
/// character.
///
/// Complete trimming gets dangerous, particularly given that CSS can
/// override the display state of any element arbitrarily, so we are *not*
/// doing that here.
pub(super) fn collapse_whitespace(txt: &mut StrTendril) {
	let alter = StrTendril::from(unsafe {
		let mut in_ws: bool = false;
		std::str::from_utf8_unchecked(&txt.as_bytes()
			.iter()
			.filter_map(|c| match *c {
				b'\t' | b'\n' | b'\x0C' | b'\r' | b' ' =>
					if in_ws { None }
					else {
						in_ws = true;
						Some(b' ')
					},
				c =>
					if in_ws {
						in_ws = false;
						Some(c)
					}
					else { Some(c) },
			})
			.collect::<Vec<u8>>())
	});

	if (*txt).ne(&alter) {
		*txt = alter;
	}
}

#[allow(clippy::match_like_matches_macro)] // We're matching a negation.
/// Is (Only) Whitespace?
///
/// Returns `true` if the node is empty or contains only whitespace.
pub(super) fn is_whitespace(txt: &StrTendril) -> bool {
	! txt.as_bytes()
		.iter()
		.any(|c| match *c {
			b'\t' | b'\n' | b'\x0C' | b'\r' | b' ' => false,
			_ => true,
		})
}

/// Trim.
pub(super) fn trim(txt: &mut StrTendril) {
	trim_start(txt);
	trim_end(txt);
}

/// Trim Start.
pub(super) fn trim_start(txt: &mut StrTendril) {
	let len: u32 = u32::saturating_from(txt.as_bytes()
		.iter()
		.take_while(|c| matches!(*c, b'\t' | b'\n' | b'\x0C' | b'\r' | b' '))
		.count());
	if 0 != len {
		txt.pop_front(len);
	}
}

/// Trim End.
pub(super) fn trim_end(txt: &mut StrTendril) {
	let len: u32 = u32::saturating_from(txt.as_bytes()
		.iter()
		.rev()
		.take_while(|c| matches!(*c, b'\t' | b'\n' | b'\x0C' | b'\r' | b' '))
		.count());
	if 0 != len {
		txt.pop_back(len);
	}
}
