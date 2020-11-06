#[cfg(not(feature = "man"))]
/// # Do Nothing.
///
/// We only need to rebuild stuff for new releases. The "man" feature is
/// basically used to figure that out.
fn main() {}



#[cfg(feature = "man")]
/// # Build.
fn main() {
	use fyi_menu::{
		Agree,
		AgreeKind,
	};
	use std::{
		env,
		path::PathBuf,
	};

	let app: Agree = Agree::new(
		"HTMinL",
		env!("CARGO_PKG_NAME"),
		env!("CARGO_PKG_VERSION"),
		env!("CARGO_PKG_DESCRIPTION"),
	)
		.with_arg(
			AgreeKind::switch("Print help information.")
				.with_short("-h")
				.with_long("--help")
		)
		.with_arg(
			AgreeKind::switch("Show progress bar while working.")
				.with_short("-p")
				.with_long("--progress")
		)
		.with_arg(
			AgreeKind::switch("Print program version.")
				.with_short("-V")
				.with_long("--version")
		)
		.with_arg(
			AgreeKind::option("<FILE>", "Read file paths from this text file.", true)
				.with_short("-l")
				.with_long("--list")
		)
		.with_arg(
			AgreeKind::arg("<PATH(s)…>", "Any number of files and directories to crawl and crunch.")
		);

	// Our files will go to ./misc.
	let mut path: PathBuf = env::var("CARGO_MANIFEST_DIR")
		.ok()
		.and_then(|x| std::fs::canonicalize(x).ok())
		.expect("Missing output directory.");

	path.push("misc");

	// Write 'em!
	app.write_bash(&path)
		.unwrap_or_else(|_| panic!("Unable to write BASH completion script: {:?}", path));
	app.write_man(&path)
		.unwrap_or_else(|_| panic!("Unable to write MAN page: {:?}", path));
}
