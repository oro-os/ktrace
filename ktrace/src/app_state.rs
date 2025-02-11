use std::sync::{
	Mutex,
	atomic::{AtomicBool, AtomicUsize, Ordering::Relaxed},
};

use ktrace_protocol::ThreadStatus;

#[derive(Default, Debug)]
pub struct AppState {
	pub daemon_connected:     Flag,
	pub last_addresses:       Mutex<Vec<u64>>,
	pub last_lower_addresses: Mutex<Vec<u64>>,
	pub thread_status:        AtomicUsize,
	pub instruction_count:    AtomicUsize,
}

impl crate::widget::status_bar::StatusBarState for AppState {
	#[inline]
	fn is_connected(&self) -> bool {
		self.daemon_connected.get()
	}

	#[inline]
	fn instruction_count(&self) -> usize {
		self.instruction_count.load(Relaxed)
	}

	#[inline]
	fn thread_status(&self) -> ThreadStatus {
		if !self.is_connected() {
			return ThreadStatus::Dead;
		}

		match self.thread_status.load(Relaxed) {
			0 => ThreadStatus::Idle,
			1 => ThreadStatus::Running,
			_ => ThreadStatus::Dead,
		}
	}
}

impl crate::widget::trace_log::TraceLogState for AppState {
	#[inline]
	fn get_last_addresses(&self) -> Vec<u64> {
		self.last_addresses.lock().unwrap().clone()
	}

	#[inline]
	fn get_last_lower_addresses(&self) -> Vec<u64> {
		self.last_lower_addresses.lock().unwrap().clone()
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
