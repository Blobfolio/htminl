/*!
# HTMinL: Minification.
*/

use crate::{
	HtminlError,
	Tree,
};
use std::{
	num::NonZeroU64,
	path::Path,
};



/// # Fragment Open.
const FRAGMENT_OPEN: &str = "<fragment-marker>";

/// # Fragment Close.
const FRAGMENT_CLOSE: &str = "</fragment-marker>";



#[expect(clippy::cast_possible_truncation, reason = "False positive.")]
/// # Minify a Document (or Fragment).
///
/// Read the raw HTML from a file, parse it into a tree, clean and minify said
/// tree, turn it _back_ into HTML, and save it!
///
/// ## Errors
///
/// This will return an error if the file is unreadable, empty, or unparseable,
/// or if issues are encountered when trying to re-save it.
pub(super) fn minify(src: &Path) -> Result<(NonZeroU64, NonZeroU64), HtminlError> {
	// Load the file.
	let mut raw = std::fs::read_to_string(src).map_err(|_| HtminlError::Read)?;
	let before = u64::try_from(raw.len())
		.ok()
		.and_then(NonZeroU64::new)
		.ok_or(HtminlError::EmptyFile)?;

	// Replace all CRLF/CR instances with LF before parsing anything.
	let mut changed = false;
	while let Some(pos) = raw.find("\r\n") {
		raw.replace_range(pos..pos + 2, "\n");
		changed = true;
	}
	while let Some(pos) = raw.find('\r') {
		raw.replace_range(pos..=pos, "\n");
		changed = true;
	}

	// If this is a "fragment", wrap it so we can tease the relevant bit back
	// out after processing.
	let fragment = is_fragment(raw.as_bytes());
	if fragment { make_whole(&mut raw); }

	// Parse the document into a tree.
	let dom = Tree::parse(raw.as_bytes())?;

	// Try to save the results!
	let out = crate::ser::serialize(&dom, before.get() as usize).ok_or(HtminlError::Save)?;
	let mut out = String::from_utf8(out).map_err(|_| HtminlError::Save)?;

	// If the original was a fragment, re-fragmentize it.
	if fragment {
		make_fragment(&mut raw); // Convert the original back too.
		if ! make_fragment(&mut out) { return Err(HtminlError::Parse); }
	}

	// Save it if different!
	if (changed || raw != out) && ! out.is_empty() {
		let after = u64::try_from(out.len())
			.ok()
			.and_then(NonZeroU64::new)
			.ok_or(HtminlError::EmptyFile)?;
		write_atomic::write_file(src, out.as_bytes()).map_err(|_| HtminlError::Save)?;
		return Ok((before, after));
	}

	// We didn't do anything.
	Ok((before, before))
}



/// # Is Fragment.
///
/// This returns `false` if the document contains (case-insensitively)
/// `<html`, `<body`, `</body>`, or `</html>`.
fn is_fragment(src: &[u8]) -> bool {
	for w in src.windows(7) {
		if w[0] == b'<' {
			match w[1] {
				b'/' => if w[6] == b'>' {
					let mid = &w[2..6];
					if mid.eq_ignore_ascii_case(b"body") || mid.eq_ignore_ascii_case(b"html") {
						return false;
					}
				},
				b'b' | b'B' => if w[2..5].eq_ignore_ascii_case(b"ody") { return false; },
				b'h' | b'H' => if w[2..5].eq_ignore_ascii_case(b"tml") { return false; },
				_ => {},
			}
		}
	}

	true
}

/// # Make (Whole) Fragment.
///
/// The content is assumed to have been a "fragment" to begin with, but will
/// only be altered — turned back into said fragment — if both the opening and
/// closing markers are present.
///
/// Returns `false` if either marker is missing, indicating corruption.
fn make_fragment(src: &mut String) -> bool {
	if
		let Some(open) = src.find(FRAGMENT_OPEN) &&
		let Some(close) = src.rfind(FRAGMENT_CLOSE)
	{
		src.truncate(close);
		src.replace_range(..open + FRAGMENT_OPEN.len(), "");
		true
	}
	else { false }
}

/// # Make (Fragment) Whole.
///
/// Wrap fragmentary HTML in special marker tags so we can find the relevant
/// part again after processing.
fn make_whole(src: &mut String) {
	src.insert_str(0, FRAGMENT_OPEN);
	src.push_str(FRAGMENT_CLOSE);
}



#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn t_fragment() {
		assert!(
			! is_fragment(include_bytes!("../skel/test-assets/blobfolio.com.html"))
		);

		let frag = include_str!("../skel/test-assets/fragment.html");
		assert!(is_fragment(frag.as_bytes()));

		// Now make it whole.
		let mut frag2 = frag.to_owned();
		make_whole(&mut frag2);
		assert_ne!(frag, frag2); // Should be different now.

		// Now make it a fragment again.
		assert!(make_fragment(&mut frag2));
		assert!(is_fragment(frag2.as_bytes()));
		assert_eq!(frag, frag2);
	}
}
