use chrono::*;
use std::env;
use std::io::{Read, Write};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

#[derive(Copy, Clone)]
pub enum LogFrom {
    Main,
    Green,
    Yellow,
    Blue,
}

struct TextOutput {
    text: &'static str,
    color: Color,
    bold: bool,
}

fn writeln_color(output: Vec<TextOutput>) {
    let mut stdout = StandardStream::stdout(ColorChoice::Always);
    for text in output {
        stdout
            .set_color(
                ColorSpec::new()
                    .set_fg(Some(text.color))
                    .set_bold(text.bold),
            )
            .unwrap();
        stdout.write(text.text.as_bytes()).unwrap();
    }
    stdout.write("\n".as_bytes()).unwrap();
    stdout
        .set_color(ColorSpec::new().set_fg(None).set_bold(false))
        .unwrap();
}

type T = TextOutput;

impl TextOutput {
    fn new(text: &'static str, color: Color, bold: bool) -> Self {
        TextOutput { text, color, bold }
    }
}

pub fn startup_banner() -> Result<(), std::io::Error> {
    let mut stdout = StandardStream::stdout(ColorChoice::Always);

    let line_1: Vec<TextOutput> = vec![
        T::new("                 ┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓\n                 ┃ ", Color::White, false),
        T::new("Welcome to ", Color::White, true),
        T::new("KUpD", Color::Magenta, true),
        T::new(" v0.1.0-alpha.1 - a ", Color::White, false),
        T::new("League of Legends", Color::White, true),
        T::new(" monitor for ", Color::White, false),
        T::new("LAMB", Color::Yellow, true),
        T::new("          ┃", Color::White, false)
    ];

    writeln_color(line_1);

    let line_2: Vec<TextOutput> = vec![
        T::new("                 ┃ ", Color::White, false),
        T::new("KUpD", Color::Magenta, true),
        T::new(" is free software licensed under ", Color::White, false),
        T::new("GPLv3", Color::White, true),
        T::new(
            "                                     ┃",
            Color::White,
            false,
        ),
    ];

    writeln_color(line_2);

    let line_3: Vec<TextOutput> = vec![
        T::new(
            "                 ┃ Source code is available @ ",
            Color::White,
            false,
        ),
        T::new("https://github.com/Starkiller645/kupd/", Color::Cyan, true),
        T::new("              ┃", Color::White, false),
    ];

    writeln_color(line_3);

    let line_4: Vec<TextOutput> = vec![
        T::new("                 ┃ KUpD was created by ", Color::White, false),
        T::new("Tallie Tye", Color::Yellow, true),
        T::new(" @ ", Color::White, false),
        T::new("https://tallie.dev/", Color::Cyan, true),
        T::new("                           ┃\n                 ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛", Color::White, false),
    ];

    writeln_color(line_4);

    Ok(())
}

pub fn log_additional(text: &str) -> Result<(), std::io::Error> {
    let mut stdout = StandardStream::stdout(ColorChoice::Always);
    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Ansi256(242))))?;
    stdout.write(format!("                 -> {}\n", text).as_bytes())?;
    stdout.set_color(ColorSpec::new().set_fg(None))?;
    Ok(())
}

pub fn log(text: &str, from: Option<LogFrom>) -> Result<(), std::io::Error> {
    let current_time: DateTime<Local> = Local::now();
    let mut stdout = StandardStream::stdout(ColorChoice::Always);
    let from: LogFrom = match from {
        Some(log_from) => log_from,
        None => LogFrom::Main,
    };
    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Ansi256(242))))?;
    stdout.write(current_time.format("%H:%M:%S ").to_string().as_bytes())?;
    stdout.set_color(ColorSpec::new().set_bold(true))?;
    match from {
        LogFrom::Main => {
            stdout.write("   ".as_bytes())?;
            stdout
                .set_color(ColorSpec::new().set_fg(Some(Color::Magenta)).set_bold(true))
                .unwrap();
            stdout.write("MAIN".as_bytes())?;
            stdout.set_color(ColorSpec::new().set_fg(None).set_bold(true))?;
        }
        LogFrom::Green => {
            stdout.write("  ".as_bytes())?;
            stdout
                .set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true))
                .unwrap();
            stdout.write("GREEN".as_bytes())?;
            stdout.set_color(ColorSpec::new().set_fg(None).set_bold(true))?;
        }
        LogFrom::Yellow => {
            stdout.write(" ".as_bytes())?;
            stdout
                .set_color(ColorSpec::new().set_fg(Some(Color::Yellow)).set_bold(true))
                .unwrap();
            stdout.write("YELLOW".as_bytes())?;
            stdout.set_color(ColorSpec::new().set_fg(None).set_bold(true))?;
        }
        LogFrom::Blue => {
            stdout.write("   ".as_bytes())?;
            stdout
                .set_color(ColorSpec::new().set_fg(Some(Color::Blue)).set_bold(true))
                .unwrap();
            stdout.write("BLUE".as_bytes())?;
            stdout.set_color(ColorSpec::new().set_fg(None).set_bold(true))?;
        }
    }
    stdout.set_color(ColorSpec::new().set_bold(false))?;
    stdout.write(format!(" {}\n", text).as_bytes())?;
    Ok(())
}
