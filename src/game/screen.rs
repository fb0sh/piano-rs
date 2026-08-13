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
        Goto,
        PrintStyledFont,
        Result,
    };

    use std::io::{stdout, Stdout, Write};

    use super::super::notes;
    use super::Color;

    struct Point {
        x: u16,
        y: u16,
    }

    // Rows where the key labels are drawn. White keys are 16 rows tall with
    // note marks on the bottom row (15); black keys are 9 rows tall with
    // marks on row 8. Labels sit just above the mark rows.
    const WHITE_LABEL_ROW: u16 = 14;
    const BLACK_LABEL_BOTTOM_ROW: u16 = 7;

    // The white keys are 3 columns wide and there are 58 of them, so the
    // keyboard body is 175 columns wide. The sustain pedal hangs below it one
    // row after the 16-row-tall keys, centered like a real piano pedal and
    // sized to a fraction of the keyboard width.
    const KEYBOARD_WIDTH: u16 = 175;
    const PEDAL_ROW: u16 = 17;
    const PEDAL_HEIGHT: u16 = 3;
    const PEDAL_WIDTH: u16 = 30;
    const PEDAL_X: u16 = (KEYBOARD_WIDTH - PEDAL_WIDTH) / 2;

    pub fn draw(show_keys: bool, sequence: i8, offset: i8, x_offset: u16, y_offset: u16) -> Result<()> {
        let mut stdout = stdout();
        print_whites(&mut stdout, x_offset, y_offset)?;
        print_blacks(&mut stdout, x_offset, y_offset)?;
        if show_keys {
            draw_labels(sequence, offset, x_offset, y_offset, &mut stdout)?;
        }
        stdout.flush()?;
        Ok(())
    }

    /// Draws the keyboard letter on each playable key for the given frequency
    /// sequence and modifier offset (0 normally, -1 while Ctrl is held, +1
    /// while Shift is held). `x_offset`/`y_offset` shift the whole keyboard.
    pub fn show_labels(sequence: i8, offset: i8, x_offset: u16, y_offset: u16) -> Result<()> {
        let mut stdout = stdout();
        draw_labels(sequence, offset, x_offset, y_offset, &mut stdout)?;
        stdout.flush()?;
        Ok(())
    }

    /// Restores the key surfaces where labels were drawn, e.g. before redrawing
    /// them at a new sequence or modifier offset.
    pub fn hide_labels(sequence: i8, offset: i8, x_offset: u16, y_offset: u16) -> Result<()> {
        let mut stdout = stdout();
        for (_key, pos, white) in notes::key_labels() {
            let screen_pos = pos + 21 * (sequence as i16 + offset as i16) + x_offset as i16;
            if screen_pos < 0 {
                continue;
            }
            let screen_pos = screen_pos as u16;
            if white {
                queue!(
                    stdout,
                    Goto(screen_pos, WHITE_LABEL_ROW + y_offset),
                    PrintStyledFont("██".white())
                )?;
            } else {
                queue!(
                    stdout,
                    Goto(screen_pos, BLACK_LABEL_BOTTOM_ROW + y_offset),
                    PrintStyledFont("█".black())
                )?;
                queue!(
                    stdout,
                    Goto(screen_pos, BLACK_LABEL_BOTTOM_ROW + y_offset - 1),
                    PrintStyledFont("█".black())
                )?;
            }
        }
        stdout.flush()?;
        Ok(())
    }

    fn draw_labels(sequence: i8, offset: i8, x_offset: u16, y_offset: u16, stdout: &mut Stdout) -> Result<()> {
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
            if white {
                // White keys have a 2-column wide body, so up to two letters
                // fit side by side.
                for (i, c) in chars.iter().take(2).enumerate() {
                    queue!(
                        stdout,
                        Goto(pos + i as u16, WHITE_LABEL_ROW + y_offset),
                        PrintStyledFont(style(c.to_string()).black().on_white())
                    )?;
                }
            } else {
                // Black keys are 1 column wide; stack extra letters vertically.
                let mut row = BLACK_LABEL_BOTTOM_ROW + y_offset;
                for c in chars.iter().take(2) {
                    queue!(
                        stdout,
                        Goto(pos, row),
                        PrintStyledFont(style(c.to_string()).white().on_black())
                    )?;
                    row -= 1;
                }
            }
        }
        Ok(())
    }

    fn print_whitekey(initial_point: Point, stdout: &mut Stdout) -> Result<()> {
        let key_height: u16 = 16;

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

    fn print_whites(stdout: &mut Stdout, x_offset: u16, y_offset: u16) -> Result<()> {
        for key in 0..58 {
            let initial_point = Point { x: key * 3 + x_offset, y: y_offset };
            print_whitekey(initial_point, stdout)?;
        }
        Ok(())
    }

    fn print_blackkey(initial_point: Point, stdout: &mut Stdout) -> Result<()> {
        let key_height = 9;
        for column_height in 0..key_height {
            queue!(
                stdout,
                Goto(initial_point.x, initial_point.y + column_height),
                PrintStyledFont("█".black())
            )?;
        }
        Ok(())
    }

    fn print_blacks(stdout: &mut Stdout, x_offset: u16, y_offset: u16) -> Result<()> {
        // First black key is lonely
        let mut initial_point = Point { x: 3 + x_offset, y: y_offset };
        print_blackkey(initial_point, stdout)?;

        for x in 0..8 {
            let g1k1 = x * 21 + 9 + x_offset;
            let g1k2 = g1k1 + 3;
            initial_point = Point { x: g1k1, y: y_offset };
            print_blackkey(initial_point, stdout)?;
            initial_point = Point { x: g1k2, y: y_offset };
            print_blackkey(initial_point, stdout)?;

            let g2k1 = g1k2 + 6;
            let g2k2 = g2k1 + 3;
            let g2k3 = g2k2 + 3;
            initial_point = Point { x: g2k1, y: y_offset };
            print_blackkey(initial_point, stdout)?;
            initial_point = Point { x: g2k2, y: y_offset };
            print_blackkey(initial_point, stdout)?;
            initial_point = Point { x: g2k3, y: y_offset };
            print_blackkey(initial_point, stdout)?;
        }

        Ok(())
    }

    /// Draws the sustain pedal below the keyboard (the space bar toggles it).
    /// `on` lights the pedal up while sustain is active; with `show_keys` the
    /// SPACE label is drawn centered on the pedal.
    pub fn draw_pedal(on: bool, show_keys: bool, x_offset: u16, y_offset: u16) -> Result<()> {
        let mut stdout = stdout();
        let bar_color = if on { Color::Yellow } else { Color::DarkGrey };

        for row in 0..PEDAL_HEIGHT {
            for x in 0..PEDAL_WIDTH {
                queue!(
                    stdout,
                    Goto(PEDAL_X + x + x_offset, PEDAL_ROW + row + y_offset),
                    PrintStyledFont(style("█").with(bar_color))
                )?;
            }
        }

        if show_keys {
            let label = "SPACE";
            let start_x = PEDAL_X + x_offset + (PEDAL_WIDTH - label.len() as u16) / 2;
            let label_row = PEDAL_ROW + 1 + y_offset;
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
}

pub fn mark_note(pos: i16, white: bool, color: Color, duration: time::Duration, x_offset: u16, y_offset: u16) {
    if white {
        // This causes a compiler panic!
        /* queue!( */
        /*     stdout(), */
        /*     Goto(pos as u16, 15), */
        /*     PrintStyledFont(StyledObject("██").with(color)) */
        /* ).unwrap(); */

        queue!(
            stdout(),
            Goto(pos as u16 + x_offset, 15 + y_offset),
            PrintStyledFont(style("██").with(color))
        ).unwrap();

    /* println!("{} Red foreground text", Colored::Fg(Color::Red)); */
    } else {
        queue!(
            stdout(),
            Goto(pos as u16 + x_offset, 8 + y_offset),
            PrintStyledFont(style("█").with(color))
        ).unwrap();
    }

    thread::spawn(move || {
        thread::sleep(duration);
        if white {
        queue!(
            stdout(),
            Goto(pos as u16 + x_offset, 15 + y_offset),
            PrintStyledFont("██".white())
        ).unwrap();
        } else {
        queue!(
            stdout(),
            Goto(pos as u16 + x_offset, 8 + y_offset),
            PrintStyledFont("█".black())
        ).unwrap();
        }
    });
}

