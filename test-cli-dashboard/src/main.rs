use ratatui::{
    backend::CrosstermBackend,
    Terminal,
    widgets::Paragraph,
    layout::{Layout, Constraint, Direction},
    style::{Style, Color},
};
use crossterm::{execute, terminal::{enable_raw_mode, disable_raw_mode}};
use std::{io, time::{Duration, Instant}};

/// CLI ダッシュボードを表示する関数
pub fn cli_dashboard(title_text:&str, count: usize, data_list: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, crossterm::terminal::EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let start_time = Instant::now();
    let duration = Duration::from_millis(1);

    while start_time.elapsed() < duration {
        terminal.draw(|f| {
            let size = f.area();
            
            let mut constraints = vec![Constraint::Length(2)];
            for _ in 0..count.min(data_list.len()) {
                constraints.push(Constraint::Length(2));
            }
            constraints.push(Constraint::Min(0));
            
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints(constraints)
                .split(size);

            let title = Paragraph::new(title_text.to_string())
                .style(Style::default().fg(Color::Yellow));
            f.render_widget(title, chunks[0]);

            for i in 0..count.min(data_list.len()) {
                let widget = Paragraph::new(data_list[i].clone())
                    .style(Style::default().fg(Color::Green));
                f.render_widget(widget, chunks[i + 1]);
            }
        })?;

        std::thread::sleep(Duration::from_millis(500));
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), crossterm::terminal::LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 使用例：cli_dashboard(データ数, データリスト)
    for i in 0..5 {
        let value1 = [rand::random::<u32>() % 100, (i * 10) as u32];
        let value2 = [rand::random::<u32>() % 50, (i * 5) as u32];
        
        let data = vec![
            format!("Item{}:{} {}", i, value1[0], value1[1]),
            format!("Value{}:{} {}", i, value2[0], value2[1]),
        ];
        cli_dashboard("cli", 2, data)?;
    }
    
    // 別の例：より詳細なデータ形式
    // let custom_data = vec![
    //     "CPU:85 %".to_string(),
    //     "Memory:4096 MB".to_string(),
    //     "Disk:250 GB".to_string(),
    //     "Network:1024 KB/s".to_string(),
    // ];
    // cli_dashboard("System Monitor", 4, custom_data)?;

    Ok(())
}