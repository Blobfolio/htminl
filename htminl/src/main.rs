/*!
# `HTMinL`

`HTMinL` is a fast, in-place HTML minifier written in Rust. It prioritizes
safety and code sanity over _ULTIMATE COMPRESSION_, so may not save quite as
much as libraries like Node's [html-minifier](https://github.com/kangax/html-minifier),
but on the other hand, it is much less likely to break shit, and is about 100x
faster.

`HTMinL` is *not* a stream processor; it parses the document in its entirety
into a DOM tree powered by Mozilla's Servo engine before meddling with the
contents. This adds some overhead not found in naive, Regex-based processors,
but ultimately allows for much more robust error correction, similar to what
browser software is capable of.

In the event syntax is sufficiently broken as to prevent tree parsing, or in
the unlikely event the resulting "minified" document is bigger than the
original, no changes are written.

## Minification

Minification is primarily achieved through whitespace collapsing — converting
all contiguous whitespace sequences (except `&nbsp;`) to a single horizontal
space. This is HTML is rendered anyway, so the extra spaces are only so much
document bloat.

Care is taken to avoid modifying whitespace inside elements like `<textarea>`
and `<pre>` — where it might matter! — as well as unknown elements, such as
custom web components.

Trimming — chopping _all_ leading and/or trailing whitespace — is applied in a
_few_ special cases where it is completely safe to do so, but unlike most HTML
minifiers, no assumptions are made about "inline" versus "block" elements as
CSS allows anything to be anything! As a result, individual (i.e. collapsed)
spaces will often be left in and around tags, but the resulting document
layout should render as intended when viewed in a browser.

Additional savings are achieved by stripping:
* Comments;
* XML processing instructions;
* Text nodes residing in `<html>` and `<head>` elements;
* Default `type` attributes on `<script>` and `<style>` elements;
* Empty attribute values;
* Values from boolean attributes like `hidden` and `disabled`;
* Space between `<pre>` and `<code>` tags;
* Leading and trailing space directly in the `<body>`;
* Closing `</path>` tags in inline SVG blocks.

While care has been taken to balance savings and safety, there are a few design
choices that could potentially break documents, worth noting before you use it:
* All documents are parsed as `UTF-8`; if you code for Windows or something weird, you might wind up with malformed text;
* Documents are processed as *HTML*, not XML or XHTML. While `SVG` elements should come through OK, other types of markup may not;
* As mentioned above, `HTMinL` does not allow free-range text inside the `<head>`, or as a direct child of `<html>`. That's what `<body>` is for!
* Whitespace inside `<style>` tags is collapsed and normalized, which could alter (unlikely) code like `[name="spa   ced"]`;

## TODO:
* Optional CSS minification;
* Optional JS minification;
* Optional JSON minification;
* Investigate other simple savings?
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
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]



use fyi_menu::ArgList;
use fyi_witcher::{
	Result,
	Witcher,
};
use std::{
	fs,
	io::{
		self,
		Write,
	},
	path::PathBuf,
};



/// -h | --help
const FLAG_HELP: u8     = 0b0001;
/// -p | --progress
const FLAG_PROGRESS: u8 = 0b0010;
/// -V | --version
const FLAG_VERSION: u8  = 0b0100;



fn main() -> Result<()> {
	let mut args = ArgList::default();
	args.expect();

	let flags = _flags(&mut args);
	// Help or Version?
	if 0 != flags & FLAG_HELP {
		_help();
		return Ok(());
	}
	else if 0 != flags & FLAG_VERSION {
		_version();
		return Ok(());
	}

	// What path are we dealing with?
	let walk = match args.pluck_opt(|x| x == "-l" || x == "--list") {
		Some(p) => Witcher::from_file(p, r"(?i).+\.html?$"),
		None => Witcher::new(&args.expect_args(), r"(?i).+\.html?$"),
	};

	if walk.is_empty() {
		return Err("No HTML files were found.".to_string());
	}

	// Without progress.
	if 0 == flags & FLAG_PROGRESS {
		walk.process(minify_file);
	}
	// With progress.
	else {
		walk.progress("HTMinL", minify_file);
	}

	Ok(())
}

#[allow(unused_must_use)]
/// Do the dirty work!
fn minify_file(path: &PathBuf) {
	if let Ok(mut data) = fs::read(path) {
		if htminl::minify_html(&mut data).is_ok() {
			//println!("Saved {:?}\n\n", size);
			//println!("{}", unsafe{ std::str::from_utf8_unchecked(&data) });
			let mut out = tempfile_fast::Sponge::new_for(path).unwrap();
			out.write_all(&data).unwrap();
			out.commit().unwrap();
		}
	}
}

/// Fetch Flags.
fn _flags(args: &mut ArgList) -> u8 {
	let len: usize = args.len();
	if 0 == len { 0 }
	else {
		let mut flags: u8 = 0;
		let mut del = 0;
		let raw = args.as_mut_vec();

		// This is basically what `Vec.retain()` does, except we're hitting
		// multiple patterns at once and sending back the results.
		let ptr = raw.as_mut_ptr();
		unsafe {
			let mut idx: usize = 0;
			while idx < len {
				match (*ptr.add(idx)).as_str() {
					"-h" | "--help" => {
						flags |= FLAG_HELP;
						del += 1;
					},
					"-p" | "--progress" => {
						flags |= FLAG_PROGRESS;
						del += 1;
					},
					"-V" | "--version" => {
						flags |= FLAG_VERSION;
						del += 1;
					},
					_ => if del > 0 {
						ptr.add(idx).swap(ptr.add(idx - del));
					}
				}

				idx += 1;
			}
		}

		// Did we find anything? If so, run `truncate()` to free the memory
		// and return the flags.
		if del > 0 {
			raw.truncate(len - del);
			flags
		}
		else { 0 }
	}
}

#[cold]
/// Print Help.
fn _help() {
	io::stdout().write_all({
		let mut s = String::with_capacity(1024);
		s.push_str("HTMinL ");
		s.push_str(env!("CARGO_PKG_VERSION"));
		s.push('\n');
		s.push_str(env!("CARGO_PKG_DESCRIPTION"));
		s.push('\n');
		s.push('\n');
		s.push_str(include_str!("../misc/help.txt"));
		s.push('\n');
		s
	}.as_bytes()).unwrap();
}

#[cold]
/// Print version and exit.
fn _version() {
	io::stdout().write_all({
		let mut s = String::from("HTMinL ");
		s.push_str(env!("CARGO_PKG_VERSION"));
		s.push('\n');
		s
	}.as_bytes()).unwrap();
}
