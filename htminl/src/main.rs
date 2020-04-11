/*!
# HTMinL

In-place minification of HTML file(s).
*/

#![warn(missing_docs)]
#![warn(trivial_casts)]
#![warn(trivial_numeric_casts)]
#![warn(unused_import_braces)]

#![deny(missing_copy_implementations)]
#![deny(missing_debug_implementations)]

extern crate clap;
extern crate fyi_core;
extern crate hyperbuild;

mod menu;

use clap::ArgMatches;
use fyi_core::{
	traits::path::FYIPathIO,
	Witch,
};
use hyperbuild::hyperbuild;
use std::path::PathBuf;



fn main() -> Result<(), String> {
	// Command line arguments.
	let opts: ArgMatches = menu::menu()
		.get_matches();

	// What path are we dealing with?
	let walk: Witch = match opts.is_present("list") {
		false => {
			let paths: Vec<PathBuf> = opts.values_of("path").unwrap()
				.into_iter()
				.map(|x| PathBuf::from(x))
				.collect();

			Witch::new(
				&paths,
				Some(r"(?i).+\.html?$".to_string())
			)
		},
		true => {
			let path = PathBuf::from(opts.value_of("list").unwrap_or(""));
			Witch::from_file(
				&path,
				Some(r"(?i).+\.html?$".to_string())
			)
		},
	};

	if walk.is_empty() {
		return Err("No encodable files were found.".to_string());
	}

	// With progress.
	if opts.is_present("progress") {
		walk.progress_crunch("HTMinL", |x| {
			let _ = x.encode().is_ok();
		});
	}
	// Without progress.
	else {
		walk.process(|ref x| {
			let _ = x.encode().is_ok();
		});
	}

	Ok(())
}

/// Encoding!
pub trait HTMinLEncode {
	/// Encode.
	fn encode(&self) -> Result<(), String>;
}

impl HTMinLEncode for PathBuf {
	/// Encode.
	fn encode(&self) -> Result<(), String> {
		// Load it.
		let mut data = self.fyi_read()?;

		if let Ok(len) = hyperbuild(&mut data) {
			// Save it?
			if 0 < len {
				self.fyi_write(&data[..len])?;
			}

			return Ok(());
		}

		Err("Unable to minify file.".into())
	}
}
