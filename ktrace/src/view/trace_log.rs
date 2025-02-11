use std::sync::Arc;

use ratatui::{
	Frame,
	layout::{
		Constraint,
		Direction::{Horizontal, Vertical},
		Layout,
	},
	widgets::{Block, Borders},
};

use crate::{app_state::AppState, symbol_resolver::Symbol, widget};

pub trait TraceLogState {
	fn get_last_addresses(&self) -> Vec<Symbol>;
	fn get_last_lower_addresses(&self) -> Vec<Symbol>;
}

pub fn draw(frame: &mut Frame, state: &Arc<AppState>) {
	let layout = Layout::default()
		.direction(Vertical)
		.constraints(&[Constraint::Fill(1), Constraint::Length(2)])
		.split(frame.area());

	let trace_layout = Layout::default()
		.direction(Horizontal)
		.constraints(&[
			Constraint::Percentage(50),
			Constraint::Length(1),
			Constraint::Percentage(50),
		])
		.split(layout[0]);

	frame.render_widget(
		widget::trace_log::TraceLog(&state.get_last_addresses()),
		trace_layout[0],
	);
	frame.render_widget(
		widget::trace_log::TraceLog(&state.get_last_lower_addresses()),
		trace_layout[2],
	);

	let status_block = Block::default().borders(Borders::TOP);
	frame.render_widget(&status_block, layout[1]);
	frame.render_widget(
		widget::status_bar::StatusBar(state.clone()),
		status_block.inner(layout[1]),
	);
}
