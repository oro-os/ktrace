use std::sync::{
	Arc, Mutex,
	atomic::{AtomicBool, AtomicUsize, Ordering::Relaxed},
};

use circular_buffer::CircularBuffer;
use ktrace_protocol::ThreadStatus;
use tinylfu_cached::cache::{cached::CacheD, config::ConfigBuilder};
use tokio::sync::Mutex as AsyncMutex;

use crate::symbol_resolver::{ResolverClient, Symbol};

pub struct AppState {
	pub daemon_connected:     Flag,
	pub last_addresses:       Mutex<CircularBuffer<256, u64>>,
	pub last_lower_addresses: Mutex<CircularBuffer<256, u64>>,
	pub thread_status:        AtomicUsize,
	pub instruction_count:    AtomicUsize,
	pub resolver_client:      ResolverClient,
	pub resolution_cache:     CacheD<u64, Arc<AsyncMutex<Symbol>>>,
}

impl AppState {
	pub fn new(resolver_client: ResolverClient) -> Self {
		Self {
			daemon_connected: Flag::new(false),
			last_addresses: Mutex::new(CircularBuffer::new()),
			last_lower_addresses: Mutex::new(CircularBuffer::new()),
			thread_status: AtomicUsize::new(ThreadStatus::Dead as usize),
			instruction_count: AtomicUsize::new(0),
			resolver_client,
			resolution_cache: CacheD::new(ConfigBuilder::new(10000, 1000, 1024 * 1024 * 10).build()),
		}
	}

	fn resolve_all_for_list<const SZ: usize>(&self, list: &CircularBuffer<SZ, u64>) -> Vec<Symbol> {
		// TODO(qix-): [internal screaming]
		list.iter()
			.map(|addr| {
				let entry = self.resolution_cache.get(addr);
				if let Some(entry) = entry {
					entry
				} else {
					let new_entry = Arc::new(AsyncMutex::new(Symbol::new_unresolved(*addr)));
					self.resolver_client
						.resolve_and_invalidate(*addr, new_entry.clone());
					// TODO(qix-): move this into the service since it's async.
					let _ = self.resolution_cache.put(*addr, new_entry.clone());
					new_entry
				}
				.blocking_lock()
				.clone()
			})
			.collect()
	}
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

impl crate::view::trace_log::TraceLogState for AppState {
	#[inline]
	fn get_last_addresses(&self) -> Vec<Symbol> {
		self.resolve_all_for_list(&self.last_addresses.lock().unwrap())
	}

	#[inline]
	fn get_last_lower_addresses(&self) -> Vec<Symbol> {
		self.resolve_all_for_list(&self.last_lower_addresses.lock().unwrap())
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
