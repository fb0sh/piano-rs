use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::net::SocketAddr;
use std::io::{stdout, Write, Result};
use std::path::PathBuf;
use crossterm::{
    cursor,
    input,
    execute,
    RawScreen,
    Clear,
    ClearType,
    InputEvent,
    SyncReader,
    Crossterm,
};
use crossterm_style::Color;

use piano_rs::arguments::Options;
use piano_rs::game::{
    PianoKeyboard,
    GameEvent,
    Note,
    NoteReader,
    screen::pianokeys,
};
use piano_rs::network::{
    NetworkEvent,
    Receiver,
    Sender,
};

fn handle_network_receive_event(
    keyboard: &Arc<Mutex<PianoKeyboard>>,
    event_sender: &Arc<Mutex<Sender>>,
    event_receiver: &Receiver,
) {
    let data = event_receiver.poll_event().unwrap();
    match data.event {
        NetworkEvent::PlayerJoin(port) => {
            let remote_receiver_addr: SocketAddr = format!("{}:{}", data.src.ip(), port)
                .parse()
                .unwrap();

            event_sender.lock().unwrap()
                .register_remote_socket(
                    event_receiver.socket.local_addr().unwrap().port(), remote_receiver_addr
                )
                .unwrap();
        }
        NetworkEvent::Peers(port, mut peers) => {
            peers[0] = format!("{}:{}", data.src.ip(), port).parse().unwrap();
            event_sender.lock().unwrap().peer_addrs = peers;
        }
        NetworkEvent::ID(id) => {
            keyboard.lock().unwrap().set_note_color(match id {
                0 => Color::Blue,
                1 => Color::Red,
                2 => Color::Green,
                3 => Color::Yellow,
                4 => Color::Cyan,
                5 => Color::Magenta,
                _ => Color::Black,
            });
        }
        NetworkEvent::Note(note) => {
            keyboard.lock().unwrap().play_note(note);
        }
       _ => { },
    }
}

fn game_loop(stdin: &mut SyncReader, keyboard: &Arc<Mutex<PianoKeyboard>>, event_sender: &Arc<Mutex<Sender>>) {
    /* let duration = Duration::from_nanos(1000); */

    loop {
        if let Some(event) = stdin.next() {
            if let InputEvent::Keyboard(key) = event {
                match keyboard.lock().unwrap().process_key(key) {
                    Some(GameEvent::Note(note)) => {
                        event_sender.lock().unwrap().tick(note).unwrap();
                    }
                    Some(GameEvent::Quit) => break,
                    None => { },
                }
            }
        }
    }
}

fn play_from_file(play_file: PathBuf, tempo: f32, keyboard: &Arc<Mutex<PianoKeyboard>>, event_sender: &Arc<Mutex<Sender>>) {
    let file_base_notes = NoteReader::from(play_file);
    for file_base_note in file_base_notes.parse_notes() {
        let note = Note::from(
            file_base_note.base_note.as_str(),
            keyboard.lock().unwrap().color,
            file_base_note.duration,
        ).unwrap();
        let normalized_delay = Duration::from_millis(
            (file_base_note.delay.as_millis() as f32 / tempo) as u64
        );
        thread::sleep(normalized_delay);
        event_sender.lock().unwrap().tick(note).unwrap();
    }
}

/// Offsets that center the instrument in a terminal of the given size,
/// clamped so the piano is never pushed off-screen. Without `central` the
/// piano stays pinned to the top-left corner.
fn centering_offsets(central: bool, (width, height): (u16, u16)) -> (u16, u16) {
    if !central {
        return (0, 0);
    }
    let x = ((width as i16 - pianokeys::KEYBOARD_WIDTH as i16) / 2).max(0) as u16;
    let y = ((height as i16 - pianokeys::INSTRUMENT_HEIGHT as i16) / 2).max(0) as u16;
    (x, y)
}

fn main() -> Result<()> {
    let arguments = Options::read();

    // With --central, shift the whole instrument so that it sits in the middle
    // of the terminal: equal margins above and below (and on both sides when
    // the terminal is wider than the 175-column piano). Without the flag the
    // piano stays pinned to the top-left corner, as before.
    let (x_offset, y_offset) = centering_offsets(
        arguments.central,
        Crossterm::new().terminal().size().unwrap_or((80, 24)),
    );

    let receiver_address = arguments.receiver_address;
    let event_receiver = Receiver::new(receiver_address)?;
    let event_sender = Arc::new(Mutex::new(Sender::new(arguments.sender_address, arguments.host_address)?));
    let event_sender_clone = event_sender.clone();

    execute!(stdout(), Clear(ClearType::All)).unwrap();

    let _raw = RawScreen::into_raw_mode();

    let keyboard = Arc::new(Mutex::new(PianoKeyboard::new(
        arguments.sequence,
        arguments.volume,
        arguments.assets,
        Duration::from_millis(arguments.note_duration),
        Duration::from_millis(arguments.mark_duration),
        Color::Blue,
        arguments.show_keys,
        arguments.show_keys || arguments.central,
        x_offset,
        y_offset,
    )));

    keyboard.lock().unwrap().draw().unwrap();

    // Responsive resizing: watch the terminal size and re-center/redraw the
    // whole instrument whenever the window changes (e.g. a split or zoom).
    // Without --central the offsets stay pinned to the top-left corner, but
    // the instrument is still redrawn so content clipped by a shrink is
    // restored when the window grows again.
    let resize_board = keyboard.clone();
    let central = arguments.central;
    thread::spawn(move || {
        let mut last_size = Crossterm::new().terminal().size().unwrap_or((80, 24));
        loop {
            thread::sleep(Duration::from_millis(250));
            let size = Crossterm::new().terminal().size().unwrap_or(last_size);
            if size != last_size {
                let (x_off, y_off) = centering_offsets(central, size);
                let mut kb = resize_board.lock().unwrap();
                kb.set_offsets(x_off, y_off);
                execute!(stdout(), Clear(ClearType::All)).unwrap();
                kb.draw().unwrap();
                last_size = size;
            }
        }
    });

    let cloneboard = keyboard.clone();

    thread::spawn(move || {
        loop {
            handle_network_receive_event(
                &cloneboard,
                &event_sender_clone,
                &event_receiver
            );
        }
    });

    event_sender.lock().unwrap().register_self(arguments.receiver_address.port())?;

    if let Some(v) = arguments.record_file {
        keyboard.lock().unwrap().set_record_file(PathBuf::from(v));
    }

    if let Some(v) = arguments.play_file {
        let play_file = PathBuf::from(v);
        let tempo = arguments.play_file_tempo;
        let fileboard = keyboard.clone();
        let file_notes_sender = event_sender.clone();
        thread::spawn(move || play_from_file(
            play_file,
            tempo,
            &fileboard,
            &file_notes_sender
        ));
    }

    let input = input();
    let mut sync_stdin = input.read_sync();

    let cursor = cursor();
    cursor.hide().unwrap_or_default();

    game_loop(&mut sync_stdin, &keyboard, &event_sender);

    Ok(())
}
