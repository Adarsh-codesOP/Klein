use crate::app::App;
use ratatui::{
    layout::Rect,
    widgets::{Paragraph, Wrap},
    Frame,
};

pub fn render(f: &mut Frame, app: &App) {
    let hover = match &app.lsp_state.hover {
        Some(h) => h,
        None => return,
    };

    if hover.contents.is_empty() {
        return;
    }

    let editor_area = app.editor_area.get();
    let editor = app.editor();

    // Calculate cursor screen position
    let cursor_screen_y = editor.cursor_y.saturating_sub(editor.scroll_y);
    let cursor_screen_x = editor.get_cursor_screen_column();

    // The popup should appear above the cursor if possible, else below
    let width = 60.min(f.size().width.saturating_sub(editor_area.x + 4));

    // Estimate height based on wrap
    let text = hover.contents.clone();
    let line_count = text.lines().count();
    let height = (line_count + 2).min(15) as u16;

    let x = editor_area.x + cursor_screen_x as u16;
    let mut y = (editor_area.y + cursor_screen_y as u16).saturating_sub(height);

    // If too close to top, show below cursor
    if y < editor_area.y {
        y = editor_area.y + cursor_screen_y as u16 + 1;
    }

    // Safety: check if coordinates are within total frame bounds
    if x >= f.size().width {
        return; // Don't even try to render if x is off-screen
    }
    if y >= f.size().height {
        y = f.size().height.saturating_sub(height + 1);
    }

    // Shift left if too close to right edge
    let mut final_x = x;
    if final_x + width > f.size().width {
        final_x = f.size().width.saturating_sub(width);
    }

    // Final safety check: ensuring Rect dimensions are valid (width/height >= 1)
    if width < 2 || height < 2 {
        return;
    }

    let area = Rect::new(final_x, y, width, height);
    f.render_widget(ratatui::widgets::Clear, area);

    let para = Paragraph::new(text).wrap(Wrap { trim: true });

    f.render_widget(para, area);
}
