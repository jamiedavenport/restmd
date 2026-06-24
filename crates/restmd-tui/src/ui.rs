//! Rendering the three-pane layout. Pure: reads `&App`, draws to the `Frame`.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, List, ListItem, ListState, Paragraph, Wrap};

use restmd_core::RequestOutcome;

use crate::app::{App, Pane, RunState};

pub fn draw(frame: &mut Frame, app: &App) {
    let [main, status] =
        Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(frame.area());
    let [files, middle, response] = Layout::horizontal([
        Constraint::Percentage(22),
        Constraint::Percentage(38),
        Constraint::Percentage(40),
    ])
    .areas(main);
    let [requests, source] =
        Layout::vertical([Constraint::Percentage(55), Constraint::Percentage(45)]).areas(middle);

    draw_files(frame, app, files);
    draw_requests(frame, app, requests);
    draw_source(frame, app, source);
    draw_response(frame, app, response);

    frame.render_widget(
        Paragraph::new(app.status_line.as_str())
            .style(Style::new().fg(Color::Black).bg(Color::Gray)),
        status,
    );
}

fn pane_block(title: &str, focused: bool) -> Block<'_> {
    let border = if focused {
        Style::new().fg(Color::Yellow)
    } else {
        Style::new().fg(Color::DarkGray)
    };
    Block::bordered()
        .title(title)
        .border_style(border)
        .title_style(Style::new().fg(Color::White).add_modifier(Modifier::BOLD))
}

fn highlight(focused: bool) -> Style {
    if focused {
        Style::new().bg(Color::Blue).fg(Color::White)
    } else {
        Style::new().add_modifier(Modifier::REVERSED)
    }
}

fn draw_files(frame: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .files
        .iter()
        .map(|f| ListItem::new(f.name.clone()))
        .collect();
    let focused = app.focus == Pane::Files;
    let list = List::new(items)
        .block(pane_block("Files", focused))
        .highlight_style(highlight(focused));
    let mut state = ListState::default().with_selected(Some(app.selected_file));
    frame.render_stateful_widget(list, area, &mut state);
}

fn draw_requests(frame: &mut Frame, app: &App, area: Rect) {
    let focused = app.focus == Pane::Requests;
    let Some(file) = app.current_file() else {
        frame.render_widget(pane_block("Requests", focused), area);
        return;
    };

    let outcomes = match app.results.get(&file.path) {
        Some(RunState::Done(report)) => Some(&report.outcomes),
        _ => None,
    };
    let running = matches!(app.results.get(&file.path), Some(RunState::Running));

    let items: Vec<ListItem> = file
        .document
        .requests
        .iter()
        .enumerate()
        .map(|(i, req)| {
            let heading = req
                .heading_span
                .slice(&file.source)
                .trim_start_matches('#')
                .trim();
            let (glyph, color) = match outcomes.and_then(|o| o.get(i)) {
                Some(o) if o.passed() => ("✓", Color::Green),
                Some(_) => ("✗", Color::Red),
                None if running => ("…", Color::Yellow),
                None => (" ", Color::DarkGray),
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!("{glyph} "), Style::new().fg(color)),
                Span::raw(heading.to_string()),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(pane_block("Requests", focused))
        .highlight_style(highlight(focused));
    let mut state = ListState::default().with_selected(Some(app.selected_request));
    frame.render_stateful_widget(list, area, &mut state);
}

fn draw_source(frame: &mut Frame, app: &App, area: Rect) {
    let block = pane_block("Source", false);
    let text = match app.current_file() {
        Some(file) if !file.parse_errors.is_empty() => file
            .parse_errors
            .iter()
            .map(|e| format!("parse error: {}", e.kind))
            .collect::<Vec<_>>()
            .join("\n"),
        Some(file) => file
            .document
            .requests
            .get(app.selected_request)
            .map(|req| req.span.slice(&file.source).to_string())
            .unwrap_or_default(),
        None => String::new(),
    };
    frame.render_widget(
        Paragraph::new(text).block(block).wrap(Wrap { trim: false }),
        area,
    );
}

fn draw_response(frame: &mut Frame, app: &App, area: Rect) {
    let focused = app.focus == Pane::Response;
    let lines = response_lines(app);
    frame.render_widget(
        Paragraph::new(lines)
            .block(pane_block("Response", focused))
            .wrap(Wrap { trim: false })
            .scroll((app.response_scroll, 0)),
        area,
    );
}

fn response_lines(app: &App) -> Vec<Line<'static>> {
    let Some(state) = app.current_run_state() else {
        return vec![Line::from("Not run. Press Enter to run through this request.").dim()];
    };
    let report = match state {
        RunState::Running => return vec![Line::from("Running…").yellow()],
        RunState::Done(report) => report,
    };
    if let Some(err) = &report.error {
        return vec![Line::from(format!("Error: {err}")).red()];
    }
    match report.outcomes.get(app.selected_request) {
        Some(outcome) => outcome_lines(outcome),
        None => vec![Line::from("Request was not reached (run stopped earlier).").dim()],
    }
}

fn outcome_lines(outcome: &RequestOutcome) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    lines.push(Line::from(format!("{} {}", outcome.method.as_str(), outcome.url)).bold());

    if let Some(err) = &outcome.error {
        lines.push(Line::from(format!("Error: {err}")).red());
        return lines;
    }

    if let Some(resp) = &outcome.response {
        let ok = (200..400).contains(&resp.status);
        let color = if ok { Color::Green } else { Color::Yellow };
        lines.push(Line::from(vec![
            Span::raw("Status: "),
            Span::styled(resp.status.to_string(), Style::new().fg(color)),
            Span::raw(format!("   {} ms", resp.elapsed.as_millis())),
        ]));
    }

    if !outcome.assertions.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from("Assertions:").bold());
        for a in &outcome.assertions {
            let (glyph, color) = if a.passed {
                ("✓", Color::Green)
            } else {
                ("✗", Color::Red)
            };
            let mut text = format!("{glyph} {}", a.description);
            if let Some(detail) = &a.detail {
                text.push_str(&format!("  ({detail})"));
            }
            lines.push(Line::from(text).style(Style::new().fg(color)));
        }
    }

    if !outcome.captures.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from("Captures:").bold());
        for c in &outcome.captures {
            let text = match (&c.value, &c.error) {
                (Some(v), _) => format!("{} = {v}", c.name),
                (None, Some(e)) => format!("{} ✗ {e}", c.name),
                (None, None) => c.name.clone(),
            };
            lines.push(Line::from(text));
        }
    }

    if let Some(resp) = &outcome.response {
        lines.push(Line::from(""));
        lines.push(Line::from("Body:").bold());
        for line in pretty_body(resp).lines() {
            lines.push(Line::from(line.to_string()));
        }
    }

    lines
}

/// Pretty-print a JSON body; otherwise return it lossily as text.
fn pretty_body(resp: &restmd_core::ResponseView) -> String {
    let is_json = resp
        .headers
        .iter()
        .any(|(k, v)| k.eq_ignore_ascii_case("content-type") && v.contains("json"));
    if is_json
        && let Ok(value) = serde_json::from_slice::<serde_json::Value>(&resp.body)
        && let Ok(pretty) = serde_json::to_string_pretty(&value)
    {
        return pretty;
    }
    resp.body_text().into_owned()
}
