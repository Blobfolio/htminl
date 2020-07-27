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
layouts wont' break willynilly!

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

#![warn(missing_docs)]
#![warn(trivial_casts)]
#![warn(trivial_numeric_casts)]
#![warn(unused_import_braces)]

#![deny(missing_copy_implementations)]
#![deny(missing_debug_implementations)]

#![allow(clippy::unknown_clippy_lints)]

#![warn(clippy::filetype_is_file)]
#![warn(clippy::integer_division)]
#![warn(clippy::needless_borrow)]
#![warn(clippy::nursery)]
#![warn(clippy::pedantic)]
#![warn(clippy::perf)]
#![warn(clippy::suboptimal_flops)]
#![warn(clippy::unneeded_field_pattern)]

#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::match_like_matches_macro)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::unknown_clippy_lints)]



use fyi_menu::ArgList;
use fyi_msg::MsgKind;
use fyi_witcher::{
	Result,
	Witcher,
};
use std::{
	ffi::OsStr,
	fs,
	io::{
		self,
		Write,
	},
	path::PathBuf,
	process,
};



/// -h | --help
const FLAG_HELP: u8     = 0b0001;
/// -p | --progress
const FLAG_PROGRESS: u8 = 0b0010;
/// -V | --version
const FLAG_VERSION: u8  = 0b0100;



#[allow(clippy::if_not_else)] // Code is confusing otherwise.
fn main() {
	let mut args = ArgList::default();
	args.expect();

	let mut flags: u8 = 0;
	args.pluck_flags(
		&mut flags,
		&[
			"-h", "--help",
			"-p", "--progress",
			"-V", "--version",
		],
		&[
			FLAG_HELP, FLAG_HELP,
			FLAG_PROGRESS, FLAG_PROGRESS,
			FLAG_VERSION, FLAG_VERSION,
		],
	);

	// Help.
	if 0 != flags & FLAG_HELP {
		_help();
	}
	// Version.
	else if 0 != flags & FLAG_VERSION {
		_version();
	}
	// Actual stuff!
	else {
		// What path are we dealing with?
		let walk = match args.pluck_opt(|x| x == "-l" || x == "--list") {
			Some(p) => unsafe { Witcher::from_file_custom(p, witch_filter) },
			None => unsafe { Witcher::custom(&args.expect_args(), witch_filter) },
		};

		if walk.is_empty() {
			MsgKind::Error.as_msg("No encodable files were found.").eprintln();
			process::exit(1);
		}
		// Without progress.
		else if 0 == flags & FLAG_PROGRESS {
			walk.process(minify_file);
		}
		// With progress.
		else {
			walk.progress("HTMinL", minify_file);
		}
	}
}

#[allow(clippy::needless_pass_by_value)] // Would if it were the expected signature!
#[allow(trivial_casts)] // Trivial though it may be, the code doesn't work without it!
/// Accept or Deny Files.
fn witch_filter(res: Result<jwalk::DirEntry<((), ())>, jwalk::Error>) -> Option<PathBuf> {
	res.ok()
		.and_then(|p| if p.file_type().is_dir() { None } else { Some(p) })
		.and_then(|p| fs::canonicalize(p.path()).ok())
		.and_then(|p| {
			let bytes: &[u8] = unsafe { &*(p.as_os_str() as *const OsStr as *const [u8]) };
			let len: usize = bytes.len();
			if
				len > 5 &&
				(
					bytes[len-5..len].eq_ignore_ascii_case(b".html") ||
					bytes[len-4..len].eq_ignore_ascii_case(b".htm")
				)
			{ Some(p) }
			else { None }
		})
}

#[allow(unused_must_use)]
/// Do the dirty work!
fn minify_file(path: &PathBuf) {
	if let Ok(mut data) = fs::read(path) {
		if htminl::minify_html(&mut data).is_ok() {
			let mut out = tempfile_fast::Sponge::new_for(path).unwrap();
			out.write_all(&data).unwrap();
			out.commit().unwrap();
		}
	}
}

#[cfg(not(feature = "man"))]
#[cold]
/// Print Help.
fn _help() {
	io::stdout().write_fmt(format_args!(
		r"
     __,---.__
  ,-'         `-.__
&/           `._\ _\
/               ''._    {}{}{}
|   ,             (∞)   Fast, safe, in-place
|__,'`-..--|__|--''     HTML minification.

{}\n",
		"\x1b[38;5;199mHTMinL\x1b[0;38;5;69m v",
		env!("CARGO_PKG_VERSION"),
		"\x1b[0m",
		include_str!("../misc/help.txt")
	)).unwrap();
}

#[cfg(feature = "man")]
#[cold]
/// Print Help.
///
/// This is a stripped-down version of the help screen made specifically for
/// `help2man`, which gets run during the Debian package release build task.
fn _help() {
	io::stdout().write_all(&[
		b"HTMinL ",
		env!("CARGO_PKG_VERSION").as_bytes(),
		b"\n",
		env!("CARGO_PKG_DESCRIPTION").as_bytes(),
		b"\n\n",
		include_bytes!("../misc/help.txt"),
		b"\n",
	].concat()).unwrap();
}

#[cold]
/// Print version and exit.
fn _version() {
	io::stdout().write_all(&[
		b"HTMinL ",
		env!("CARGO_PKG_VERSION").as_bytes(),
		b"\n"
	].concat()).unwrap();
}
