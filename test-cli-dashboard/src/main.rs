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
/// 
/// # 引数
/// * `count` - 表示するデータ項目数
/// * `data_list` - 表示するデータのリスト
pub fn cli_dashboard(count: usize, data_list: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, crossterm::terminal::EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let start_time = Instant::now();
    let duration = Duration::from_secs(5);

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

            let title = Paragraph::new("Dashboard")
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
    let data = vec![
        format!("Number: {}", rand::random::<u32>() % 1000),
        format!("Float: {:.2}", rand::random::<f64>() * 100.0),
        format!("Percent: {:.1}%", rand::random::<f64>() * 100.0),
    ];
    cli_dashboard(3, data)?;
    
    // 別の例
    // let custom_data = vec![
    //     "Custom Data 1".to_string(),
    //     "Custom Data 2".to_string(),
    //     "Custom Data 3".to_string(),
    //     "Custom Data 4".to_string(),
    // ];
    // cli_dashboard(4, custom_data)?;

    Ok(())
}