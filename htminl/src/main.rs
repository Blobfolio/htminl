/*!
# `HTMinL`

In-place minification of HTML file(s).
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
	traits::WitchIO,
	Witcher,
};
use hyperbuild::hyperbuild;
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
		walk.process(minify_html);
	}
	// With progress.
	else {
		walk.progress("HTMinL", minify_html);
	}

	Ok(())
}

#[allow(unused_must_use)]
/// Do the dirty work!
fn minify_html(path: &PathBuf) {
	if let Ok(mut data) = fs::read(path) {
		let old_len: usize = data.len();
		if 0 != old_len {
			if let Ok(len) = hyperbuild(&mut data) {
				if 0 < len && len < old_len {
					path.witch_write(&data[..len]);
				}
			}
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
