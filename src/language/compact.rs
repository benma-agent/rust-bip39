use super::*;

// TEXT holds the word bytes without padding. Each group of sixteen variable-length words has
// one u64 index entry with its starting byte offset in the low 16 bits, then sixteen 3-bit
// lengths. A word's byte length is base + step * its encoded length.
#[derive(Clone, Copy)]
pub(super) struct WordList {
	text: &'static str,
	base: usize,
	step: usize,
	blocks: Option<&'static [u64; 128]>,
}

const fn lengths_fit(words: &[&str; 2048], base: usize, step: usize) -> bool {
	let mut i = 0;
	while i < words.len() {
		let len = words[i].len();
		if len < base || len > base + 7 * step || (step != 0 && (len - base) % step != 0) {
			return false;
		}
		i += 1;
	}
	true
}

const fn build_blocks(words: &[&str; 2048], base: usize, step: usize) -> [u64; 128] {
	let mut blocks = [0; 128];
	let mut position = 0;
	let mut i = 0;
	while i < words.len() {
		let len = words[i].len();
		if i % 16 == 0 {
			blocks[i / 16] = position as u64;
		}
		let encoded = (len - base) / step;
		blocks[i / 16] |= (encoded as u64) << (16 + 3 * (i % 16));
		position += len;
		i += 1;
	}
	blocks
}

// Build compact storage from the canonical list so there is only one word list to maintain.
// Unlike WORDS, this representation needs no pointer/length pair for each word.
macro_rules! compact_list {
	($language:ident, base = $base:literal, step = $step:literal) => {{
		const LIST: WordList = {
			// Reject offsets or lengths that cannot be encoded. This array-length assertion
			// works on Rust 1.46, before assertions in const evaluation were supported.
			let _: [(); 1] = [(); ($language::TEXT.len() <= 0xffff
				&& lengths_fit(&$language::WORDS, $base, $step)) as usize];
			WordList {
				text: $language::TEXT,
				base: $base,
				step: $step,
				blocks: if $step == 0 {
					None
				} else {
					Some(&build_blocks(&$language::WORDS, $base, $step))
				},
			}
		};
		LIST
	}};
}

impl WordList {
	pub(super) fn for_language(language: Language) -> Self {
		match language {
			// English words are 3 to 8 bytes long, encoded as length minus 3.
			Language::English => compact_list!(english, base = 3, step = 1),
			// Both Chinese lists have a fixed width. A zero step omits the index.
			#[cfg(feature = "chinese-simplified")]
			Language::SimplifiedChinese => compact_list!(chinese_simplified, base = 3, step = 0),
			#[cfg(feature = "chinese-traditional")]
			Language::TraditionalChinese => compact_list!(chinese_traditional, base = 3, step = 0),
			#[cfg(feature = "czech")]
			Language::Czech => compact_list!(czech, base = 4, step = 1),
			#[cfg(feature = "french")]
			Language::French => compact_list!(french, base = 5, step = 1),
			#[cfg(feature = "italian")]
			Language::Italian => compact_list!(italian, base = 4, step = 1),
			#[cfg(feature = "japanese")]
			Language::Japanese => compact_list!(japanese, base = 9, step = 3),
			#[cfg(feature = "korean")]
			Language::Korean => compact_list!(korean, base = 12, step = 3),
			#[cfg(feature = "portuguese")]
			Language::Portuguese => compact_list!(portuguese, base = 4, step = 1),
			#[cfg(feature = "spanish")]
			Language::Spanish => compact_list!(spanish, base = 3, step = 1),
		}
	}

	fn word_length(self, lengths: u64) -> usize {
		self.base + self.step * (lengths & 7) as usize
	}

	pub(super) fn word_at(self, index: usize) -> Option<&'static str> {
		if index >= 2048 {
			return None;
		}
		let (start, len) = match self.blocks {
			Some(blocks) => {
				let block = blocks[index / 16];
				let mut start = (block & 0xffff) as usize;
				let mut lengths = block >> 16;
				// Sum at most fifteen lengths to locate the word within its block.
				for _ in 0..index % 16 {
					start += self.word_length(lengths);
					lengths >>= 3;
				}
				(start, self.word_length(lengths))
			}
			None => (index * self.base, self.base),
		};
		self.text.get(start..start + len)
	}

	pub(super) fn iter(self) -> impl Iterator<Item = &'static str> + Clone {
		(0..2048).scan(self.text, move |remaining, index| {
			// The remaining text starts at the next word, so only its length needs decoding.
			let len = match self.blocks {
				Some(blocks) => self.word_length(blocks[index / 16] >> (16 + 3 * (index % 16))),
				None => self.base,
			};
			let (word, rest) = remaining.split_at(len);
			*remaining = rest;
			Some(word)
		})
	}

	pub(super) fn binary_search(self, word: &str) -> Option<usize> {
		// Search indices because the compact representation has no slice of whole words.
		let mut low = 0;
		let mut high = 2048;
		while low < high {
			let mid = low + (high - low) / 2;
			match self.word_at(mid).unwrap().cmp(word) {
				core::cmp::Ordering::Less => low = mid + 1,
				core::cmp::Ordering::Greater => high = mid,
				core::cmp::Ordering::Equal => return Some(mid),
			}
		}
		None
	}
}
