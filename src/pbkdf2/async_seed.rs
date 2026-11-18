use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

use super::{block_round, create_hmac_engine, xor};
use crate::{PBKDF2_BYTES, PBKDF2_ROUNDS};

pub(crate) fn derive<'a, M, F, Fut>(
	mnemonic: M,
	unprefixed_salt: &'a [u8],
	yield_now: F,
) -> impl Future<Output = [u8; PBKDF2_BYTES]> + 'a
where
	M: Iterator<Item = &'static str> + Clone + 'a,
	F: FnMut() -> Fut + 'a,
	Fut: Future<Output = ()> + 'a,
{
	// Initialize the HMAC engine when the caller first polls the future.
	let mut mnemonic = Some(mnemonic);
	let mut prf = None;
	let mut salt = [0; PBKDF2_BYTES];
	let mut remaining = 0..PBKDF2_ROUNDS;
	let rounds = core::iter::from_fn(move || {
		let round = remaining.next()?;
		if let Some(words) = mnemonic.take() {
			prf = Some(create_hmac_engine(words));
		}
		block_round(prf.as_ref()?, unprefixed_salt, 0, round, &mut salt);
		Some(salt)
	});
	SeedFuture {
		pending: None,
		rounds,
		seed: [0; PBKDF2_BYTES],
		yield_now: Some(yield_now),
	}
}

// Poll explicitly to support no_std on Rust 1.41.1. Rust 1.44 added no_std async/await.
struct SeedFuture<R, F, Fut> {
	// Drop a suspended callback future before its factory on cancellation.
	pending: Option<Fut>,
	rounds: R,
	seed: [u8; PBKDF2_BYTES],
	yield_now: Option<F>,
}

impl<R, F, Fut> Future for SeedFuture<R, F, Fut>
where
	R: Iterator<Item = [u8; PBKDF2_BYTES]>,
	F: FnMut() -> Fut,
	Fut: Future<Output = ()>,
{
	type Output = [u8; PBKDF2_BYTES];

	fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		// SAFETY: Only `pending` is structurally pinned. We poll it in place and
		// use Pin::set to drop it in place before starting another callback.
		let (rounds, seed, callback, mut pending) = unsafe {
			let this = self.get_unchecked_mut();
			(
				&mut this.rounds,
				&mut this.seed,
				&mut this.yield_now,
				Pin::new_unchecked(&mut this.pending),
			)
		};
		let yield_now = callback.as_mut().expect("seed future polled after completion");
		loop {
			if let Some(future) = pending.as_mut().as_pin_mut() {
				if future.poll(cx).is_pending() {
					return Poll::Pending;
				}
				pending.set(None);
			}
			match rounds.next() {
				Some(round_hash) => xor(seed, &round_hash),
				None => {
					*callback = None;
					return Poll::Ready(*seed);
				}
			}
			pending.set(Some(yield_now()));
		}
	}
}
