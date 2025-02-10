use std::sync::Arc;

use ratatui::{
	buffer::Buffer,
	layout::Rect,
	widgets::{List, ListItem, Widget},
};

pub struct TraceLog<S: TraceLogState>(pub Arc<S>);

pub trait TraceLogState {
	fn get_last_addresses(&self) -> Vec<u64>;
}

impl<S: TraceLogState> Widget for TraceLog<S> {
	fn render(self, area: Rect, buf: &mut Buffer)
	where
		Self: Sized,
	{
		let num_rows = usize::from(area.height);
		let last_addresses = self.0.get_last_addresses();
		let Some(addr_slice) = last_addresses.get(last_addresses.len().saturating_sub(num_rows)..)
		else {
			return;
		};

		List::new(
			addr_slice
				.iter()
				.map(|addr| ListItem::new(format!("{:#016X}", addr))),
		)
		.render(area, buf);
	}
}
