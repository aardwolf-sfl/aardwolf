//! Command line interface UI. Currently the only actual UI in Aardwolf.

use std::fs::File;
use std::io::{prelude::*, BufReader, SeekFrom};

use term::{
    color::{self, Color},
    Attr, StdoutTerminal,
};
use unicode_width::UnicodeWidthChar;

use super::Ui;
use crate::api::Api;
use crate::data::{statement::Loc, types::FileId};
use crate::plugins::{LocalizationItem, Metadata, Rationale, RationaleChunk};

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

    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.0.get_mut(index)
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

struct SourceSnippet {
    file: FileId,
    locs: Vec<(usize, Loc)>,
}

fn lineno_width(lineno: u32) -> usize {
    ((lineno as f32).log10() as usize) + 1
}

fn loc_width(loc: &Loc) -> usize {
    std::cmp::max(lineno_width(loc.line_begin), lineno_width(loc.line_end))
}

impl SourceSnippet {
    fn new(file: FileId) -> Self {
        SourceSnippet {
            file,
            locs: Vec::new(),
        }
    }

    fn push(&mut self, anchor: usize, loc: Loc) {
        self.locs.push((anchor, loc));
        self.locs
            .sort_by_key(|loc| (loc.1.line_begin, loc.1.col_begin));
    }

    pub fn write(&self, ui: &mut CliUi, api: &Api) {
        debug_assert!(!self.locs.is_empty());
        let loc = self.locs[0].1;

        let mut file = File::open(api.full_file(&loc.file_id).unwrap()).unwrap();
        let gutter_width = 1 + self
            .locs
            .iter()
            .map(|item| loc_width(&item.1))
            .max()
            .unwrap();

        ui.write(self.indent(gutter_width - 1));
        ui.write("--> ");
        ui.bold();
        ui.write(api.file(&loc.file_id).unwrap().to_str().unwrap());
        ui.reset_style();
        ui.newline();

        for (anchor, loc) in self.locs.iter() {
            file.seek(SeekFrom::Start(0)).unwrap();
            if loc.line_begin == loc.line_end {
                ui.writeln(self.construct_gutter(gutter_width));
                ui.write(self.indent(gutter_width - 1 - lineno_width(loc.line_begin)));
                ui.write(format!("{} | ", loc.line_begin));

                let (prefix, lines, postfix) = self.read_snippet(&mut file, loc);
                ui.write(prefix);
                ui.write(&lines[0]);
                ui.writeln(postfix);

                ui.write(self.construct_gutter(gutter_width));
                ui.write(self.indent(loc.col_begin as usize));

                let underline = (loc.col_end - loc.col_begin) as usize;
                let underline = if underline == 0 { 1 } else { underline };
                ui.color(color::YELLOW);
                ui.write(ui.construct_bar(underline, '^'));
                ui.write(format!(" [{}]", anchor));
                ui.reset_color();
            } else {
                // TODO:
                //        +--
                // prefix |multiline
                //         snippet| postfix
                //              --+
            }

            ui.newline();
        }

        ui.newline();
    }

    fn indent(&self, len: usize) -> String {
        (0..len).fold(String::with_capacity(len), |mut indent, _| {
            indent.push(' ');
            indent
        })
    }

    fn construct_gutter(&self, len: usize) -> String {
        let mut spaces = self.indent(len);
        spaces.push('|');
        spaces
    }

    fn read_snippet<R: Read>(&self, file: R, loc: &Loc) -> (String, Vec<String>, String) {
        let file = BufReader::new(file);
        let mut iter = file.lines().enumerate();

        let mut prefix = String::new();
        let mut lines = Vec::new();
        let mut postfix = String::new();

        while let Some((lineno, line)) = iter.next() {
            let line = line.unwrap();

            if lineno + 1 == loc.line_begin as usize {
                let line_chars = &mut line.chars();
                prefix = line_chars.take(loc.col_begin as usize).collect::<String>();

                if loc.line_begin == loc.line_end {
                    lines.push(line_chars.take(loc.col_end as usize).collect::<String>());
                    postfix = line_chars.collect::<String>();
                    return (prefix, lines, postfix);
                } else {
                    lines.push(line_chars.collect::<String>());
                }

                break;
            }
        }

        while let Some((lineno, line)) = iter.next() {
            let line = line.unwrap();

            if lineno + 1 == loc.line_end as usize {
                let line_chars = &mut line.chars();

                lines.push(line_chars.take(loc.col_end as usize).collect::<String>());
                postfix = line_chars.collect::<String>();
                return (prefix, lines, postfix);
            } else {
                lines.push(line);
            }
        }

        (prefix, lines, postfix)
    }
}

impl PartialEq for SourceSnippet {
    fn eq(&self, other: &Self) -> bool {
        self.file.eq(&other.file)
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

        let mut snippets = DedupVec::new();

        for (index, loc) in anchors.into_iter().enumerate() {
            let snippet = snippets.push(SourceSnippet::new(loc.file_id));
            snippets.get_mut(snippet).unwrap().push(index + 1, *loc);
        }

        let snippets = snippets.into_inner();

        for snippet in snippets {
            snippet.write(self, api);
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

    fn metadata(&mut self, metadata: &Metadata, _api: &Api) {
        if !metadata.is_empty() {
            let bar = self.construct_bar(14, '-');

            self.newline();
            self.newline();
            self.writeln(format!("{}+", bar));
            self.write("   Metadata");
            self.writeln("   |");
            self.writeln(format!("{}+", bar));
            self.newline();

            for item in metadata.iter() {
                self.write(item);
            }
        }
    }

    fn error(&mut self, error: &str) {
        self.color(color::RED);
        self.writeln("An error occured!");
        self.newline();
        self.color(color::YELLOW);
        self.writeln(error);
        self.reset();
    }
}
