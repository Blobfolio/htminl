/*!
# `HTMinL`
*/

#![forbid(unsafe_code)]

#![warn(
	clippy::filetype_is_file,
	clippy::integer_division,
	clippy::needless_borrow,
	clippy::nursery,
	clippy::pedantic,
	clippy::perf,
	clippy::suboptimal_flops,
	clippy::unneeded_field_pattern,
	macro_use_extern_crate,
	missing_copy_implementations,
	missing_debug_implementations,
	missing_docs,
	non_ascii_idents,
	trivial_casts,
	trivial_numeric_casts,
	unreachable_pub,
	unused_crate_dependencies,
	unused_extern_crates,
	unused_import_braces,
)]

#![allow(
	clippy::module_name_repetitions,
	clippy::redundant_pub_crate,
)]



mod error;
mod htminl;

use argyle::{
	Argue,
	ArgyleError,
	FLAG_HELP,
	FLAG_REQUIRED,
	FLAG_VERSION,
};
use dowser::{
	Dowser,
	Extension,
};
pub(crate) use error::HtminlError;
use fyi_msg::{
	BeforeAfter,
	Msg,
	MsgKind,
	Progless,
};
use rayon::iter::{
	IntoParallelRefIterator,
	ParallelIterator,
};
use std::{
	path::PathBuf,
	sync::atomic::{
		AtomicU64,
		Ordering::Relaxed,
	},
};



// The E_HTML, E_HTM constants are generated by build.rs.
include!(concat!(env!("OUT_DIR"), "/htminl-extensions.rs"));



/// # Main.
fn main() {
	match _main() {
		Ok(_) => {},
		Err(HtminlError::Argue(ArgyleError::WantsVersion)) => {
			println!(concat!("HTMinL v", env!("CARGO_PKG_VERSION")));
		},
		Err(HtminlError::Argue(ArgyleError::WantsHelp)) => {
			helper();
		},
		Err(e) => {
			Msg::error(e).die(1);
		},
	}
}

#[inline]
/// # Actual Main.
fn _main() -> Result<(), HtminlError> {
	// Parse CLI arguments.
	let args = Argue::new(FLAG_HELP | FLAG_REQUIRED | FLAG_VERSION)?
		.with_list();

	// Put it all together!
	let paths: Vec<PathBuf> = Dowser::default()
		.with_paths(args.args_os())
		.into_vec(|p|
			Extension::try_from4(p).map_or_else(
				|| Some(E_HTM) == Extension::try_from3(p),
				|e| e == E_HTML
			)
		);

	if paths.is_empty() {
		return Err(HtminlError::NoDocuments);
	}

	// Sexy run-through.
	if args.switch2(b"-p", b"--progress") {
		// Boot up a progress bar.
		let progress = Progless::try_from(paths.len())?
			.with_title(Some(Msg::custom("HTMinL", 199, "Reticulating &splines;")));

		// Check file sizes before we start.
		let before = AtomicU64::new(0);
		let after = AtomicU64::new(0);

		// Process!
		paths.par_iter().for_each(|x|
			if let Ok(mut enc) = htminl::Htminl::try_from(x) {
				let tmp = x.to_string_lossy();
				progress.add(&tmp);

				if let Ok((b, a)) = enc.minify() {
					before.fetch_add(b, Relaxed);
					after.fetch_add(a, Relaxed);
				}
				else {
					before.fetch_add(enc.size, Relaxed);
					after.fetch_add(enc.size, Relaxed);
				}

				progress.remove(&tmp);
			}
		);

		// Finish up.
		progress.finish();
		progress.summary(MsgKind::Crunched, "document", "documents")
			.with_bytes_saved(BeforeAfter::from((
				before.into_inner(),
				after.into_inner(),
			)))
			.print();
	}
	else {
		paths.par_iter().for_each(|x|
			if let Ok(mut enc) = htminl::Htminl::try_from(x) {
				let _res = enc.minify();
			}
		);
	}

	Ok(())
}

#[allow(clippy::non_ascii_literal)] // Doesn't work with an r"" literal.
#[cold]
/// # Print Help.
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
    -h, --help        Print help information and exit.
    -p, --progress    Show progress bar while minifying.
    -V, --version     Print program version and exit.

OPTIONS:
    -l, --list <FILE> Read (absolute) file and/or directory paths from this
                      text file, one entry per line.

ARGS:
    <PATH(S)>...      One or more files or directories to compress.
"
	));
}
