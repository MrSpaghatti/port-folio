use std::{io, time::Duration};

use ratatui::{
    prelude::*,
    widgets::*,
};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use netstat2::{get_sockets_info, AddressFamilyFlags, ProtocolFlags, ProtocolSocketInfo, SocketInfo, error::Error as NetstatError};

mod ui;
use ui::stateful_list::StatefulList;

struct App {
    processes: Result<StatefulList<SocketInfo>, NetstatError>,
}

impl App {
    fn new() -> App {
        let af_flags = AddressFamilyFlags::all();
        let proto_flags = ProtocolFlags::all();
        let sockets_info = get_sockets_info(af_flags, proto_flags);

        App {
            processes: sockets_info.map(|s| StatefulList::with_items(s)),
        }
    }
}

fn main() -> io::Result<()> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let app = App::new();
    let res = run_app(&mut terminal, app);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Down => {
                        if let Ok(processes) = &mut app.processes {
                            processes.next();
                        }
                    }
                    KeyCode::Up => {
                        if let Ok(processes) = &mut app.processes {
                            processes.previous();
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

fn ui(frame: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(80),
            Constraint::Percentage(20),
        ])
        .split(frame.size());

    let top_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(70),
            Constraint::Percentage(30),
        ])
        .split(chunks[0]);

    match &mut app.processes {
        Ok(processes) => {
            let processes_list_items: Vec<ListItem> = processes
                .items
                .iter()
                .map(|c| {
                    let s = match &c.protocol_socket_info {
                        ProtocolSocketInfo::Tcp(tcp) => {
                            format!(
                                "TCP {}:{} -> {}:{} {:?} - {}",
                                tcp.local_addr,
                                tcp.local_port,
                                tcp.remote_addr,
                                tcp.remote_port,
                                c.associated_pids,
                                tcp.state
                            )
                        }
                        ProtocolSocketInfo::Udp(udp) => {
                            format!(
                                "UDP {}:{} -> *:* {:?}",
                                udp.local_addr, udp.local_port, c.associated_pids
                            )
                        }
                    };
                    ListItem::new(s)
                })
                .collect();

            let processes_list = List::new(processes_list_items)
                .block(Block::default().borders(Borders::ALL).title("Processes"))
                .highlight_style(
                    Style::default()
                        .bg(Color::Blue)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol(">> ");
            frame.render_stateful_widget(processes_list, top_chunks[0], &mut processes.state);
        }
        Err(e) => {
            let error_message = format!("Error fetching socket information: {}", e);
            let block = Block::default().title("Error").borders(Borders::ALL);
            let paragraph = Paragraph::new(error_message).block(block);
            frame.render_widget(paragraph, top_chunks[0]);
        }
    }

    frame.render_widget(
        Block::new().borders(Borders::ALL).title("Details"),
        top_chunks[1],
    );
    frame.render_widget(
        Block::new().borders(Borders::ALL).title("Logs"),
        chunks[1],
    );
}
