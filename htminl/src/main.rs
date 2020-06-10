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



mod menu;

use clap::ArgMatches;
use fyi_witcher::{
	Result,
	traits::WitchIO,
	Witcher,
};
use hyperbuild::hyperbuild;
use std::{
	fs,
	path::PathBuf,
};



fn main() -> Result<()> {
	// Command line arguments.
	let opts: ArgMatches = menu::menu()
		.get_matches();

	// What path are we dealing with?
	let walk = if opts.is_present("list") {
		Witcher::from_file(
			opts.value_of("list").unwrap_or(""),
			r"(?i).+\.html?$"
		)
	}
	else {
		Witcher::new(
			&opts.values_of("path")
				.unwrap()
				.collect::<Vec<&str>>(),
			r"(?i).+\.html?$"
		)
	};

	if walk.is_empty() {
		return Err("No HTML files were found.".to_string());
	}

	// With progress.
	if opts.is_present("progress") {
		walk.progress("HTMinL", encode_path);
	}
	// Without progress.
	else {
		walk.process(encode_path);
	}

	Ok(())
}



#[allow(unused_must_use)]
// Do the dirty work!
fn encode_path(path: &PathBuf) {
	if let Ok(mut data) = fs::read(path) {
		if ! data.is_empty() {
			if let Ok(len) = hyperbuild(&mut data) {
				// Save it?
				if 0 < len {
					path.witch_write(&data[..len]);
				}
			}
		}
	}
}
