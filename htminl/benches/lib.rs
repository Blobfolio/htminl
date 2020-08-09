/*!
# Benchmark: `htminl`
*/

use criterion::{
	Criterion,
	criterion_group,
	criterion_main,
};
use htminl::strtendril;
use std::path::PathBuf;
use tendril::StrTendril;



fn collapse_whitespace(c: &mut Criterion) {
	let mut group = c.benchmark_group("htminl::collapse_whitespace");

	group.bench_function(format!("{:?}", "My name is 	Jeffrey.\n\nI like flowers!     "), move |b| {
		b.iter_with_setup(||
			StrTendril::from("My name is 	Jeffrey.\n\nI like flowers!     "),
			|mut st| strtendril::collapse_whitespace(&mut st)
		)
	});

	group.finish();
}

fn minify_html(c: &mut Criterion) {
	let mut group = c.benchmark_group("htminl::minify_html");

	let path = PathBuf::from("../test-assets/blobfolio.com.html");
	assert!(path.is_file());

	group.bench_function("blobfolio.com.html", move |b| {
		b.iter_with_setup(||
			std::fs::read(&path).unwrap(),
			|mut t| htminl::minify_html(&mut t)
		)
	});

	group.finish();
}



criterion_group!(
	benches,
	minify_html,
	collapse_whitespace,
);
criterion_main!(benches);
