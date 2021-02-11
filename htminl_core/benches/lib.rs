/*!
# Benchmark: `htminl`
*/

use fyi_bench::{
	Bench,
	benches,
};
use htminl_core::strtendril;
use tendril::StrTendril;

benches!(
	Bench::new("htminl", "collapse_whitespace()")
		.with_setup(
			StrTendril::from("My name is 	Jeffrey.\n\nI like flowers!     "),
			|mut st| {
				strtendril::collapse_whitespace(&mut st);
				st.len()
			}
		),

	Bench::new("htminl", "minify_html()")
		.with_setup(
			std::fs::read("./test-assets/blobfolio.com.html").expect("Unable to open test file."),
			|mut t| {
				htminl_core::minify_html(&mut t).expect("Fail!");
				t.len()
			}
		)
);
