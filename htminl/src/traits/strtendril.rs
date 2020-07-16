/*!
# HTML Traits: `StrTendril`

This trait exposes a few string manipulation methods to the `StrTendril`
struct.
*/

use tendril::StrTendril;



/// Extra String Methods for Tendril.
pub trait MinifyStrTendril {
	/// Collapse Whitespace.
	fn collapse_whitespace(&mut self);

	/// Is Whitespace?
	fn is_whitespace(&self) -> bool;

	/// Trim.
	fn trim(&mut self);

	/// Trim Start.
	fn trim_start(&mut self);

	/// Trim End.
	fn trim_end(&mut self);
}

impl MinifyStrTendril for StrTendril {
	/// Collapse Whitespace.
	///
	/// HTML rendering largely ignores whitespace, and at any rate treats all
	/// types (other than the no-break space `\xA0`) the same.
	///
	/// There is some nuance, but for most elements, we can safely convert all
	/// contiguous sequences of (ASCII) whitespace to a single horizontal space
	/// character.
	///
	/// Complete trimming gets dangerous, particularly given that CSS can
	/// override the display state of any element arbitrarily, so we are *not*
	/// doing that here.
	fn collapse_whitespace(&mut self) {
		let alter = Self::from(unsafe {
			let mut in_ws: bool = false;
			std::str::from_utf8_unchecked(&self.as_bytes()
				.iter()
				.filter_map(|c| match *c {
					b'\t' | b'\n' | b'\x0C' | b'\r' | b' ' => if in_ws { None }
						else {
							in_ws = true;
							Some(b' ')
						},
					c => if in_ws {
							in_ws = false;
							Some(c)
						}
						else {
							Some(c)
						},
				})
				.collect::<Vec<u8>>())
		});

		if (*self).ne(&alter) {
			*self = alter;
		}
	}

	/// Is (Only) Whitespace?
	///
	/// Returns `true` if the node is empty or contains only whitespace.
	fn is_whitespace(&self) -> bool {
		! self.as_bytes()
			.iter()
			.any(|c| match *c {
				b'\t' | b'\n' | b'\x0C' | b'\r' | b' ' => false,
				_ => true,
			})
	}

	/// Trim.
	fn trim(&mut self) {
		self.trim_start();
		self.trim_end();
	}

	/// Trim Start.
	fn trim_start(&mut self) {
		let len: u32 = self.as_bytes()
			.iter()
			.take_while(|c| match *c {
				b'\t' | b'\n' | b'\x0C' | b'\r' | b' ' => true,
				_ => false,
			})
			.count() as u32;
		if 0 != len {
			self.pop_front(len);
		}
	}

	/// Trim End.
	fn trim_end(&mut self) {
		let len: u32 = self.as_bytes()
			.iter()
			.rev()
			.take_while(|c| match *c {
				b'\t' | b'\n' | b'\x0C' | b'\r' | b' ' => true,
				_ => false,
			})
			.count() as u32;
		if 0 != len {
			self.pop_back(len);
		}
	}
}
