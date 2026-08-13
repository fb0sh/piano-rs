pub mod screen;
pub mod notes;
pub mod notes_file;

use std::time::Duration;
use std::path::PathBuf;
pub use notes::Note;
pub use notes::Player;
pub use notes_file::{NoteReader, FileNote, NoteRecorder};
use screen::pianokeys;
use serde_derive::{Serialize, Deserialize};
use crossterm::{KeyEvent, Result};
use crossterm_style::Color;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameEvent {
    Note(Note),
    Quit,
}

pub struct PianoKeyboard {
    sequence: i8,
    modifier_offset: i8,
    volume: f32,
    sound_duration: Duration,
    mark_duration: Duration,
    show_keys: bool,
    x_offset: u16,
    y_offset: u16,
    pub color: Color,
    player: Player,
    recorder: NoteRecorder,
}

impl PianoKeyboard {
    pub fn new(sequence: i8, volume: f32, assets: Option<PathBuf>, sound_duration: Duration, mark_duration: Duration, color: Color, show_keys: bool, x_offset: u16, y_offset: u16) -> PianoKeyboard {
        let player = match assets {
            Some(assets_path) => Player::from(assets_path),
            None => Player::new(),
        };

        PianoKeyboard {
            sequence,
            modifier_offset: 0,
            volume,
            sound_duration,
            mark_duration,
            show_keys,
            x_offset,
            y_offset,
            color,
            player,
            recorder: NoteRecorder::new(),
        }
    }

    pub fn set_record_file(&mut self, record_file: PathBuf) {
        self.recorder.set_file(record_file);
    }

    pub fn draw(&self) -> Result<()> {
        pianokeys::draw(self.show_keys, self.sequence, self.modifier_offset, self.x_offset, self.y_offset)?;
        Ok(())
    }

    pub fn play_note(&mut self, note: Note) {
        note.play(&self.player, self.volume);

        screen::mark_note(
            note.position,
            note.white,
            note.color,
            self.mark_duration,
            self.x_offset,
            self.y_offset,
        );

        if self.recorder.record_file.is_some(){
            self.recorder.write_note(note);
        }
    }

    pub fn set_note_color(&mut self, color: Color) {
        self.color = color;
    }

    pub fn process_key(&mut self, key: KeyEvent) -> Option<GameEvent> {
        // Shift/Ctrl temporarily shift the note mapping by one octave. Keep
        // the --show-keys labels in sync with the positions the notes will
        // actually land on.
        if let Some(new_offset) = notes::modifier_offset(&key) {
            if new_offset != self.modifier_offset {
                if self.show_keys {
                    pianokeys::hide_labels(self.sequence, self.modifier_offset, self.x_offset, self.y_offset).unwrap();
                }
                self.modifier_offset = new_offset;
                if self.show_keys {
                    pianokeys::show_labels(self.sequence, self.modifier_offset, self.x_offset, self.y_offset).unwrap();
                }
            }
        }

        match key {
            KeyEvent::Right => {
                if self.sequence < 6 {
                    if self.show_keys {
                        pianokeys::hide_labels(self.sequence, self.modifier_offset, self.x_offset, self.y_offset).unwrap();
                    }
                    self.sequence += 1;
                    if self.show_keys {
                        pianokeys::show_labels(self.sequence, self.modifier_offset, self.x_offset, self.y_offset).unwrap();
                    }
                }
                None
            }
            KeyEvent::Left => {
                if self.sequence > 0 {
                    if self.show_keys {
                        pianokeys::hide_labels(self.sequence, self.modifier_offset, self.x_offset, self.y_offset).unwrap();
                    }
                    self.sequence -= 1;
                    if self.show_keys {
                        pianokeys::show_labels(self.sequence, self.modifier_offset, self.x_offset, self.y_offset).unwrap();
                    }
                }
                None
            }
            KeyEvent::Up => {
                // The note sound files are maximum 8s in length
                if self.sound_duration < Duration::from_millis(8000) {
                    self.sound_duration += Duration::from_millis(50);
                }
                None
            }
            KeyEvent::Down => {
                if self.sound_duration > Duration::new(0, 0) {
                    self.sound_duration -= Duration::from_millis(50);
                }
                None
            }
            KeyEvent::Char('+') => {
                self.volume += 0.1;
                None
            }
            KeyEvent::Char('-') => {
                self.volume -= 0.1;
                None
            }
            KeyEvent::Esc => {
                Some(GameEvent::Quit)
            }
            _ => notes::key_to_base_note(key, self.sequence)
                .and_then(|note| Note::from(&note, self.color, self.sound_duration))
                .map(GameEvent::Note),
        }
    }
}

#[cfg(test)]
mod test {
    use super::{
        PianoKeyboard,
        Color,
        KeyEvent,
        Player,
        Duration,
        GameEvent,
        Note,
        NoteRecorder,
    };

    #[test]
    fn new_pianokeyboard() {
        let actual_keyboard = PianoKeyboard::new(
            2,
            0.4,
            None,
            Duration::from_millis(7000),
            Duration::from_millis(500),
            Color::Blue,
            false,
            0,
            0,
        );

        let expected_keyboard = PianoKeyboard {
            sequence: 2,
            modifier_offset: 0,
            volume: 0.4,
            sound_duration: Duration::from_millis(7000),
            mark_duration: Duration::from_millis(500),
            show_keys: false,
            x_offset: 0,
            y_offset: 0,
            color: Color::Blue,
            player: Player::new(),
            recorder: NoteRecorder::new(),
        };

        assert_eq!(actual_keyboard.sequence, expected_keyboard.sequence);
        assert_eq!(actual_keyboard.volume, expected_keyboard.volume);
        assert_eq!(actual_keyboard.sound_duration, expected_keyboard.sound_duration);
        assert_eq!(actual_keyboard.mark_duration, expected_keyboard.mark_duration);
        assert_eq!(actual_keyboard.color, expected_keyboard.color);
    }

    #[test]
    fn set_note_color() {
        let mut keyboard = PianoKeyboard::new(
            2,
            0.4,
            None,
            Duration::from_millis(7000),
            Duration::from_millis(500),
            Color::Blue,
            false,
            0,
            0,
        );
        keyboard.set_note_color(Color::Red);
        assert_eq!(keyboard.color, Color::Red);
    }

    #[test]
    fn process_increase_volume_key() {
        let mut keyboard = PianoKeyboard::new(
            2,
            0.4,
            None,
            Duration::from_millis(7000),
            Duration::from_millis(500),
            Color::Blue,
            false,
            0,
            0,
        );

        let event = keyboard.process_key(KeyEvent::Char('+'));
        assert!(event.is_none());
        assert_eq!(keyboard.volume, 0.5);
    }

    #[test]
    fn process_decrease_volume_key() {
        let mut keyboard = PianoKeyboard::new(
            2,
            0.4,
            None,
            Duration::from_millis(7000),
            Duration::from_millis(500),
            Color::Blue,
            false,
            0,
            0,
        );

        let event = keyboard.process_key(KeyEvent::Char('-'));
        assert!(event.is_none());
        assert_eq!(keyboard.volume, 0.3);
    }

    #[test]
    fn process_increase_sequence_key() {
        let mut keyboard = PianoKeyboard::new(
            2,
            0.4,
            None,
            Duration::from_millis(7000),
            Duration::from_millis(500),
            Color::Blue,
            false,
            0,
            0,
        );

        let event = keyboard.process_key(KeyEvent::Right);
        assert!(event.is_none());
        assert_eq!(keyboard.sequence, 3);
    }

    #[test]
    fn process_decrease_sequence_key() {
        let mut keyboard = PianoKeyboard::new(
            2,
            0.4,
            None,
            Duration::from_millis(7000),
            Duration::from_millis(500),
            Color::Blue,
            false,
            0,
            0,
        );

        let event = keyboard.process_key(KeyEvent::Left);
        assert!(event.is_none());
        assert_eq!(keyboard.sequence, 1);
    }

    #[test]
    fn process_shifted_key_updates_modifier_offset() {
        let mut keyboard = PianoKeyboard::new(
            2,
            0.4,
            None,
            Duration::from_millis(7000),
            Duration::from_millis(500),
            Color::Blue,
            false,
            0,
            0,
        );

        // Shift+z lands one octave higher.
        let event = keyboard.process_key(KeyEvent::Char('Z'));
        assert!(event.is_some());
        assert_eq!(keyboard.modifier_offset, 1);

        // A plain key returns to the base mapping.
        keyboard.process_key(KeyEvent::Char('z'));
        assert_eq!(keyboard.modifier_offset, 0);

        // Ctrl lands one octave lower.
        keyboard.process_key(KeyEvent::Ctrl('z'));
        assert_eq!(keyboard.modifier_offset, -1);

        // Arrow keys keep the current modifier.
        keyboard.process_key(KeyEvent::Right);
        assert_eq!(keyboard.modifier_offset, -1);
    }

    #[test]
    fn process_increase_note_duration_key() {
        let mut keyboard = PianoKeyboard::new(
            2,
            0.4,
            None,
            Duration::from_millis(7000),
            Duration::from_millis(500),
            Color::Blue,
            false,
            0,
            0,
        );

        let event = keyboard.process_key(KeyEvent::Up);
        assert!(event.is_none());
        assert_eq!(keyboard.sound_duration, Duration::from_millis(7050));
    }

    #[test]
    fn process_decrease_note_duration_key() {
        let mut keyboard = PianoKeyboard::new(
            2,
            0.4,
            None,
            Duration::from_millis(7000),
            Duration::from_millis(500),
            Color::Blue,
            false,
            0,
            0,
        );

        let event = keyboard.process_key(KeyEvent::Down);
        assert!(event.is_none());
        assert_eq!(keyboard.sound_duration, Duration::from_millis(6950));
    }

    #[test]
    fn process_quit_key() {
        let mut keyboard = PianoKeyboard::new(
            2,
            0.4,
            None,
            Duration::from_millis(7000),
            Duration::from_millis(500),
            Color::Blue,
            false,
            0,
            0,
        );

        let event = keyboard.process_key(KeyEvent::Esc);
        match event {
            Some(GameEvent::Quit) => assert!(true),
            _ => panic!("This key should have returned a Quit event!"),
        }
    }

    #[test]
    fn process_note_key() {
        let mut keyboard = PianoKeyboard::new(
            2,
            0.4,
            None,
            Duration::from_millis(7000),
            Duration::from_millis(500),
            Color::Blue,
            false,
            0,
            0,
        );

        let event = keyboard.process_key(KeyEvent::Char('a'));

        let expected_note = Note {
            sound: "gs1".to_string(),
            base: "gs".to_string(),
            frequency: 1,
            position: 42,
            white: false,
            color: Color::Blue,
            duration: Duration::from_millis(7000),
        };

        match event {
            Some(GameEvent::Note(v)) => assert_eq!(v, expected_note),
            _ => panic!("This key should have returned a corresponding Note!"),
        }
    }
}
