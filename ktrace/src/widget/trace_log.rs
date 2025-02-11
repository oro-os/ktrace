use std::sync::Arc;

use ratatui::{
	buffer::Buffer,
	layout::{Constraint, Direction, Layout, Rect},
	widgets::{List, ListItem, Widget},
};

pub struct TraceLog<S: TraceLogState>(pub Arc<S>);

pub trait TraceLogState {
	fn get_last_addresses(&self) -> Vec<u64>;
	fn get_last_lower_addresses(&self) -> Vec<u64>;
}

impl<S: TraceLogState> Widget for TraceLog<S> {
	fn render(self, area: Rect, buf: &mut Buffer)
	where
		Self: Sized,
	{
		let layout = Layout::default()
			.direction(Direction::Horizontal)
			.constraints([
				Constraint::Percentage(50),
				Constraint::Length(1),
				Constraint::Percentage(50),
			])
			.split(area);

		{
			let num_rows = usize::from(layout[2].height);
			let last_addresses = self.0.get_last_lower_addresses();
			let Some(addr_slice) = last_addresses.get(last_addresses.len().saturating_sub(num_rows)..) else {
				return;
			};

			List::new(
				addr_slice
					.iter()
					.map(|addr| ListItem::new(format!("{addr:#016X}"))),
			)
			.render(layout[2], buf);
		}

		{
			let num_rows = usize::from(layout[0].height);
			let last_addresses = self.0.get_last_addresses();
			let Some(addr_slice) = last_addresses.get(last_addresses.len().saturating_sub(num_rows)..) else {
				return;
			};

			List::new(
				addr_slice
					.iter()
					.map(|addr| ListItem::new(format!("{addr:#016X}"))),
			)
			.render(layout[0], buf);
		}
	}
}
