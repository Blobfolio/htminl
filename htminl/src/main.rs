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
	Error,
	Result,
};
use fyi_witch::{
	traits::WitchIO,
	Witch,
};
use hyperbuild::hyperbuild;
use std::path::PathBuf;



fn main() -> Result<()> {
	// Command line arguments.
	let opts: ArgMatches = menu::menu()
		.get_matches();

	// What path are we dealing with?
	let walk: Witch = match opts.is_present("list") {
		false => Witch::new(
			&opts.values_of("path")
				.unwrap()
				.collect::<Vec<&str>>(),
			Some(r"(?i).+\.html?$".to_string())
		),
		true => Witch::from_file(
			opts.value_of("list").unwrap_or(""),
			Some(r"(?i).+\.html?$".to_string())
		),
	};

	if walk.is_empty() {
		return Err(Error::new("No encodable files found."));
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
	fn encode(&self) -> Result<()>;
}

impl HTMinLEncode for PathBuf {
	/// Encode.
	fn encode(&self) -> Result<()> {
		// Load it.
		let mut data = self.witch_read()?;

		if let Ok(len) = hyperbuild(&mut data) {
			// Save it?
			if 0 < len {
				self.witch_write(&data[..len])?;
			}

			return Ok(());
		}

		Err(Error::new(format!("Unable to minify {:?}.", self.to_path_buf())))
	}
}
