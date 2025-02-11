use std::{path::Path, sync::Arc};

use tokio::sync::{
	Mutex,
	mpsc::{UnboundedReceiver, UnboundedSender},
};
use wholesym::{LookupAddress, SymbolManager, SymbolManagerConfig};

async fn run_async(mut receiver: UnboundedReceiver<ResolverRequest>) {
	let symbol_library = SymbolManager::with_config(SymbolManagerConfig::default());

	let mut symbol_maps = vec![];

	loop {
		match receiver.recv().await.expect("failed to receive request") {
			ResolverRequest::AddBinary { path } => {
				symbol_maps.push(
					symbol_library
						.load_symbol_map_for_binary_at_path(Path::new(&path), None)
						.await
						.expect("failed to load symbol map"),
				);
			}
			ResolverRequest::ResolveAddress { address, entry } => {
				for map in &symbol_maps {
					let sym = map.lookup(LookupAddress::Svma(address)).await;
					if let Some(sym) = sym {
						let mut entry = entry.lock().await;
						entry.name = Some(sym.symbol.name);
						entry.sym_addr = Some(u64::from(sym.symbol.address));
						if let Some((f, l)) = sym
							.frames
							.and_then(|f| f.get(0).cloned())
							.map(|f| (f.file_path, f.line_number))
						{
							entry.file = f.map(|p| p.display_path());
							entry.line = l.map(u64::from);
						}
						crate::invalidate();
						break;
					}
				}
			}
		}
	}
}

pub fn run() -> ResolverClient {
	let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();

	// Run in Tokio.
	std::thread::spawn(|| {
		tokio::runtime::Runtime::new()
			.unwrap()
			.block_on(run_async(receiver));
	});

	ResolverClient { sender }
}

#[derive(Debug)]
enum ResolverRequest {
	ResolveAddress {
		address: u64,
		entry:   Arc<Mutex<Symbol>>,
	},
	AddBinary {
		path: String,
	},
}

pub struct ResolverClient {
	sender: UnboundedSender<ResolverRequest>,
}

impl ResolverClient {
	pub fn add_binary(&self, path: String) {
		self.sender
			.send(ResolverRequest::AddBinary { path })
			.unwrap();
	}

	pub fn resolve_and_invalidate(&self, address: u64, entry: Arc<Mutex<Symbol>>) {
		self.sender
			.send(ResolverRequest::ResolveAddress { address, entry })
			.unwrap();
	}
}

#[derive(Debug, Clone)]
pub struct Symbol {
	pub addr:     u64,
	pub name:     Option<String>,
	pub file:     Option<String>,
	pub line:     Option<u64>,
	pub sym_addr: Option<u64>,
}

impl Symbol {
	pub const fn new_unresolved(address: u64) -> Self {
		Symbol {
			addr:     address,
			name:     None,
			file:     None,
			line:     None,
			sym_addr: None,
		}
	}
}
