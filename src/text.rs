// use super::font::*;
// use glium::backend::{Context, Facade};
// use sdf::font::*;
// use std::collections::HashMap;
// use std::rc::Rc;

// pub struct GLText {
//     context: Rc<Context>,
//     text: String,
//     layouts: HashMap<String, GLTextBlockLayout>,
//     font_size: f32,
//     position: [f32; 2],
//     size: Option<[f32; 2]>,
//     intrisic_size: Option<[f32; 2]>,
// }

// #[derive(Debug)]
// struct WordIterator<'a> {
//     new_line: bool,
//     word_start: Option<usize>,
//     text: &'a str,
// }

// impl<'a> WordIterator<'a> {
//     fn new(text: &'a str) -> Self {
//         Self {
//             new_line: false,
//             word_start: None,
//             text,
//         }
//     }
// }

// enum WordIteratorResult<'a> {
//     NewLine,
//     Word(&'a str),
// }

// impl<'a> Iterator for WordIterator<'a> {
//     type Item = WordIteratorResult<'a>;

//     fn next(&mut self) -> Option<Self::Item> {
//         if self.new_line {
//             self.new_line = false;
//             return Some(WordIteratorResult::NewLine);
//         }

//         for (i, c) in self.text.chars().enumerate() {
//             if let Some(word_start) = self.word_start {
//                 if c.is_whitespace() {
//                     let word = &self.text[word_start..i];
//                     self.word_start = None;
//                     self.text = &self.text[i + 1..];
//                     self.new_line = c == '\n';
//                     return Some(WordIteratorResult::Word(word));
//                 }
//             } else {
//                 if !c.is_whitespace() {
//                     self.word_start = Some(i);
//                 }
//             }

//             if c == '\n' {
//                 self.text = &self.text[i + 1..];
//                 return Some(WordIteratorResult::NewLine);
//             }
//         }

//         if let Some(word_start) = self.word_start {
//             let word = &self.text[word_start..];
//             self.word_start = None;
//             self.text = &self.text[self.text.len()..];
//             return Some(WordIteratorResult::Word(word));
//         }

//         return None;
//     }
// }

// impl GLText {
//     pub fn new<F: ?Sized + Facade>(font_size: f32, facade: &F) -> Self {
//         Self {
//             context: facade.get_context().clone(),
//             text: String::new(),
//             layouts: HashMap::new(),
//             font_size,
//             position: [0.0, 0.0],
//             size: None,
//             intrisic_size: None,
//         }
//     }

//     pub fn set_text(&mut self, text: &str, font: &mut Font) {
//         if text == self.text {
//             return;
//         }
//         self.text = text.into();
//         self.parse(font);
//     }

//     pub fn parse(&mut self, font: &mut Font) {
//         let mut pos = [0.0, 0.0];

//         let space_width = self.font_size;
//         let line_height = self.font_size * 2.0;

//         for result in WordIterator::new(&self.text) {
//             match result {
//                 WordIteratorResult::Word(word) => {
//                     if let Some(layout) = self.layouts.get(word) {

//                     } else {
//                         // self.layouts.insert(
//                         //     word,
//                         //     GLTextBlockLayout::new(
//                         //         &self.context,
//                         //         text_block_layout: &TextBlockLayout,
//                         //     ),
//                         // )
//                     }
//                     println!("'{}'", word);
//                 }
//                 WordIteratorResult::NewLine => {
//                     pos[0] = 0.0;
//                     pos[1] += line_height;
//                 }
//             }
//         }
//     }
// }
