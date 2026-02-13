//! Terminal markdown rendering utilities.
//!
//! # Doc Audit
//! - ignore: internal implementation detail

use colored::Colorize;
use pulldown_cmark::{Event, Parser, Tag, TagEnd};

use super::icons as ui;

/// Re-export status_icon from icons module for backward compatibility
pub use super::icons::status_icon;

/// Renders markdown text to the terminal with ANSI formatting
pub fn render_markdown(markdown: &str) {
    let parser = Parser::new(markdown);
    let mut renderer = TerminalRenderer::new();

    for event in parser {
        renderer.handle_event(event);
    }

    renderer.flush();
}

struct TerminalRenderer {
    buffer: String,
    in_code_block: bool,
    in_italic: bool,
    in_bold: bool,
    heading_level: usize,
    in_table_cell: bool,
    table_cells: Vec<String>,
    list_depth: usize,
    ordered_list_depth: Vec<usize>,
}

impl TerminalRenderer {
    fn new() -> Self {
        Self {
            buffer: String::new(),
            in_code_block: false,
            in_italic: false,
            in_bold: false,
            heading_level: 0,
            in_table_cell: false,
            table_cells: Vec::new(),
            list_depth: 0,
            ordered_list_depth: Vec::new(),
        }
    }

    fn handle_event(&mut self, event: Event) {
        match event {
            Event::Start(tag) => self.handle_start_tag(tag),
            Event::End(tag_end) => self.handle_end_tag(tag_end),
            Event::Text(text) => self.buffer.push_str(&text),
            Event::Code(text) => {
                self.buffer.push('`');
                self.buffer.push_str(&text);
                self.buffer.push('`');
            }
            Event::SoftBreak | Event::HardBreak => {
                self.buffer.push('\n');
            }
            Event::Rule => {
                self.flush();
                println!("{}", "─".repeat(40).dimmed());
            }
            _ => {}
        }
    }

    fn handle_start_tag(&mut self, tag: Tag) {
        match tag {
            Tag::Heading { level, .. } => {
                self.flush();
                self.heading_level = match level {
                    pulldown_cmark::HeadingLevel::H1 => 1,
                    pulldown_cmark::HeadingLevel::H2 => 2,
                    pulldown_cmark::HeadingLevel::H3 => 3,
                    pulldown_cmark::HeadingLevel::H4 => 4,
                    pulldown_cmark::HeadingLevel::H5 => 5,
                    pulldown_cmark::HeadingLevel::H6 => 6,
                };
            }
            Tag::Paragraph => {
                // Paragraph start - no special handling needed
            }
            Tag::Emphasis => {
                self.in_italic = true;
            }
            Tag::Strong => {
                self.in_bold = true;
            }
            Tag::CodeBlock(_) => {
                self.flush();
                self.in_code_block = true;
                self.buffer.clear();
            }
            Tag::Link { .. } => {
                // Link text will be handled, URL will be appended on end
            }
            Tag::Image { .. } => {
                // Image alt text will be handled
            }
            Tag::List(ordered) => {
                self.flush();
                self.list_depth += 1;
                if let Some(start_num) = ordered {
                    self.ordered_list_depth.push(start_num as usize);
                } else {
                    self.ordered_list_depth.push(0);
                }
            }
            Tag::Item => {
                self.flush();
                let indent = "  ".repeat(self.list_depth.saturating_sub(1));
                if let Some(last) = self.ordered_list_depth.last_mut() {
                    if *last > 0 {
                        print!("{}{}. ", indent, last);
                        *last += 1;
                    } else {
                        print!("{}• ", indent);
                    }
                }
            }
            Tag::Table(_) => {
                self.flush();
            }
            Tag::TableHead => {
                // Table head - cells will be collected
            }
            Tag::TableRow => {
                // Row start
            }
            Tag::TableCell => {
                self.in_table_cell = true;
            }
            Tag::BlockQuote => {
                self.flush();
                print!("{}", "> ".dimmed());
            }
            _ => {}
        }
    }

    fn handle_end_tag(&mut self, tag_end: TagEnd) {
        match tag_end {
            TagEnd::Heading(_) => {
                let formatted = ui::colors::markdown_heading(&self.buffer, self.heading_level);
                println!("{}", formatted);
                self.buffer.clear();
                self.heading_level = 0;
                println!();
            }
            TagEnd::Paragraph => {
                if !self.buffer.is_empty() {
                    let output = if self.in_bold && self.in_italic {
                        self.buffer.bold().italic().to_string()
                    } else if self.in_bold {
                        self.buffer.bold().to_string()
                    } else if self.in_italic {
                        self.buffer.italic().to_string()
                    } else {
                        self.buffer.clone()
                    };
                    println!("{}", output);
                    self.buffer.clear();
                }
                println!();
            }
            TagEnd::Emphasis => {
                self.in_italic = false;
            }
            TagEnd::Strong => {
                self.in_bold = false;
            }
            TagEnd::CodeBlock => {
                self.in_code_block = false;
                if !self.buffer.is_empty() {
                    for line in self.buffer.lines() {
                        println!("{}", line.dimmed());
                    }
                    self.buffer.clear();
                }
                println!();
            }
            TagEnd::Link => {
                // Link end - text already in buffer
            }
            TagEnd::Image => {
                // Image alt text already handled
            }
            TagEnd::List(_) => {
                if self.list_depth > 0 {
                    self.list_depth -= 1;
                    self.ordered_list_depth.pop();
                }
                println!();
            }
            TagEnd::Item => {
                if !self.buffer.is_empty() {
                    println!("{}", self.buffer);
                    self.buffer.clear();
                }
            }
            TagEnd::Table => {
                println!();
            }
            TagEnd::TableHead => {
                // Separator after table head
            }
            TagEnd::TableRow => {
                self.table_cells.clear();
            }
            TagEnd::TableCell => {
                self.in_table_cell = false;
                self.table_cells.push(self.buffer.clone());
                self.buffer.clear();
            }
            _ => {}
        }
    }

    fn flush(&mut self) {
        if !self.buffer.is_empty() {
            let output = if self.in_bold && self.in_italic {
                self.buffer.bold().italic().to_string()
            } else if self.in_bold {
                self.buffer.bold().to_string()
            } else if self.in_italic {
                self.buffer.italic().to_string()
            } else {
                self.buffer.clone()
            };
            print!("{}", output);
            self.buffer.clear();
        }
    }
}
