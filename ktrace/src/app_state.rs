use std::sync::{
	Mutex,
	atomic::{AtomicBool, Ordering::Relaxed},
};

#[derive(Default, Debug)]
pub struct AppState {
	pub daemon_connected: Flag,
	pub last_addresses:   Mutex<Vec<u64>>,
}

impl crate::widget::status_bar::StatusBarState for AppState {
	#[inline]
	fn is_connected(&self) -> bool {
		self.daemon_connected.get()
	}
}

impl crate::widget::trace_log::TraceLogState for AppState {
	#[inline]
	fn get_last_addresses(&self) -> Vec<u64> {
		self.last_addresses.lock().unwrap().clone()
	}
}

#[derive(Default, Debug)]
pub struct Flag(AtomicBool);

impl Flag {
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
