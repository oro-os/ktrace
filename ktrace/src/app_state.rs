use std::sync::atomic::{AtomicBool, Ordering::Relaxed};

#[derive(Default, Debug)]
pub struct AppState {
	daemon_connected: Flag,
}

impl AppState {
	#[expect(dead_code)]
	#[inline]
	pub fn set_daemon_connected(&self, v: bool) {
		self.daemon_connected.set(v);
	}
}

impl crate::widget::status_bar::StatusBarState for AppState {
	#[inline]
	fn is_connected(&self) -> bool {
		self.daemon_connected.get()
	}
}

#[derive(Default, Debug)]
struct Flag(AtomicBool);

impl Flag {
	#[expect(dead_code)]
	pub const fn new(v: bool) -> Self {
		Self(AtomicBool::new(v))
	}

	#[inline]
	pub fn set(&self, v: bool) {
		self.0.store(v, Relaxed);
	}

	#[inline]
	pub fn get(&self) -> bool {
		self.0.load(Relaxed)
	}
}
