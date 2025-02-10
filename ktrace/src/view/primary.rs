use std::sync::Arc;

use ratatui::{
	Frame,
	layout::{Constraint, Direction::Vertical, Layout},
	widgets::{Block, Borders},
};

use crate::{app_state::AppState, widget};

pub fn draw(frame: &mut Frame, state: &Arc<AppState>) {
	let layout = Layout::default()
		.direction(Vertical)
		.constraints(&[Constraint::Fill(1), Constraint::Length(2)])
		.split(frame.area());

	let status_block = Block::default().borders(Borders::TOP);

	frame.render_widget(widget::trace_log::TraceLog(state.clone()), layout[0]);

	frame.render_widget(&status_block, layout[1]);
	frame.render_widget(
		widget::status_bar::StatusBar(state.clone()),
		status_block.inner(layout[1]),
	);
}
