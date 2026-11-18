#![cfg(feature = "async")]

use std::cell::Cell;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

use bip39::{Language, Mnemonic};

fn noop_waker() -> Waker {
	fn raw_waker() -> RawWaker {
		static VTABLE: RawWakerVTable =
			RawWakerVTable::new(|_| raw_waker(), |_| {}, |_| {}, |_| {});
		RawWaker::new(std::ptr::null(), &VTABLE)
	}
	// SAFETY: All clones use the same static vtable. Its functions never access the data pointer.
	unsafe { Waker::from_raw(raw_waker()) }
}

fn yield_once() -> impl Future<Output = ()> {
	struct YieldOnce {
		yielded: bool,
	}

	impl Future for YieldOnce {
		type Output = ();

		fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
			if self.yielded {
				Poll::Ready(())
			} else {
				self.yielded = true;
				cx.waker().wake_by_ref();
				Poll::Pending
			}
		}
	}

	YieldOnce {
		yielded: false,
	}
}

fn finish(future: impl Future<Output = [u8; 64]>, expected_yields: usize) -> [u8; 64] {
	let mut future = Box::pin(future);
	let waker = noop_waker();
	let mut cx = Context::from_waker(&waker);
	for _ in 0..expected_yields {
		assert!(future.as_mut().poll(&mut cx).is_pending());
	}
	match future.as_mut().poll(&mut cx) {
		Poll::Ready(seed) => seed,
		Poll::Pending => panic!("derivation did not finish"),
	}
}

#[test]
fn test_to_seed_normalized_async() {
	let passphrases = [
		String::new(),
		"TREZOR".to_owned(),
		"pa\u{308}ssphrase".to_owned(),
		"\0x".to_owned(),
		"p".repeat(512),
	];
	for &language in Language::ALL {
		for (i, &len) in [16, 20, 24, 28, 32].iter().enumerate() {
			let entropy: Vec<u8> = (0..len).map(|n| (n * 37 + i) as u8).collect();
			let mnemonic = Mnemonic::from_entropy_in(language, &entropy).unwrap();
			let passphrase = &passphrases[i];
			let expected = mnemonic.to_seed_normalized(passphrase);
			assert_eq!(
				&finish(mnemonic.to_seed_normalized_async(passphrase, yield_once), 2048)[..],
				&expected[..]
			);
			let mut rounds = 0;
			let future = mnemonic.to_seed_normalized_async(passphrase, || {
				rounds += 1;
				async {}
			});
			assert_eq!(&finish(future, 0)[..], &expected[..]);
			assert_eq!(rounds, 2048);
		}
	}
}

#[test]
fn test_yield_future_borrows_its_own_state() {
	let mnemonic = Mnemonic::from_entropy(&[0; 16]).unwrap();
	let completed = Cell::new(0);
	let mut future = Box::pin(mnemonic.to_seed_normalized_async("", || async {
		let mut value = 0;
		let borrowed = &mut value;
		yield_once().await;
		*borrowed += 1;
		completed.set(completed.get() + value);
	}));
	let waker = noop_waker();
	let mut cx = Context::from_waker(&waker);
	assert!(future.as_mut().poll(&mut cx).is_pending());
	assert!(future.as_mut().poll(&mut cx).is_pending());
	assert_eq!(completed.get(), 1);
}

#[test]
fn test_to_seed_normalized_async_cancellation() {
	struct OnDrop<'a>(&'a Cell<usize>);
	impl Drop for OnDrop<'_> {
		fn drop(&mut self) {
			self.0.set(self.0.get() + 1);
		}
	}

	let mnemonic = Mnemonic::from_entropy(&[0; 16]).unwrap();
	for polls in [0, 1, 2048].iter().copied() {
		let drops = Cell::new(0);
		let started = Cell::new(0);
		let finished = Cell::new(0);
		let mut future = Box::pin(mnemonic.to_seed_normalized_async("", || async {
			let _guard = OnDrop(&drops);
			started.set(started.get() + 1);
			yield_once().await;
			finished.set(finished.get() + 1);
		}));
		let waker = noop_waker();
		let mut cx = Context::from_waker(&waker);
		for _ in 0..polls {
			assert!(future.as_mut().poll(&mut cx).is_pending());
		}
		drop(future);
		assert_eq!(started.get(), polls);
		assert_eq!(drops.get(), polls);
		assert_eq!(finished.get(), polls.saturating_sub(1));
	}
}
