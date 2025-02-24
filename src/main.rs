/*!
# HTMinL
*/

#![forbid(unsafe_code)]

#![deny(
	clippy::allow_attributes_without_reason,
	clippy::correctness,
	unreachable_pub,
)]

#![warn(
	clippy::complexity,
	clippy::nursery,
	clippy::pedantic,
	clippy::perf,
	clippy::style,

	clippy::allow_attributes,
	clippy::clone_on_ref_ptr,
	clippy::create_dir,
	clippy::filetype_is_file,
	clippy::format_push_string,
	clippy::get_unwrap,
	clippy::impl_trait_in_params,
	clippy::lossy_float_literal,
	clippy::missing_assert_message,
	clippy::missing_docs_in_private_items,
	clippy::needless_raw_strings,
	clippy::panic_in_result_fn,
	clippy::pub_without_shorthand,
	clippy::rest_pat_in_fully_bound_structs,
	clippy::semicolon_inside_block,
	clippy::str_to_string,
	clippy::string_to_string,
	clippy::todo,
	clippy::undocumented_unsafe_blocks,
	clippy::unneeded_field_pattern,
	clippy::unseparated_literal_suffix,
	clippy::unwrap_in_result,

	macro_use_extern_crate,
	missing_copy_implementations,
	missing_docs,
	non_ascii_idents,
	trivial_casts,
	trivial_numeric_casts,
	unused_crate_dependencies,
	unused_extern_crates,
	unused_import_braces,
)]

#![expect(clippy::redundant_pub_crate, reason = "Unresolvable.")]
#![expect(clippy::doc_markdown, reason = "HTMinL makes this annoying.")]



mod error;
mod htminl;

use argyle::Argument;
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
	process::ExitCode,
	sync::atomic::{
		AtomicU64,
		Ordering::Relaxed,
	},
};



// The E_HTML, E_HTM constants are generated by build.rs.
include!(concat!(env!("OUT_DIR"), "/htminl-extensions.rs"));



/// # Main.
fn main() -> ExitCode {
	match main__() {
		Ok(()) => ExitCode::SUCCESS,
		Err(e @ (HtminlError::PrintHelp | HtminlError::PrintVersion)) => {
			println!("{e}");
			ExitCode::SUCCESS
		},
		Err(e) => {
			Msg::error(e.to_string()).eprint();
			ExitCode::FAILURE
		},
	}
}

#[inline]
/// # Actual Main.
fn main__() -> Result<(), HtminlError> {
	// Parse CLI arguments.
	let args = argyle::args()
		.with_keywords(include!(concat!(env!("OUT_DIR"), "/argyle.rs")));

	let mut progress = false;
	let mut paths = Dowser::default();
	for arg in args {
		match arg {
			Argument::Key("-h" | "--help") => return Err(HtminlError::PrintHelp),
			Argument::Key("-p" | "--progress") => { progress = true; },
			Argument::Key("-V" | "--version") => return Err(HtminlError::PrintVersion),

			Argument::KeyWithValue("-l" | "--list", s) => {
				paths.read_paths_from_file(&s).map_err(|_| HtminlError::ListFile)?;
			},

			Argument::Path(s) => { paths = paths.with_path(s); },

			// Mistake?
			Argument::Other(s) => return Err(HtminlError::InvalidCli(s)),
			Argument::InvalidUtf8(s) => return Err(HtminlError::InvalidCli(s.to_string_lossy().into_owned())),

			// Nothing else is relevant.
			_ => {},
		}
	}

	// Put it all together!
	let paths: Vec<PathBuf> = paths.into_vec_filtered(|p|
		Extension::try_from4(p).map_or_else(
			|| Some(E_HTM) == Extension::try_from3(p),
			|e| e == E_HTML
		)
	);

	if paths.is_empty() { return Err(HtminlError::NoDocuments); }

	// Sexy run-through.
	if progress {
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
