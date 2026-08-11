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

    struct Point {
        x: u16,
        y: u16,
    }

    // Rows where the key labels are drawn. White keys are 16 rows tall with
    // note marks on the bottom row (15); black keys are 9 rows tall with
    // marks on row 8. Labels sit just above the mark rows.
    const WHITE_LABEL_ROW: u16 = 14;
    const BLACK_LABEL_BOTTOM_ROW: u16 = 7;

    pub fn draw(show_keys: bool, sequence: i8) -> Result<()> {
        let mut stdout = stdout();
        print_whites(&mut stdout)?;
        print_blacks(&mut stdout)?;
        if show_keys {
            draw_labels(sequence, &mut stdout)?;
        }
        stdout.flush()?;
        Ok(())
    }

    /// Draws the keyboard letter on each playable key for the given frequency
    /// sequence.
    pub fn show_labels(sequence: i8) -> Result<()> {
        let mut stdout = stdout();
        draw_labels(sequence, &mut stdout)?;
        stdout.flush()?;
        Ok(())
    }

    /// Restores the key surfaces where labels were drawn, e.g. before redrawing
    /// them at a new sequence.
    pub fn hide_labels(sequence: i8) -> Result<()> {
        let mut stdout = stdout();
        for (_key, pos, white) in notes::key_labels() {
            let screen_pos = (pos + 21 * (sequence as i16)) as u16;
            if white {
                queue!(
                    stdout,
                    Goto(screen_pos, WHITE_LABEL_ROW),
                    PrintStyledFont("██".white())
                )?;
            } else {
                queue!(
                    stdout,
                    Goto(screen_pos, BLACK_LABEL_BOTTOM_ROW),
                    PrintStyledFont("█".black())
                )?;
                queue!(
                    stdout,
                    Goto(screen_pos, BLACK_LABEL_BOTTOM_ROW - 1),
                    PrintStyledFont("█".black())
                )?;
            }
        }
        stdout.flush()?;
        Ok(())
    }

    fn draw_labels(sequence: i8, stdout: &mut Stdout) -> Result<()> {
        // Group the labels by their on-screen position, since some keys map to
        // the same note (e.g. `,` and `q` are both A on the white key at 22).
        let mut groups: Vec<(u16, Vec<char>, bool)> = Vec::new();
        for (key, pos, white) in notes::key_labels() {
            let screen_pos = (pos + 21 * (sequence as i16)) as u16;
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
                        Goto(pos + i as u16, WHITE_LABEL_ROW),
                        PrintStyledFont(style(c.to_string()).black().on_white())
                    )?;
                }
            } else {
                // Black keys are 1 column wide; stack extra letters vertically.
                let mut row = BLACK_LABEL_BOTTOM_ROW;
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

    fn print_whites(stdout: &mut Stdout) -> Result<()> {
        for key in 0..58 {
            let initial_point = Point { x: key * 3, y: 0 };
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

    fn print_blacks(stdout: &mut Stdout) -> Result<()> {
        // First black key is lonely
        let mut initial_point = Point { x: 3, y: 0 };
        print_blackkey(initial_point, stdout)?;

        for x in 0..8 {
            let g1k1 = x * 21 + 9;
            let g1k2 = g1k1 + 3;
            initial_point = Point { x: g1k1, y: 0 };
            print_blackkey(initial_point, stdout)?;
            initial_point = Point { x: g1k2, y: 0 };
            print_blackkey(initial_point, stdout)?;

            let g2k1 = g1k2 + 6;
            let g2k2 = g2k1 + 3;
            let g2k3 = g2k2 + 3;
            initial_point = Point { x: g2k1, y: 0 };
            print_blackkey(initial_point, stdout)?;
            initial_point = Point { x: g2k2, y: 0 };
            print_blackkey(initial_point, stdout)?;
            initial_point = Point { x: g2k3, y: 0 };
            print_blackkey(initial_point, stdout)?;
        }

        Ok(())
    }
}

pub fn mark_note(pos: i16, white: bool, color: Color, duration: time::Duration) {
    if white {
        // This causes a compiler panic!
        /* queue!( */
        /*     stdout(), */
        /*     Goto(pos as u16, 15), */
        /*     PrintStyledFont(StyledObject("██").with(color)) */
        /* ).unwrap(); */

        queue!(
            stdout(),
            Goto(pos as u16, 15),
            PrintStyledFont(style("██").with(color))
        ).unwrap();

    /* println!("{} Red foreground text", Colored::Fg(Color::Red)); */
    } else {
        queue!(
            stdout(),
            Goto(pos as u16, 8),
            PrintStyledFont(style("█").with(color))
        ).unwrap();
    }

    thread::spawn(move || {
        thread::sleep(duration);
        if white {
        queue!(
            stdout(),
            Goto(pos as u16, 15),
            PrintStyledFont("██".white())
        ).unwrap();
        } else {
        queue!(
            stdout(),
            Goto(pos as u16, 8),
            PrintStyledFont("█".black())
        ).unwrap();
        }
    });
}

