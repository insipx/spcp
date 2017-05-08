#![feature(associated_consts)]
#![feature(concat_idents)]

#[macro_use]
mod macros;

pub type sample_t = i16;

pub mod SPC_DSP;
mod voice;
mod state;
mod sizes;
mod registers;
mod config;


