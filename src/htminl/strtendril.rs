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
	let alter = StrTendril::from({
		let mut in_ws: bool = false;
		String::from_utf8_lossy(&txt.bytes()
			.filter_map(|c| match c {
				b'\t' | b'\n' | b'\x0C' | b'\r' | b' ' =>
					if in_ws { None }
					else {
						in_ws = true;
						Some(b' ')
					},
				c => {
					if in_ws { in_ws = false; }
					Some(c)
				},
			})
			.collect::<Vec<u8>>())
			.as_ref()
	});

	if (*txt).ne(&alter) {
		*txt = alter;
	}
}

/// Is (Only) Whitespace?
///
/// Returns `true` if the node is empty or contains only whitespace.
pub(super) fn is_whitespace(txt: &StrTendril) -> bool {
	txt.is_empty() ||
	txt.bytes().all(|c| matches!(c, b'\t' | b'\n' | b'\x0C' | b'\r' | b' '))
}

/// Trim.
pub(super) fn trim(txt: &mut StrTendril) {
	trim_start(txt);
	trim_end(txt);
}

/// Trim Start.
pub(super) fn trim_start(txt: &mut StrTendril) {
	let len = txt.bytes()
		.take_while(|c| matches!(c, b'\t' | b'\n' | b'\x0C' | b'\r' | b' '))
		.count();
	if 0 != len {
		txt.pop_front(u32::saturating_from(len));
	}
}

/// Trim End.
pub(super) fn trim_end(txt: &mut StrTendril) {
	let len = txt.bytes()
		.rev()
		.take_while(|c| matches!(c, b'\t' | b'\n' | b'\x0C' | b'\r' | b' '))
		.count();
	if 0 != len {
		txt.pop_back(u32::saturating_from(len));
	}
}
