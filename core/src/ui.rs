use term::{
    color::{self, Color},
    Attr, StdoutTerminal,
};
use unicode_width::UnicodeWidthChar;

use crate::api::Api;
use crate::plugins::{LocalizationItem, Rationale, RationaleChunk};

const NEWLINE: &'static str = "\n";

struct TextWrapper {
    col: usize,
    wrap_at: usize,
}

impl TextWrapper {
    pub fn new(wrap_at: usize) -> Self {
        TextWrapper { col: 0, wrap_at }
    }

    pub fn fill<S: Into<String>>(&mut self, text: S) -> String {
        let mut output = String::new();

        let string = text.into();
        let mut last_space = 0;
        for (_, (byte_pos, ch)) in string.char_indices().enumerate() {
            self.col += ch.width().unwrap();

            if self.col > self.wrap_at {
                self.col = self.col % self.wrap_at;
                last_space += 1;
                output.push_str(NEWLINE);
            }

            if ch == ' ' {
                output.push_str(&string[last_space..byte_pos]);
                last_space = byte_pos;
            } else if ch == '\n' {
                self.col = 0;
            }
        }

        if last_space < string.len() {
            output.push_str(&string[last_space..]);
        }

        output
    }

    pub fn reset(&mut self) {
        self.col = 0;
    }
}

pub struct CliUi<'data> {
    api: &'data Api<'data>,
    terminal: Box<StdoutTerminal>,
    wrapper: TextWrapper,
    current_color: Option<Color>,
    hypothesis: usize,
}

impl<'data> CliUi<'data> {
    pub fn new(api: &'data Api<'data>) -> Option<Self> {
        Some(CliUi {
            api,
            terminal: term::stdout()?,
            wrapper: TextWrapper::new(80),
            current_color: None,
            hypothesis: 1,
        })
    }

    pub fn plugin(&mut self, id: &str) {
        let bar = self.construct_bar(id.len() + 20, '-');

        self.newline();
        self.newline();
        self.writeln(format!("{}+", bar));
        self.write("   Localization: ");
        self.bold();
        self.write(id);
        self.reset_style();
        self.writeln("   |");
        self.writeln(format!("{}+", bar));
        self.newline();

        self.hypothesis = 1;
    }

    pub fn result(&mut self, item: &LocalizationItem<'data>) {
        let bar = self.construct_bar(20, '_');

        self.writeln(bar.clone());
        self.newline();

        self.color(color::CYAN);
        self.write(">>> Hypothesis ");
        self.bold();
        self.write(format!("#{}", self.hypothesis));
        self.reset();
        self.newline();
        self.newline();

        self.write("at ");
        self.bold();
        self.write(item.loc.to_string(self.api));
        self.reset_style();
        self.write("\twith suspiciousness ");
        self.bold();
        self.write(format!("{}", (item.score * 100.0).round() / 100.0));
        self.reset_style();
        self.newline();

        self.newline();
        self.rationale(&item.rationale);

        self.writeln(bar);
        self.newline();

        self.hypothesis += 1;
    }

    fn rationale(&mut self, rationale: &Rationale) {
        let mut anchors = Vec::new();

        self.writeln("Rationale:");
        self.newline();
        self.color(color::YELLOW);

        for chunk in rationale.chunks() {
            match chunk {
                RationaleChunk::Text(text) => {
                    let wrapped = self.wrapper.fill(text);
                    self.write(wrapped);
                }
                RationaleChunk::Anchor(anchor) => {
                    anchors.push(anchor);
                    let reference = format!("[{}]", anchors.len());

                    self.bold();
                    let wrapped = self.wrapper.fill(reference);
                    self.write(wrapped);
                    self.reset_style();
                }
            }
        }

        self.wrapper.reset();
        self.reset_color();

        if !anchors.is_empty() {
            self.newline();
            self.newline();
        }

        for (index, anchor) in anchors.into_iter().enumerate() {
            self.bold();
            self.write(format!("[{}]", index + 1));
            self.reset_style();

            self.write(" --> ");

            self.bold();
            self.write(anchor.to_string(self.api));
            self.reset_style();
        }

        self.newline();
    }

    fn write<S: Into<String>>(&mut self, text: S) {
        write!(self.terminal, "{}", text.into()).unwrap();
    }

    fn writeln<S: Into<String>>(&mut self, text: S) {
        write!(self.terminal, "{}{}", text.into(), NEWLINE).unwrap();
    }

    fn newline(&mut self) {
        self.write(NEWLINE);
    }

    fn color(&mut self, color: Color) {
        self.terminal.fg(color).unwrap();
        self.current_color = Some(color);
    }

    fn bold(&mut self) {
        self.terminal.attr(Attr::Bold).unwrap();
    }

    fn reset(&mut self) {
        self.reset_color();
        self.reset_style();
    }

    fn reset_color(&mut self) {
        self.terminal.reset().unwrap();
        self.current_color = None;
    }

    fn reset_style(&mut self) {
        self.terminal.reset().unwrap();
        if let Some(color) = self.current_color {
            self.terminal.fg(color).unwrap();
        }
    }

    fn construct_bar(&self, len: usize, ch: char) -> String {
        (0..len).fold(String::with_capacity(len), |mut bar, _| {
            bar.push(ch);
            bar
        })
    }
}
