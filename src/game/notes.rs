pub mod embedded;
pub mod play;

// The 37 playable keys. Every entry in `KEYS` maps to the entry with the
// same index in `BASE_SOUNDS`, `INIT_POSES`, `WHITES` and `FACTORS`.
const KEYS: [char; 37] = ['z', 's', 'x', 'c', 'f', 'v', 'g', 'b', 'n',
                          'j', 'm', 'k', '1', ',', 'q', 'l', '2', '.',
                          'w', '/', 'e', '\'', '4', 'r', '5', 't', 'y',
                          '7', 'u', '8', 'i', '9', 'o', 'p', '[', ']', 'a'];

const BASE_SOUNDS: [&str; 37] = ["a", "as", "b", "c", "cs", "d", "ds", "e", "f",
                                 "fs", "g", "gs", "gs", "a", "a", "as", "as", "b",
                                 "b", "c", "c", "cs", "cs", "d", "ds", "e", "f",
                                 "fs", "g", "gs", "a", "as", "b", "c", "d", "e", "gs"];

const INIT_POSES: [i16; 37] = [1, 3, 4, 7, 9, 10, 12, 13, 16,
                               18, 19, 21, 21, 22, 22, 24, 24, 25,
                               25, 28, 28, 30, 30, 31, 33, 34, 37,
                               39, 40, 42, 43, 45, 46, 49, 52, 55, 0];

const WHITES: [bool; 37] = [true, false, true, true, false, true, false, true, true,
                            false, true, false, false, true, true, false, false, true,
                            true, true, true, false, false, true, false, true, true,
                            false, true, false, true, false, true, true, true, true, false];

const FACTORS: [i8; 37] = [-1, -1, -1, 0, 0, 0, 0, 0, 0,
                           0, 0, 0, 0, 0, 0, 0, 0, 0,
                           0, 1, 1, 1, 1, 1, 1, 1, 1,
                           1, 1, 1, 1, 1, 1, 2, 2, 2, -1];

// Characters produced when the playable number/punctuation keys are pressed
// with Shift held, and the plain key each one maps to.
const SPECIAL_KEYS: [char; 13] = ['!', '@', '$', '%', '&', '*', '(', '"', '<',
                                  '>', '?', '{', '}'];
const SPECIAL_MATCHES: [char; 13] = ['1', '2', '4', '5', '7', '8', '9', '\'', ',',
                                     '.', '/', '[', ']'];

use std::num::ParseIntError;
use std::convert::Infallible;
use serde_derive::{Serialize, Deserialize};
use std::time::Duration;
use crossterm::KeyEvent;
use crossterm_style::Color;
pub use play::Player;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Note {
    pub sound: String,
    pub base: String,
    pub frequency: i8,
    pub position: i16,
    pub white: bool,
    pub color: Color,
    pub duration: Duration,
}

impl Note {
    pub fn from(sound: &str, color: Color, duration: Duration) -> Option<Note> {
        match Self::extract_base_sound_and_frequency(sound) {
            (Ok(base_sound), Ok(frequency)) => {
                let parsed_note = Self::parse_note(
                    &base_sound,
                    frequency,
                    color,
                    duration,
                );
                parsed_note.ok()
            }
            _ => None,
        }
    }

    fn extract_base_sound_and_frequency(note: &str) -> (Result<String, Infallible>, Result<i8, ParseIntError>) {
        let mut base_sound: Result<String, Infallible> = note.parse();
        let mut frequency: Result<i8, ParseIntError> = note.parse();
        for start_index in 1..note.len() {
            frequency = note[start_index..].parse();
            base_sound = note[..start_index].parse();
            if frequency.is_ok() {
                break;
            }
        }
        (base_sound, frequency)
    }

    fn parse_note(base_sound: &str, frequency: i8, color: Color, duration: Duration) -> Result<Note, String> {
        let index = BASE_SOUNDS.iter()
                       .position(|&key| key == base_sound);

        match index {
            Some(v) => Ok(Note {
                sound: format!("{}{}", base_sound, frequency),
                base: BASE_SOUNDS[v].to_string(),
                frequency,
                position: INIT_POSES[v] + 21 * ((frequency - FACTORS[v]) as i16),
                white: WHITES[v],
                color,
                duration,
            }),
            None => Err(String::from("We're Fucked.")),
        }
    }

    pub fn play(&self, player: &Player, volume: f32) {
        player.play(&self.base, self.frequency, self.duration, volume);
    }
}

pub fn key_to_base_note(mut key: KeyEvent, sequence: i8) -> Option<String> {
    let mut offset: i8 = 0;

    // Handle terminal control characters
    if key == KeyEvent::Enter {
        // Ctrl+m sends Enter in terminal
        key = KeyEvent::Ctrl('m');
    } else if key == KeyEvent::Tab {
        // Ctrl+i sends Tab in terminal
        key = KeyEvent::Ctrl('i');
    }

    // Translate Ctrl+<character> and Alt+<character> to <character>, playing
    // the note one octave lower, like Shift plays it one octave higher.
    if let KeyEvent::Ctrl(c) = key {
        key = KeyEvent::Char(c);
        offset -= 1;
    } else if let KeyEvent::Alt(c) = key {
        key = KeyEvent::Char(c);
        offset -= 1;
    }

    // Increment `offset` if key was shift prefixed (Shift+<character>)
    let note: Option<String> = if let KeyEvent::Char(mut c) = key {
        if c.is_uppercase() {
            c = c.to_ascii_lowercase();
            offset += 1;
        } else if SPECIAL_KEYS.contains(&c) {
            let j = SPECIAL_KEYS.iter()
                           .position(|&key| key == c)
                           .unwrap();
            c = SPECIAL_MATCHES[j];
            offset += 1;
        }

        if let Some(i) = KEYS.iter().position(|&key| key == c) {
            let factor = FACTORS[i];

            let base_note = format!("{}{}",
                BASE_SOUNDS[i].to_string(),
                offset + factor + sequence
            );

            Some(base_note)

        } else {
            None
        }
    } else {
        None
    };

    note
}

/// Returns the temporary frequency offset implied by the modifier held with a
/// key event: -1 while Ctrl or Alt is held, +1 while Shift is held, 0 for a
/// plain key, and None for keys that don't carry a modifier (arrows, Esc, ...).
/// Note positions and the --show-keys labels shift by 21 columns per offset.
/// The space bar is the sustain pedal, so it doesn't carry a note modifier.
pub fn modifier_offset(key: &KeyEvent) -> Option<i8> {
    match key {
        KeyEvent::Ctrl(_) | KeyEvent::Alt(_) => Some(-1),
        KeyEvent::Char(c) if c.is_uppercase() || SPECIAL_KEYS.contains(c) => Some(1),
        KeyEvent::Char(' ') => None,
        KeyEvent::Char(_) => Some(0),
        _ => None,
    }
}

/// Returns the keyboard letter, the base on-screen position and whether the
/// key is white, for each of the 37 playable keys. The actual on-screen
/// position for a given frequency `sequence` is `pos + 21 * sequence`.
pub fn key_labels() -> Vec<(char, i16, bool)> {
    KEYS.iter()
        .zip(INIT_POSES.iter())
        .zip(WHITES.iter())
        .map(|((&key, &pos), &white)| (key, pos, white))
        .collect()
}

#[cfg(test)]
mod test {
    #[test]
    fn note_from() {
        let expected_note = super::Note {
            sound: "a2".to_string(),
            base: "a".to_string(),
            frequency: 2,
            position: 64,
            white: true,
            color: super::Color::Blue,
            duration: super::Duration::from_millis(100)
        };

        match super::Note::from("a2", super::Color::Blue, super::Duration::from_millis(100)) {
            Some(actual_note) => assert_eq!(actual_note, expected_note),
            None => panic!("This note should have been parsable!"),
        }
    }

    #[test]
    fn extract_base_sound_and_frequency_from_sound() {
        match super::Note::extract_base_sound_and_frequency("a2") {
            (Ok(v), Ok(w)) => assert_eq!((v.as_str(), w), ("a", 2)),
            _ => panic!("This sound should have been parsable!"),
        }
    }

    #[test]
    fn parse_note() {
        let actual_note = super::Note::parse_note(
            "a",
            2,
            super::Color::Blue,
            super::Duration::from_millis(100)
        ).unwrap();

        let expected_note = super::Note {
            sound: "a2".to_string(),
            base: "a".to_string(),
            frequency: 2,
            position: 64,
            white: true,
            color: super::Color::Blue,
            duration: super::Duration::from_millis(100),
        };

        assert_eq!(actual_note, expected_note);
    }

    #[test]
    fn parse_note_err() {
        let note = super::Note::parse_note(
            "z",
            9,
            super::Color::Blue,
            super::Duration::from_millis(100)
        );
        assert!(note.is_err());
    }

    #[test]
    fn key_to_base_note() {
        let base_note = super::key_to_base_note(super::KeyEvent::Char('a'), 2);
        match base_note {
            Some(v) => assert_eq!(v, "gs1"),
            None => panic!("The key should have been parsable!"),
        }
    }

    #[test]
    fn key_to_base_note_none() {
        let base_note = super::key_to_base_note(super::KeyEvent::Char('~'), 2);
        assert!(base_note.is_none());
    }

    #[test]
    fn key_to_base_note_alt_shifts_octave_down() {
        use super::KeyEvent;
        // Alt+<key> plays the note one octave lower (same as Ctrl).
        let base_note = super::key_to_base_note(KeyEvent::Alt('a'), 2);
        match base_note {
            Some(v) => assert_eq!(v, "gs0"),
            None => panic!("The key should have been parsable!"),
        }
    }

    #[test]
    fn modifier_offset_tracking() {
        use super::KeyEvent;
        assert_eq!(super::modifier_offset(&KeyEvent::Ctrl('z')), Some(-1));
        assert_eq!(super::modifier_offset(&KeyEvent::Alt('z')), Some(-1));
        assert_eq!(super::modifier_offset(&KeyEvent::Char('Z')), Some(1));
        assert_eq!(super::modifier_offset(&KeyEvent::Char('!')), Some(1));
        assert_eq!(super::modifier_offset(&KeyEvent::Char('z')), Some(0));
        assert_eq!(super::modifier_offset(&KeyEvent::Char(' ')), None);
        assert_eq!(super::modifier_offset(&KeyEvent::Backspace), None);
        assert_eq!(super::modifier_offset(&KeyEvent::Right), None);
        assert_eq!(super::modifier_offset(&KeyEvent::Esc), None);
    }

    #[test]
    fn key_labels_len_and_content() {
        let labels = super::key_labels();
        assert_eq!(labels.len(), 37);
        assert!(labels.contains(&('z', 1, true)));
        assert!(labels.contains(&('s', 3, false)));
        assert!(labels.contains(&('a', 0, false)));
    }
}

