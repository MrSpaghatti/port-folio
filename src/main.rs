use std::{io, time::Duration};
use tokio::time::interval;

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
use sysinfo::System;

mod ui;
use ui::stateful_list::StatefulList;

struct App {
    processes: Result<StatefulList<SocketInfo>, NetstatError>,
    system: System,
}

impl App {
    fn new() -> App {
        let af_flags = AddressFamilyFlags::all();
        let proto_flags = ProtocolFlags::all();
        let sockets_info = get_sockets_info(af_flags, proto_flags);

        App {
            processes: sockets_info.map(|s| StatefulList::with_items(s)),
            system: System::new_all(),
        }
    }

    fn update(&mut self) {
        let af_flags = AddressFamilyFlags::all();
        let proto_flags = ProtocolFlags::all();
        let new_sockets_info_result = get_sockets_info(af_flags, proto_flags);

        match new_sockets_info_result {
            Ok(new_sockets_info) => {
                if let Ok(processes) = &mut self.processes {
                    let previously_selected = processes.state.selected();
                    processes.items = new_sockets_info;
                    if let Some(previously_selected) = previously_selected {
                        if previously_selected >= processes.items.len() {
                            if processes.items.is_empty() {
                                processes.state.select(None);
                            } else {
                                processes.state.select(Some(processes.items.len() - 1));
                            }
                        }
                    }
                } else {
                    self.processes = Ok(StatefulList::with_items(new_sockets_info));
                }
            }
            Err(e) => {
                self.processes = Err(e);
            }
        }
        self.system.refresh_all();
    }
}

#[tokio::main]
async fn main() -> io::Result<()> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let app = App::new();
    let res = run_app(&mut terminal, app).await;

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

async fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
    let mut ticker = interval(Duration::from_secs(2));

    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        let event = tokio::task::spawn_blocking(move || -> io::Result<Option<Event>> {
            if event::poll(Duration::from_millis(250))? {
                Ok(Some(event::read()?))
            } else {
                Ok(None)
            }
        });

        tokio::select! {
            _ = ticker.tick() => {
                app.update();
            },
            res = event => {
                if let Ok(Ok(Some(Event::Key(key)))) = res {
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

    let details_block = Block::default().borders(Borders::ALL).title("Details");
    let details_text = if let Ok(processes) = &app.processes {
        if let Some(selected) = processes.state.selected() {
            let socket_info = &processes.items[selected];
            let pids = &socket_info.associated_pids;
            if let Some(pid) = pids.first() {
                if let Some(process) = app.system.process(sysinfo::Pid::from_u32(*pid)) {
                    let mut text = Vec::new();
                    text.push(Line::from(format!("PID: {}", process.pid())));
                    text.push(Line::from(format!("Name: {}", process.name())));
                    text.push(Line::from(format!("Status: {:?}", process.status())));
                    text.push(Line::from(format!("CPU: {:.2}%", process.cpu_usage())));
                    text.push(Line::from(format!("Memory: {} KB", process.memory())));
                    text
                } else {
                    vec![Line::from("Process not found.")]
                }
            } else {
                vec![Line::from("No process associated with this socket.")]
            }
        } else {
            vec![Line::from("No process selected.")]
        }
    } else {
        vec![Line::from("Error loading processes.")]
    };
    let details_paragraph = Paragraph::new(details_text).block(details_block);
    frame.render_widget(details_paragraph, top_chunks[1]);
    frame.render_widget(
        Block::new().borders(Borders::ALL).title("Logs"),
        chunks[1],
    );
}
