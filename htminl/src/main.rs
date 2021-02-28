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



use argyle::{
	Argue,
	ArgyleError,
	FLAG_HELP,
	FLAG_REQUIRED,
	FLAG_VERSION,
};
use dactyl::NiceU32;
use dowser::{
	Dowser,
	utility::du,
};
use fyi_msg::{
	Msg,
	Progless,
};
use rayon::iter::{
	IntoParallelRefIterator,
	ParallelIterator,
};
use std::{
	convert::TryFrom,
	ffi::OsStr,
	fs,
	io::Write,
	os::unix::ffi::OsStrExt,
	path::{
		Path,
		PathBuf,
	},
};



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
		Dowser::default()
			.with_filter(|p: &Path| p.extension()
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
		use num_traits::cast::FromPrimitive;

		// Check file sizes before we start.
		let before: u64 = du(&paths);
		let len: u32 = u32::try_from(paths.len())
			.map_err(|_| ArgyleError::Custom("Only 4,294,967,295 files can be crunched at one time."))?;

		// Boot up a progress bar.
		let progress = Progless::steady(len)
			.with_title(Some(Msg::custom("HTMinL", 199, "Reticulating &splines;")));

		// Process!
		paths.par_iter().for_each(|x| {
			let tmp = x.to_string_lossy();
			progress.add(&tmp);
			minify_file(x);
			progress.remove(&tmp);
		});

		// Finish up.
		let elapsed = progress.finish();
		let after: u64 = du(&paths);

		let saved: u64 =
			if after > 0 && after < before { before - after }
			else { 0 };

		let percent: Option<f64> =
			if saved > 0 {
				f64::from_u64(before - after)
					.zip(f64::from_u64(before))
					.map(|(a, b)| a / b)
			}
			else { None };

		// Print a summary!
		unsafe {
			Msg::crunched_unchecked(&[
				NiceU32::from(len).as_bytes(),
				if len == 1 { b" document in " } else { b" documents in " },
				elapsed.as_bytes(),
				b".",
			].concat())
		}
			.with_bytes_saved(saved, percent)
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
		.and_then(|mut data| htminl_core::minify_html(&mut data)
			.and_then(|_| tempfile_fast::Sponge::new_for(path))
			.and_then(|mut out| out.write_all(&data).and_then(|_| out.commit()))
		);
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
