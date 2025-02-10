use std::sync::Arc;

use ratatui::{
	buffer::Buffer,
	layout::Rect,
	style::{Color, Style, Stylize},
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
				if connected {
					Style::default().fg(Color::Green)
				} else {
					Style::default().fg(Color::Red).slow_blink()
				},
			),
		])
		.render(area, buf);
	}
}
