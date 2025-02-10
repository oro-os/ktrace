use std::sync::Arc;

use ktrace_protocol::ThreadStatus;
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
	fn thread_status(&self) -> ThreadStatus;
	fn instruction_count(&self) -> usize;
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
			Span::from(" | thread status: "),
			match self.0.thread_status() {
				ThreadStatus::Idle => Span::styled("idle", Style::default().fg(Color::Yellow)),
				ThreadStatus::Running => Span::styled("running", Style::default().fg(Color::Green)),
				ThreadStatus::Dead => Span::styled("dead", Style::default().fg(Color::Red)),
			},
			Span::from(" | icount: "),
			Span::styled(
				format!("{}", self.0.instruction_count()),
				Style::default().fg(Color::Cyan),
			),
		])
		.render(area, buf);
	}
}
