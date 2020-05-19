use term::{
    color::{self, Color},
    Attr, StdoutTerminal,
};
use unicode_width::UnicodeWidthChar;

use super::Ui;
use crate::api::Api;
use crate::data::statement::Loc;
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
            self.col += ch.width().unwrap_or(0);

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

struct DedupVec<T>(Vec<T>);

impl<T> DedupVec<T> {
    pub fn new() -> Self {
        DedupVec(Vec::new())
    }

    pub fn into_inner(self) -> Vec<T> {
        self.0
    }
}

impl<T: PartialEq> DedupVec<T> {
    pub fn push(&mut self, item: T) -> usize {
        match self
            .0
            .iter()
            .enumerate()
            .find(|(_, present)| *present == &item)
        {
            Some((index, _)) => index,
            None => {
                self.0.push(item);
                self.0.len() - 1
            }
        }
    }
}

pub struct CliUi {
    terminal: Box<StdoutTerminal>,
    wrapper: TextWrapper,
    current_color: Option<Color>,
    hypothesis: usize,
}

impl CliUi {
    pub fn new() -> Option<Self> {
        Some(CliUi {
            terminal: term::stdout()?,
            wrapper: TextWrapper::new(80),
            current_color: None,
            hypothesis: 1,
        })
    }

    fn rationale(&mut self, rationale: &Rationale, api: &Api) {
        let mut anchors = DedupVec::new();

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
                    let reference = format!("[{}]", anchors.push(anchor) + 1);

                    self.bold();
                    let wrapped = self.wrapper.fill(reference);
                    self.write(wrapped);
                    self.reset_style();
                }
            }
        }

        self.wrapper.reset();
        self.reset_color();

        let anchors = anchors.into_inner();

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
            self.write_loc(anchor, api);
            self.reset_style();
            self.newline();
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

    fn write_loc(&mut self, loc: &Loc, api: &Api) {
        self.write(api.file(&loc.file_id).unwrap().to_str().unwrap());
        self.write(":");

        if loc.line_begin == loc.line_end && loc.col_begin == loc.col_end {
            self.write(&format!("{}:{}", loc.line_begin, loc.col_begin));
        } else {
            self.write(&format!(
                "{}:{}-{}:{}",
                loc.line_begin, loc.col_begin, loc.line_end, loc.col_end
            ));
        }
    }
}

impl Ui for CliUi {
    fn plugin(&mut self, id: &str, _api: &Api) {
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

    fn result(&mut self, item: &LocalizationItem, api: &Api) {
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
        self.write_loc(&item.loc, api);
        self.reset_style();
        self.write("\twith suspiciousness ");
        self.bold();
        self.write(format!("{}", (item.score * 100.0).round() / 100.0));
        self.reset_style();
        self.newline();

        self.newline();
        self.rationale(&item.rationale, api);

        self.writeln(bar);
        self.newline();

        self.hypothesis += 1;
    }

    fn error(&mut self, error: &str) {
        self.color(color::RED);
        self.writeln("An error occured!");
        self.newline();
        self.color(color::YELLOW);
        self.writeln(error);
    }
}
