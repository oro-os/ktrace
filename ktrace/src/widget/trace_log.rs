use ratatui::{
	buffer::Buffer,
	layout::Rect,
	style::{Color, Style},
	text::{Span, Text},
	widgets::{List, ListItem, Widget},
};

use crate::symbol_resolver::Symbol;

pub struct TraceLog<'a>(pub &'a Vec<Symbol>);

const ADDR_STYLE: Style = Style::new().fg(Color::Yellow);
const SYM_ADDR_STYLE: Style = Style::new().fg(Color::Yellow);
const NAME_STYLE: Style = Style::new().fg(Color::White);
const FILE_STYLE: Style = Style::new().fg(Color::DarkGray);
const LINE_STYLE: Style = Style::new().fg(Color::Cyan);

impl<'a> Widget for TraceLog<'a> {
	fn render(self, area: Rect, buf: &mut Buffer)
	where
		Self: Sized,
	{
		let num_rows = usize::from(area.height);
		let Some(sym_slice) = self.0.get(self.0.len().saturating_sub(num_rows)..) else {
			return;
		};

		List::new(sym_slice.iter().map(|sym| {
			let mut text = Text::default();
			text.push_span(Span::styled(format!("{:016X}", sym.addr), ADDR_STYLE));

			if let Some(name) = &sym.name {
				text.push_span(Span::raw(" "));
				text.push_span(Span::styled(name.clone(), NAME_STYLE));
			}

			if let Some(sym_addr) = sym.sym_addr {
				text.push_span(Span::raw("<"));
				text.push_span(Span::styled(format!("{:016X}", sym_addr), SYM_ADDR_STYLE));
				text.push_span(Span::raw(">"));
			}

			if let Some(file) = &sym.file {
				text.push_span(Span::raw(" "));
				text.push_span(Span::styled(file.clone(), FILE_STYLE));
			}

			if let Some(line) = sym.line {
				text.push_span(Span::raw(":"));
				text.push_span(Span::styled(format!("{}", line), LINE_STYLE));
			}

			ListItem::new(text)
		}))
		.render(area, buf);
	}
}
