/*!
# `HTMinL`

`HTMinL` is a fast, in-place HTML minifier written in Rust for Linux. It
prioritizes safety and code sanity over _ULTIMATE COMPRESSION_, so may not save
quite as much as libraries like Node's [html-minifier](https://github.com/kangax/html-minifier) — at least with all
the plugins enabled — but is also much less likely to break shit.

And it runs about 150x faster…

Speed, however, is not everything. Unlike virtually every other minification
tool in the wild, `HTMinL` is *not* a stream processor; it builds a complete
DOM tree from the full document code *before* getting down to the business of
minification. This understandably adds some overhead, but allows for much more
accurate processing and very robust error recovery.

Speaking of errors, if a document cannot be parsed — due to syntax or encoding
errors, etc. — or if for some reason the "minified" version winds up bigger
than the original, the original document is left as-was (i.e. no changes are
written to it).



## Use

For basic use, just toss one or more file or directory paths after the command,
like:
```bash
# Crunch one file.
htminl /path/to/one.html

# Recursively crunch every .htm(l) file in a directory.
htminl /path/to

# Do the same thing but with a progress bar.
htminl -p /path/to

# For a full list of options, run help:
htminl -h
```



## Minification

Minification is primarily achieved through (conservative) whitespace
manipulation — trimming, collapsing, or both — in text nodes, tags, and
attribute values, but only when it is judged completely safe to do so.

For example, whitespace is not altered in "value" attributes or inside elements
like `<pre>` or `<textarea>`, where it generally matters.

Speaking of "generally matters", `HTMinL` does *not* make any assumptions about
the display type of elements, as *CSS is a Thing*. Just because a `<div>` is
normally block doesn't mean someone hasn't styled one to render inline. While
this will often mean an occasional extra (unnecessary) byte, at least styled
layouts won't break willynilly!

Additional savings are achieved by stripping:
* HTML Comments;
* XML processing instructions;
* Child text nodes of `<html>` and `<head>` elements (they don't belong there!);
* Leading and trailing whitespace directly in the `<body>`;
* Whitespace in inline CSS is collapsed and trimmed (but otherwise unaltered);
* Whitespace sandwhiched between non-renderable elements like `<script>` or `<style>` tags;
* Default `type` attributes on `<script>` and `<style>` elements;
* Pointless attributes (like an empty "id" or "alt" or a falsey boolean like `hidden="false"`);
* Empty or implied attribute values;
* Leading and trailing whitespace in non-value attributes;

The above list is non-exhaustive, but hopefully you get the idea!

With the exception of CSS — which has its whitespace fully minified — inline
foreign content like Javascript and JSON are passed through unchanged. This is
one of the biggest "missed opportunities" for byte savings, but also where
minifiers tend to accidentally break things. Better a few extra bytes than a
broken page!



## Caution

While care has been taken to balance savings and safety, there are a few design
choices that could potentially break documents, worth noting before you use it:
* Documents are expected to be encoded in UTF-8. Other encodings might be OK, but some text could get garbled.
* Documents are processed as *HTML*, not XML or XHTML. Inline SVG elements should be fine, but other XML-ish data will likely be corrupted.
* Child text nodes of `<html>` and `<head>` elements are removed. Text doesn't belong there anyway, but HTML is awfully forgiving; who knows what kinds of markup will be found in the wild!
* CSS whitespace is trimmed and collapsed, which could break (very unlikely!) selectors like `input[value="Spa  ced"]`.
* Element tags are normalized, which can break fussy `camelCaseCustomElements`. (Best to write tags like `my-custom-tag` anyway...)



## Roadmap:

* Bloated inline scripts, styles, and other sorts of data — JSON, SVG, etc. —
can really add to a document's size. `HTMinL` currently applies a few (very
basic) optimizations for such content, but would benefit from crates like
[minifier-rs](https://github.com/GuillaumeGomez/minifier-rs), should they
become production-ready.

* Minification is a quest! There are endless opportunities for savings that can
be implemented into `HTMinL`; they just need to come to light!

*/

#![warn(clippy::filetype_is_file)]
#![warn(clippy::integer_division)]
#![warn(clippy::needless_borrow)]
#![warn(clippy::nursery)]
#![warn(clippy::pedantic)]
#![warn(clippy::perf)]
#![warn(clippy::suboptimal_flops)]
#![warn(clippy::unneeded_field_pattern)]
#![warn(macro_use_extern_crate)]
#![warn(missing_copy_implementations)]
#![warn(missing_debug_implementations)]
#![warn(missing_docs)]
#![warn(non_ascii_idents)]
#![warn(trivial_casts)]
#![warn(trivial_numeric_casts)]
#![warn(unreachable_pub)]
#![warn(unused_crate_dependencies)]
#![warn(unused_extern_crates)]
#![warn(unused_import_braces)]

#![allow(clippy::module_name_repetitions)]



pub mod attribute;
pub mod element;
pub mod meta;
pub mod noderef;
mod serialize;
pub mod strtendril;



use argyle::{
	Argue,
	ArgyleError,
	FLAG_HELP,
	FLAG_REQUIRED,
	FLAG_VERSION,
};
use crate::{
	meta::{a, t},
	serialize::serialize,
};
use dowser::{
	Dowser,
	utility::du,
};
use fyi_msg::{
	BeforeAfter,
	Msg,
	MsgKind,
	Progless,
};
use marked::{
	Element,
	filter::Action,
	html::parse_utf8,
	NodeData,
	NodeRef,
};
use rayon::iter::{
	IntoParallelRefIterator,
	ParallelIterator,
};
use std::{
	borrow::BorrowMut,
	cell::RefCell,
	convert::TryFrom,
	ffi::OsStr,
	fs,
	io::{
		self,
		Write,
	},
	os::unix::ffi::OsStrExt,
	path::{
		Path,
		PathBuf,
	},
};
use tendril::StrTendril;



/// Main.
fn main() {
	match _main() {
		Ok(_) => {},
		Err(ArgyleError::WantsVersion) => {
			println!(concat!("HTMinL v", env!("CARGO_PKG_VERSION")));
		},
		Err(ArgyleError::WantsHelp) => {
			helper();
		},
		Err(e) => {
			Msg::error(e).die(1);
		},
	}
}

#[inline]
/// Actual Main.
fn _main() -> Result<(), ArgyleError> {
	// Parse CLI arguments.
	let args = Argue::new(FLAG_HELP | FLAG_REQUIRED | FLAG_VERSION)?
		.with_list();

		// Put it all together!
	let paths = Vec::<PathBuf>::try_from(
		Dowser::filtered(|p: &Path| p.extension()
				.map_or(
					false,
					|e| {
						let ext = e.as_bytes().to_ascii_lowercase();
						ext == b"html" || ext == b"htm"
					}
				)
			)
			.with_paths(args.args().iter().map(|x| OsStr::from_bytes(x.as_ref())))
	).map_err(|_| ArgyleError::Custom("No documents were found."))?;

	// Sexy run-through.
	if args.switch2(b"-p", b"--progress") {
		// Boot up a progress bar.
		let progress = Progless::try_from(paths.len())
			.map_err(|_| ArgyleError::Custom("Progress can only be displayed for up to 4,294,967,295 files. Try again with fewer files or without the -p/--progress flag."))?
			.with_title(Some(Msg::custom("HTMinL", 199, "Reticulating &splines;")));

		// Check file sizes before we start.
		let mut ba = BeforeAfter::start(du(&paths));

		// Process!
		paths.par_iter().for_each(|x| {
			let tmp = x.to_string_lossy();
			progress.add(&tmp);
			minify_file(x);
			progress.remove(&tmp);
		});

		// Check file sizes again.
		ba.stop(du(&paths));

		// Finish up.
		progress.finish();
		progress.summary(MsgKind::Crunched, "document", "documents")
			.with_bytes_saved(ba.less(), ba.less_percent())
			.print();
	}
	else {
		paths.par_iter().for_each(|x| {
			minify_file(x);
		});
	}

	Ok(())
}

#[allow(unused_must_use)]
/// Do the dirty work!
fn minify_file(path: &Path) {
	let _res = fs::read(path)
		.and_then(|mut data| minify_html(&mut data)
			.and_then(|_| tempfile_fast::Sponge::new_for(path))
			.and_then(|mut out| out.write_all(&data).and_then(|_| out.commit()))
		);
}

/// Minify HTML.
///
/// This convenience method minifies (in-place) the HTML source in the byte
/// vector, and returns the size savings (if any).
///
/// ## Errors
///
/// This method returns an error if the file is invalid, empty, or minification
/// otherwise fails, including cases where everything worked but no compression
/// was possible.
pub fn minify_html(mut data: &mut Vec<u8>) -> io::Result<usize> {
	use regex::bytes::Regex;

	lazy_static::lazy_static! {
		static ref RE_HAS_HTML: Regex = Regex::new(r"(?i)(<html|<body|</body>|</html>)").unwrap();
	}

	// We need something to encode!
	if data.is_empty() {
		return Err(io::ErrorKind::WriteZero.into());
	}

	// Note our starting length.
	let old_len: usize = data.len();

	// Is this a fragment? If so, we'll wrap it in a temporary scaffold so the
	// DOM tree can be built.
	let fragment: bool = ! RE_HAS_HTML.is_match(data);
	if fragment {
		unsafe { prepend_slice(&mut data, b"<html><head></head><body>"); }
		data.extend_from_slice(b"</body></html>");
	}

	// Parse the document.
	let mut doc = parse_utf8(data);

	// First Pass: clean up elements, strip pointless nodes.
	// Second Pass: merge adjacent text nodes.
	// Third Pass: clean up and minify text nodes.
	doc.filter(filter_minify_one);
	doc.filter(filter_minify_two);
	doc.filter(filter_minify_three);

	// Save it!
	data.truncate(0);
	serialize(&mut data, &doc.document_node_ref())?;

	// Chop off the fragment scaffold, if present.
	if fragment {
		if
			data.starts_with(b"<html><head></head><body>") &&
			data.ends_with(b"</body></html>")
		{
			data.drain(0..25);
			data.truncate(data.len() - 14);
		}
		// Something went weird.
		else {
			return Err(io::ErrorKind::UnexpectedEof.into());
		}
	}

	// Return the amount saved.
	let new_len: usize = data.len();
	if new_len >= old_len {
		Err(io::ErrorKind::Other.into())
	}
	else { Ok(old_len - new_len) }
}

/// Minify #1
///
/// This strips comments and XML processing instructions, removes default type
/// attributes for scripts and styles, and removes truthy attribute values for
/// boolean properties like "defer" and "disabled".
pub fn filter_minify_one(node: NodeRef<'_>, data: &mut NodeData) -> Action {
	match data {
		NodeData::Elem(Element { name, attrs, .. }) => {
			let mut len: usize = attrs.len();
			let mut idx: usize = 0;

			while idx < len {
				// Almost always trim...
				if attrs[idx].name.local != a::VALUE {
					strtendril::trim(&mut attrs[idx].value);
				}

				// Drop the whole thing.
				if attribute::can_drop(&attrs[idx], &name.local) {
					attrs.remove(idx);
					len -= 1;
					continue;
				}

				// Drop the value.
				if attribute::can_drop_value(&attrs[idx]) {
					attrs[idx].value = StrTendril::new();
				}
				// Compact the value.
				else if attribute::can_compact_value(&attrs[idx]) {
					strtendril::collapse_whitespace(&mut attrs[idx].value);
				}

				idx += 1;
			}

			Action::Continue
		},
		NodeData::Text(_) => {
			// We never need text nodes in the `<head>` or `<html>`.
			if node.parent()
				.as_deref()
				.and_then(|p| p.as_element())
				.filter(|el| element::can_drop_text_nodes(el))
				.is_some()
			{
				return Action::Detach;
			}

			Action::Continue
		},
		// Remove comments and XML processing instructions.
		NodeData::Comment(_) | NodeData::Pi(_) => Action::Detach,
		// Whatever else there is can pass through unchanged.
		_ => Action::Continue,
	}
}

/// Minify #2
///
/// This pass merges adjacent text nodes. The code is identical to what
/// `marked` exports under `text_normalize`, except it does not mess about with
/// whitespace. (We do that later, with greater intention.)
pub fn filter_minify_two(pos: NodeRef<'_>, data: &mut NodeData) -> Action {
    thread_local! {
        static MERGE_Q: RefCell<StrTendril> = RefCell::new(StrTendril::new())
    };

    if let Some(t) = data.as_text_mut() {
        // If the immediately following sibling is also text, then push this
        // tendril to the merge queue and detach.
        let node_r = pos.next_sibling();
        if node_r.map_or(false, |n| n.as_text().is_some()) {
            MERGE_Q.with(|q| {
                q.borrow_mut().push_tendril(t)
            });
            return Action::Detach;
        }

        // Otherwise add this tendril to anything in the queue, consuming it.
        MERGE_Q.with(|q| {
            let mut qt = q.borrow_mut();
            if qt.len() > 0 {
                qt.push_tendril(t);
                drop(qt);
                *t = q.replace(StrTendril::new());
            }
        });

        if t.is_empty() {
            return Action::Detach;
        }
    }

    Action::Continue
}

#[allow(clippy::suspicious_else_formatting)] // Sorry not sorry.
/// Minify #3
///
/// This pass cleans up text nodes, collapsing whitespace (when it is safe to
/// do so), and trimming a bit (also only when safe).
///
/// See `collapse_whitespace` for more details.
pub fn filter_minify_three(node: NodeRef<'_>, data: &mut NodeData) -> Action {
	if let Some((txt, el)) = data.as_text_mut()
		.zip(node.parent().as_deref().and_then(|p| p.as_element()))
	{
		// Special cases.
		if strtendril::is_whitespace(txt) && noderef::can_drop_if_whitespace(&node) {
			return Action::Detach;
		}

		// Can we trim the text?
		if element::can_trim_whitespace(el) {
			strtendril::trim(txt.borrow_mut());
		}

		// How about collapse it?
		if element::can_collapse_whitespace(el) {
			strtendril::collapse_whitespace(txt.borrow_mut());

			// If the body starts or ends with a text node, we can trim it
			// from the left or the right respectively.
			if el.is_elem(t::BODY) {
				// Drop the start.
				if txt.starts_with(' ') && noderef::is_first_child(&node) {
					txt.pop_front(1);
				}
				// Drop the end.
				if txt.ends_with(' ') && noderef::is_last_child(&node) {
					txt.pop_back(1);
				}
			}
		}

		// Drop empty nodes entirely.
		if txt.is_empty() {
			return Action::Detach;
		}
	}

	Action::Continue
}

/// # Prepend Data to Vec.
///
/// Insert a slice into the beginning of an array.
///
/// ## Safety
///
/// This copies data with pointers, but allocates as needed so should be fine.
unsafe fn prepend_slice<T: Copy>(vec: &mut Vec<T>, slice: &[T]) {
	use std::ptr;

	let len = vec.len();
	let amt = slice.len();
	vec.reserve(amt);

	ptr::copy(
		vec.as_ptr(),
		vec.as_mut_ptr().add(amt),
		len
	);
	ptr::copy(
		slice.as_ptr(),
		vec.as_mut_ptr(),
		amt
	);
	vec.set_len(len + amt);
}

#[allow(clippy::non_ascii_literal)] // Doesn't work with an r"" literal.
#[cold]
/// Print Help.
fn helper() {
	println!(concat!(
		r"
     __,---.__
  ,-'         `-.__
&/           `._\ _\
/               ''._    ", "\x1b[38;5;199mHTMinL\x1b[0;38;5;69m v", env!("CARGO_PKG_VERSION"), "\x1b[0m", r"
|   ,             (∞)   Fast, safe, in-place
|__,'`-..--|__|--''     HTML minification.

USAGE:
    htminl [FLAGS] [OPTIONS] <PATH(S)>...

FLAGS:
    -h, --help        Prints help information
    -p, --progress    Show progress bar while minifying.
    -V, --version     Prints version information

OPTIONS:
    -l, --list <list>    Read file paths from this list.

ARGS:
    <PATH(S)>...    One or more files or directories to compress.
"
	));
}
