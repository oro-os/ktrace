use std::sync::Arc;

use ratatui::{
	buffer::Buffer,
	layout::Rect,
	style::{Color, Style},
	text::{Line, Span},
	widgets::Widget,
};

pub struct StatusBar<S>(pub Arc<S>);

pub trait StatusBarState {
	fn is_connected(&self) -> bool;
}

impl<S: StatusBarState> Widget for StatusBar<S> {
	fn render(self, area: Rect, buf: &mut Buffer) {
		let connected = self.0.is_connected();

		Line::from_iter([
			Span::from("daemon: "),
			Span::styled(
				if connected { "online" } else { "offline" },
				Style::default().fg(if connected { Color::Green } else { Color::Red }),
			),
		])
		.render(area, buf);
	}
}
