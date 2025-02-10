use std::sync::Arc;

use app_state::AppState;
use crossterm::event::{self, Event};

pub(crate) mod app_state;
pub(crate) mod view;
pub(crate) mod widget;

fn main() {
	let app_state = Arc::new(AppState::default());

	let mut terminal = ratatui::init();
	loop {
		terminal
			.draw(|f| view::primary::draw(f, &app_state))
			.expect("failed to draw frame");

		if matches!(event::read().expect("failed to read event"), Event::Key(_)) {
			break;
		}
	}

	ratatui::restore();
}
