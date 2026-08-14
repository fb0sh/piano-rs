use std::{thread, time};

use crossterm::{
    style,
    queue,
    Colorize,
    Goto,
    PrintStyledFont,
};

use crossterm_style::Color;

use std::io::{stdout, Write};

/*
█▒
*/

pub mod pianokeys {
    use crossterm::{
        queue,
        style,
        Colorize,
        Crossterm,
        Goto,
        PrintStyledFont,
        Result,
    };

    use std::io::{stdout, Stdout, Write};
    use std::time::Duration;

    use super::super::notes;
    use super::Color;

    struct Point {
        x: u16,
        y: u16,
    }

    // The white keys are 3 columns wide and there are 58 of them, so the
    // keyboard body is 175 columns wide. On a real piano the sustain pedal
    // sits on the right of the pedal board, so the pedal hangs near the
    // right edge of the keyboard, sized to a fraction of the keyboard width.
    pub const KEYBOARD_WIDTH: u16 = 175;
    const PEDAL_HEIGHT: u16 = 3;
    const PEDAL_WIDTH: u16 = 30;
    const PEDAL_X: u16 = KEYBOARD_WIDTH - PEDAL_WIDTH - 8;

    // The keyboard is 16 rows tall at full size with black keys 9 rows tall:
    // a fixed 16:9 ratio between white and black key heights. When the
    // terminal is short the whole instrument scales down in discrete steps
    // (16, 14, 12, ... key rows) keeping that ratio, and on very small
    // terminals the hint panel, pedal and status row are dropped so the
    // piano still fits.
    const KEY_HEIGHT_STEPS: [u16; 8] = [16, 14, 12, 10, 8, 6, 4, 2];

    /// Where and how big the instrument is drawn. Computed from the terminal
    /// height so the piano adapts when the window is resized.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct InstrumentLayout {
        /// Rows of white key body (16 at full size).
        pub white_height: u16,
        /// Draw the sustain pedal below the keys.
        pub show_pedal: bool,
        /// Draw the volume/note/octave status row.
        pub show_status: bool,
        /// Draw the key-hint panel in the top-right corner (still gated on
        /// `--show-keys` by the caller).
        pub show_hints: bool,
        /// Total rows the instrument occupies (keys + gap + optional pedal
        /// and status row).
        pub height: u16,
    }

    /// Black key height for a white key height, keeping the 16:9 ratio
    /// between white and black keys (rounded to whole rows).
    pub fn black_height(white_height: u16) -> u16 {
        (white_height * 9 + 8) / 16
    }

    /// The largest instrument layout that fits in a terminal `term_height`
    /// rows tall. The key rows scale down first (16:9 ratio kept), then on
    /// very small terminals the hint panel, the pedal and finally the status
    /// row are dropped so the piano still fits.
    pub fn instrument_layout(term_height: u16) -> InstrumentLayout {
        let mut white = 2;
        for &w in &KEY_HEIGHT_STEPS {
            if w + 5 <= term_height {
                white = w;
                break;
            }
        }
        let mut show_pedal = true;
        let mut show_status = true;
        if white + 5 > term_height {
            show_pedal = false;
            if white + 2 > term_height {
                show_status = false;
            }
        }
        let show_hints = white >= 12;
        let height = white + 1
            + if show_pedal { 3 } else { 0 }
            + if show_status { 1 } else { 0 };
        InstrumentLayout {
            white_height: white,
            show_pedal,
            show_status,
            show_hints,
            height,
        }
    }

    // Row helpers relative to the key heights: note marks sit on the bottom
    // row of a key, labels just above them. On tiny keyboards the label rows
    // can fall off the top; callers skip rows that are negative.
    fn white_label_row(white_height: u16) -> u16 {
        white_height - 2
    }
    fn black_label_rows(white_height: u16) -> (i16, i16) {
        let b = black_height(white_height) as i16;
        (b - 2, b - 3)
    }

    // Key hint lines shown at the top-right corner of the terminal (with
    // `-k` or `-c`): every piano control key gets on-screen feedback.
    const HINT_ROWS: [&str; 5] = [
        "Shift+Key  Octave +1",
        "Alt+Key    Octave -1",
        "Arrows     Change octave",
        "Space      Sustain",
        "Backspace  Sustain lock",
    ];

    /// True when a label or mark of `cells` columns, starting at the absolute
    /// screen column `screen_pos` (x_offset already added), fits inside the
    /// keyboard body. Relative column 0 is the left border of the first white
    /// key (the `a` key maps there but no black key is rendered at that
    /// column), so the paintable body starts at relative column 1. At extreme
    /// octaves shifted labels/marks would land outside the body: they are not
    /// drawn, and anything left there is erased, so no block lingers on the
    /// background.
    pub fn in_body(screen_pos: i16, cells: u16, x_offset: u16) -> bool {
        screen_pos > x_offset as i16
            && screen_pos + cells as i16 - 1 <= (x_offset + KEYBOARD_WIDTH - 1) as i16
    }

    pub fn draw(show_keys: bool, sequence: i8, offset: i8, white_height: u16, x_offset: u16, y_offset: u16) -> Result<()> {
        let mut stdout = stdout();
        print_whites(&mut stdout, white_height, x_offset, y_offset)?;
        print_blacks(&mut stdout, black_height(white_height), x_offset, y_offset)?;
        if show_keys {
            draw_labels(sequence, offset, white_height, x_offset, y_offset, &mut stdout)?;
        }
        stdout.flush()?;
        Ok(())
    }

    /// Draws the keyboard letter on each playable key for the given frequency
    /// sequence and modifier offset (0 normally, -1 while Ctrl is held, +1
    /// while Shift is held). `x_offset`/`y_offset` shift the whole keyboard.
    pub fn show_labels(sequence: i8, offset: i8, white_height: u16, x_offset: u16, y_offset: u16) -> Result<()> {
        let mut stdout = stdout();
        draw_labels(sequence, offset, white_height, x_offset, y_offset, &mut stdout)?;
        stdout.flush()?;
        Ok(())
    }

    /// Restores the key surfaces where labels were drawn, e.g. before redrawing
    /// them at a new sequence or modifier offset. Labels that would sit off the
    /// right edge of the keyboard body are erased instead of repainted, so
    /// shifting the octave back never leaves white blocks on the background.
    pub fn hide_labels(sequence: i8, offset: i8, white_height: u16, x_offset: u16, y_offset: u16) -> Result<()> {
        let mut stdout = stdout();
        for (_key, pos, white) in notes::key_labels() {
            let screen_pos = pos + 21 * (sequence as i16 + offset as i16) + x_offset as i16;
            // Nothing is ever drawn left of the keyboard body or on relative
            // column 0 (the `a` key's position is the border of the first
            // white key, where no black key is rendered), so there is nothing
            // to restore there either: repainting would leave black blocks.
            if screen_pos <= x_offset as i16 {
                continue;
            }
            if white {
                // White labels are only drawn on keyboards tall enough to
                // have a row above the mark row.
                if white_height < 4 {
                    continue;
                }
                if in_body(screen_pos, 2, x_offset) {
                    queue!(
                        stdout,
                        Goto(screen_pos as u16, white_label_row(white_height) + y_offset),
                        PrintStyledFont("██".white())
                    )?;
                } else {
                    // Off the right edge: erase any label drawn there at a
                    // higher octave so it doesn't linger.
                    queue!(
                        stdout,
                        Goto(screen_pos as u16, white_label_row(white_height) + y_offset),
                        PrintStyledFont(style("  "))
                    )?;
                }
            } else {
                let rows = black_label_rows(white_height);
                for row in &[rows.0, rows.1] {
                    if *row < 0 {
                        continue;
                    }
                    let row = *row as u16;
                    if in_body(screen_pos, 1, x_offset) {
                        queue!(
                            stdout,
                            Goto(screen_pos as u16, row + y_offset),
                            PrintStyledFont("█".black())
                        )?;
                    } else {
                        queue!(
                            stdout,
                            Goto(screen_pos as u16, row + y_offset),
                            PrintStyledFont(style(" "))
                        )?;
                    }
                }
            }
        }
        stdout.flush()?;
        Ok(())
    }

    fn draw_labels(sequence: i8, offset: i8, white_height: u16, x_offset: u16, y_offset: u16, stdout: &mut Stdout) -> Result<()> {
        // Group the labels by their on-screen position, since some keys map to
        // the same note (e.g. `,` and `q` are both A on the white key at 22).
        let mut groups: Vec<(u16, Vec<char>, bool)> = Vec::new();
        for (key, pos, white) in notes::key_labels() {
            let screen_pos = pos + 21 * (sequence as i16 + offset as i16) + x_offset as i16;
            if screen_pos < 0 {
                continue;
            }
            let screen_pos = screen_pos as u16;
            match groups.iter_mut().find(|(p, _, w)| *p == screen_pos && *w == white) {
                Some((_, chars, _)) => chars.push(key),
                None => groups.push((screen_pos, vec![key], white)),
            }
        }

        for (pos, chars, white) in groups {
            // Skip labels that would land outside the keyboard body (off the
            // right edge at high octaves): nothing is drawn there, so nothing
            // lingers when the octave moves back.
            let cells = if white { 2 } else { 1 };
            if !in_body(pos as i16, cells, x_offset) {
                continue;
            }
            if white {
                if white_height < 4 {
                    continue;
                }
                // White keys have a 2-column wide body, so up to two letters
                // fit side by side.
                for (i, c) in chars.iter().take(2).enumerate() {
                    queue!(
                        stdout,
                        Goto(pos + i as u16, white_label_row(white_height) + y_offset),
                        PrintStyledFont(style(c.to_string()).black().on_white())
                    )?;
                }
            } else {
                // Black keys are 1 column wide; stack extra letters vertically
                // on the black-label rows (bottom row first). On tiny
                // keyboards the upper row may not exist.
                let rows = black_label_rows(white_height);
                for (i, c) in chars.iter().take(2).enumerate() {
                    let row = if i == 0 { rows.0 } else { rows.1 };
                    if row >= 0 {
                        queue!(
                            stdout,
                            Goto(pos, row as u16 + y_offset),
                            PrintStyledFont(style(c.to_string()).white().on_black())
                        )?;
                    }
                }
            }
        }
        Ok(())
    }

    fn print_whitekey(initial_point: Point, key_height: u16, stdout: &mut Stdout) -> Result<()> {
        for column_height in 0..key_height {
            queue!(
                stdout,
                Goto(initial_point.x, initial_point.y + column_height),
                PrintStyledFont("|".black().on_white())
            )?;
            queue!(
                stdout,
                Goto(initial_point.x + 1, initial_point.y + column_height),
                PrintStyledFont("██".white())
            )?;
            queue!(
                stdout,
                Goto(initial_point.x + 3, initial_point.y + column_height),
                PrintStyledFont("|".black())
            )?;
        }
        Ok(())
    }

    fn print_whites(stdout: &mut Stdout, white_height: u16, x_offset: u16, y_offset: u16) -> Result<()> {
        for key in 0..58 {
            let initial_point = Point { x: key * 3 + x_offset, y: y_offset };
            print_whitekey(initial_point, white_height, stdout)?;
        }
        Ok(())
    }

    fn print_blackkey(initial_point: Point, key_height: u16, stdout: &mut Stdout) -> Result<()> {
        for column_height in 0..key_height {
            queue!(
                stdout,
                Goto(initial_point.x, initial_point.y + column_height),
                PrintStyledFont("█".black())
            )?;
        }
        Ok(())
    }

    fn print_blacks(stdout: &mut Stdout, black_height: u16, x_offset: u16, y_offset: u16) -> Result<()> {
        // First black key is lonely
        let mut initial_point = Point { x: 3 + x_offset, y: y_offset };
        print_blackkey(initial_point, black_height, stdout)?;

        for x in 0..8 {
            let g1k1 = x * 21 + 9 + x_offset;
            let g1k2 = g1k1 + 3;
            initial_point = Point { x: g1k1, y: y_offset };
            print_blackkey(initial_point, black_height, stdout)?;
            initial_point = Point { x: g1k2, y: y_offset };
            print_blackkey(initial_point, black_height, stdout)?;

            let g2k1 = g1k2 + 6;
            let g2k2 = g2k1 + 3;
            let g2k3 = g2k2 + 3;
            initial_point = Point { x: g2k1, y: y_offset };
            print_blackkey(initial_point, black_height, stdout)?;
            initial_point = Point { x: g2k2, y: y_offset };
            print_blackkey(initial_point, black_height, stdout)?;
            initial_point = Point { x: g2k3, y: y_offset };
            print_blackkey(initial_point, black_height, stdout)?;
        }

        Ok(())
    }

    /// Draws the sustain pedal below the keyboard (the space bar toggles it).
    /// `on` lights the pedal up while sustain is active; with `show_keys` the
    /// SPACE label is drawn centered on the pedal.
    pub fn draw_pedal(on: bool, show_keys: bool, white_height: u16, x_offset: u16, y_offset: u16) -> Result<()> {
        let mut stdout = stdout();
        let pedal_row = white_height + 1;
        let bar_color = if on { Color::Yellow } else { Color::DarkGrey };

        for row in 0..PEDAL_HEIGHT {
            for x in 0..PEDAL_WIDTH {
                queue!(
                    stdout,
                    Goto(PEDAL_X + x + x_offset, pedal_row + row + y_offset),
                    PrintStyledFont(style("█").with(bar_color))
                )?;
            }
        }

        if show_keys {
            let label = "SPACE";
            let start_x = PEDAL_X + x_offset + (PEDAL_WIDTH - label.len() as u16) / 2;
            let label_row = pedal_row + 1 + y_offset;
            for (i, c) in label.chars().enumerate() {
                let cell = if on {
                    style(c.to_string()).black().on_yellow()
                } else {
                    style(c.to_string()).white().on_dark_grey()
                };
                queue!(
                    stdout,
                    Goto(start_x + i as u16, label_row),
                    PrintStyledFont(cell)
                )?;
            }
        }

        stdout.flush()?;
        Ok(())
    }

    /// Text shown in the status row below the pedal. All fields are fixed
    /// width so the line never changes length. A duration of 0 means "play
    /// until the sample ends" (the longest note), which is shown as `ring`
    /// so the display never lies about the sound.
    pub fn status_text(volume: f32, duration: Duration, sequence: i8) -> String {
        let note_field = if duration.is_zero() {
            "ring ".to_string()
        } else {
            format!("{:.2}s", duration.as_millis() as f64 / 1000.0)
        };
        format!(
            "Volume: {:.2}  Note: {}  Octave: {}",
            volume, note_field, sequence
        )
    }

    /// Draws the status line below the pedal: volume, note length and octave.
    /// It is the on-screen feedback for the `+`/`-` (volume), `Up`/`Down`
    /// (note length) and `Left`/`Right` (octave) keys. The row it is drawn on
    /// follows the keyboard and pedal: with the pedal hidden the status moves
    /// up next to the gap row.
    pub fn draw_status(volume: f32, duration: Duration, sequence: i8, white_height: u16, show_pedal: bool, x_offset: u16, y_offset: u16) -> Result<()> {
        let mut stdout = stdout();
        let status_row = white_height + 1 + if show_pedal { PEDAL_HEIGHT } else { 0 };
        // Clear the whole row first so a shorter text never leaves a tail.
        queue!(
            stdout,
            Goto(x_offset, status_row + y_offset),
            PrintStyledFont(style(" ".repeat(KEYBOARD_WIDTH as usize)))
        )?;
        let text = status_text(volume, duration, sequence);
        let start_x = x_offset + (KEYBOARD_WIDTH - text.len() as u16) / 2;
        queue!(
            stdout,
            Goto(start_x, status_row + y_offset),
            PrintStyledFont(style(text).grey())
        )?;
        stdout.flush()?;
        Ok(())
    }

    /// Draws the key hints at the top-right corner of the terminal (rows
    /// 0..4, right-aligned as a block). Shown when hints are enabled
    /// (`-k` or `-c`).
    pub fn draw_hints(enabled: bool) -> Result<()> {
        if !enabled {
            return Ok(());
        }
        let mut stdout = stdout();
        // Right-align the whole panel against the terminal's right edge
        // (falling back to the keyboard's right edge if the size can't be
        // queried). Every line shares the same start column, so the key
        // names and their descriptions line up in two tidy columns.
        let (width, _) = Crossterm::new().terminal().size().unwrap_or((KEYBOARD_WIDTH, 24));
        let right = if width > KEYBOARD_WIDTH + 1 { width } else { KEYBOARD_WIDTH };
        let max_len = HINT_ROWS.iter().map(|l| l.len()).max().unwrap_or(0) as i32;
        let start = (right as i32 - max_len).max(0) as u16;
        for (i, line) in HINT_ROWS.iter().enumerate() {
            queue!(
                stdout,
                Goto(start, i as u16),
                PrintStyledFont(style(line.to_string()).grey())
            )?;
        }
        stdout.flush()?;
        Ok(())
    }
}

pub fn mark_note(pos: i16, white: bool, color: Color, duration: time::Duration, white_height: u16, x_offset: u16, y_offset: u16) {
    // Notes at extreme octaves can sit outside the keyboard body (e.g. '[' at
    // octave 6): draw no mark there and don't spawn a restore that would
    // paint key-surface blocks onto the background.
    if !pianokeys::in_body(pos + x_offset as i16, if white { 2 } else { 1 }, x_offset) {
        return;
    }
    if white {
        // This causes a compiler panic!
        /* queue!( */
        /*     stdout(), */
        /*     Goto(pos as u16, 15), */
        /*     PrintStyledFont(StyledObject("██").with(color)) */
        /* ).unwrap(); */

        let mark_row = white_height - 1;
        queue!(
            stdout(),
            Goto(pos as u16 + x_offset, mark_row + y_offset),
            PrintStyledFont(style("██").with(color))
        ).unwrap();

    /* println!("{} Red foreground text", Colored::Fg(Color::Red)); */
    } else {
        let mark_row = pianokeys::black_height(white_height) - 1;
        queue!(
            stdout(),
            Goto(pos as u16 + x_offset, mark_row + y_offset),
            PrintStyledFont(style("█").with(color))
        ).unwrap();
    }

    thread::spawn(move || {
        thread::sleep(duration);
        if white {
        let mark_row = white_height - 1;
        queue!(
            stdout(),
            Goto(pos as u16 + x_offset, mark_row + y_offset),
            PrintStyledFont("██".white())
        ).unwrap();
        } else {
        let mark_row = pianokeys::black_height(white_height) - 1;
        queue!(
            stdout(),
            Goto(pos as u16 + x_offset, mark_row + y_offset),
            PrintStyledFont("█".black())
        ).unwrap();
        }
    });
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use super::pianokeys;

    #[test]
    fn status_text_formats_controls() {
        let text = pianokeys::status_text(0.4, Duration::from_millis(7000), 2);
        assert_eq!(text, "Volume: 0.40  Note: 7.00s  Octave: 2");
        // 0 means "until the sample ends"; show that honestly.
        let ring = pianokeys::status_text(0.4, Duration::from_millis(0), 2);
        assert_eq!(ring, "Volume: 0.40  Note: ring   Octave: 2");
    }

    #[test]
    fn status_text_width_is_constant() {
        let texts = [
            pianokeys::status_text(0.05, Duration::from_millis(50), 0),
            pianokeys::status_text(1.2, Duration::from_millis(8000), 6),
            pianokeys::status_text(0.4, Duration::from_millis(500), 2),
            pianokeys::status_text(0.4, Duration::from_millis(0), 2),
        ];
        assert!(texts.iter().all(|t| t.len() == texts[0].len()));
    }

    #[test]
    fn in_body_bounds() {
        // The paintable keyboard body spans relative columns 1 ..= 174
        // (x_offset + 1 .. x_offset + 174); relative column 0 is the left
        // border of the first white key, where the `a` key maps but no black
        // key is rendered.
        assert!(pianokeys::in_body(13, 2, 12)); // first white-key label cells
        assert!(pianokeys::in_body(12 + 174 - 1, 1, 12)); // last column (186)
        assert!(!pianokeys::in_body(187, 1, 12)); // just past the right edge
        assert!(pianokeys::in_body(184, 2, 12)); // 2 cells at 184..185
        assert!(!pianokeys::in_body(186, 2, 12)); // 2 cells at 186..187 -> off
        assert!(!pianokeys::in_body(12, 1, 12)); // relative col 0 (left border)
        assert!(!pianokeys::in_body(11, 1, 12)); // left of the body
        assert!(!pianokeys::in_body(-20, 2, 0)); // negative (Ctrl octave down)
    }

    #[test]
    fn black_height_keeps_white_black_ratio() {
        // The white:black ratio is 16:9 at full size and stays as close to it
        // as whole rows allow at every scaled-down step.
        assert_eq!(pianokeys::black_height(16), 9);
        assert_eq!(pianokeys::black_height(14), 8);
        assert_eq!(pianokeys::black_height(12), 7);
        assert_eq!(pianokeys::black_height(10), 6);
        assert_eq!(pianokeys::black_height(8), 5);
        assert_eq!(pianokeys::black_height(6), 3);
        assert_eq!(pianokeys::black_height(4), 2);
        assert_eq!(pianokeys::black_height(2), 1);
    }

    #[test]
    fn instrument_layout_scales_with_terminal_height() {
        // Tall terminal: full-size keyboard with pedal, status and hints.
        let full = pianokeys::instrument_layout(24);
        assert_eq!(full.white_height, 16);
        assert!(full.show_pedal && full.show_status && full.show_hints);
        assert_eq!(full.height, 21);

        // One step down keeps everything but shortens the keys.
        let mid = pianokeys::instrument_layout(20);
        assert_eq!(mid.white_height, 14);
        assert_eq!(mid.height, 19);

        // Below a 12-row keyboard the top-right hint panel goes away.
        let small = pianokeys::instrument_layout(16);
        assert_eq!(small.white_height, 10);
        assert!(!small.show_hints);
        assert!(small.show_pedal && small.show_status);

        // Very small: the pedal drops first, the status row survives.
        let tiny = pianokeys::instrument_layout(6);
        assert_eq!(tiny.white_height, 2);
        assert!(!tiny.show_pedal);
        assert!(tiny.show_status);
        assert_eq!(tiny.height, 4);

        // Minimum: keys + gap only.
        let min = pianokeys::instrument_layout(3);
        assert_eq!(min.white_height, 2);
        assert!(!min.show_pedal && !min.show_status);
        assert_eq!(min.height, 3);
    }
}

