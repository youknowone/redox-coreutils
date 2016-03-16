#![deny(warnings)]

extern crate coreutils;

use std::env;
use std::fs::File;
use std::io::{self, stdout, stderr, Read, Write, Stderr};
use std::iter;
use std::ops::{Add, AddAssign};
use std::process::exit;


use coreutils::extra::{OptionalExt, WriteExt};

static MAN_PAGE: &'static str = r#"
    NAME
        wc - count words, bytes and lines of a file or byte stream.
    SYNOPSIS
        wc [-h | --help] [-c | --bytes] [-w | --words] [-l | --lines] [FILE 1] [FILE 2]...
    DESCRIPTION
        This utility will dump word count, line count, and byte count of a file or byte stream. If multiple files are given, it will print a total count. If no file name is given, 'wc' reads from stdin until EOF.

        Flags can be arbitrarily combined into a single flag, for example, '-c -w' (count bytes and count words) can be reduced to '-cw'.

        If no flags are given, 'wc' defaults to '-cwl'.
    OPTIONS
        -h
        --help
            Print this manual page.
        -c
        --bytes
            Count bytes.
        -w
        --words
            Count words (i.e. contiguous strings seperated by whitespaces).
        -l
        --lines
            Count lines (seperated by NL).
    AUTHOR
        This program was written mainly by Alice Maz.
"#;

#[derive(Copy, Clone, Default)]
struct Counter {
    lines: u64,
    words: u64,
    bytes: u64,
}

impl Counter {
    #[inline]
    fn new<T: Read>(input: T) -> Self {
        let mut line_count = 0;
        let mut word_count = 0;
        let mut byte_count = 0;
        let mut got_space = true;

        for byte_result in input.bytes() {
            if let Ok(byte) = byte_result {
                if byte == b'\n' {
                    line_count += 1;
                }

                if is_whitespace(byte) {
                    got_space = true;
                } else if got_space {
                    got_space = false;
                    word_count += 1;
                }

                byte_count += 1;
            }
        }

        Counter { lines: line_count, words: word_count, bytes: byte_count }
    }
}

impl Add for Counter {
    type Output = Counter;

    fn add(self, other: Self) -> Self {
        Counter {
            lines: self.lines + other.lines,
            words: self.words + other.words,
            bytes: self.bytes + other.bytes
        }
    }
}

impl AddAssign for Counter {
    fn add_assign(&mut self, _rhs: Self) {
        self.lines += _rhs.lines;
        self.words += _rhs.words;
        self.bytes += _rhs.bytes;
    }
}

#[derive(Copy, Clone, Default)]
struct Flags {
    count_lines: bool,
    count_words: bool,
    count_bytes: bool,
}

impl Flags {
    fn print_counts<W: Write>(self, mut counts: Vec<(Counter, String)>, stdout: &mut W, stderr: &mut Stderr) {
        use std::cmp::max;

        let mut max_lines_digits = 0;
        let mut max_words_digits = 0;
        let mut max_bytes_digits = 0;

        for &mut (count, _) in &mut counts {
            if self.count_lines {
                max_lines_digits = max(max_lines_digits, count.lines.to_string().len());
            }
            if self.count_words {
                max_words_digits = max(max_words_digits, count.words.to_string().len());
            }
            if self.count_bytes {
                max_bytes_digits = max(max_bytes_digits, count.bytes.to_string().len());
            }
        }

        fn print_val<W: Write>(val: u64, max_digits: usize, stdout: &mut W, stderr: &mut Stderr) {
            let count_str = val.to_string();
            stdout.write(count_str.as_bytes()).try(stderr);
            for _ in 0..(max_digits - count_str.len() + 1) {
                stdout.write(b" ").try(stderr);
            }
        }

        for &mut (count, ref mut path) in &mut counts {
            stdout.write(b"    ").try(stderr);

            if self.count_lines {
                print_val(count.lines, max_lines_digits, stdout, stderr);
            }
            if self.count_words {
                print_val(count.words, max_words_digits, stdout, stderr);
            }
            if self.count_bytes {
                print_val(count.bytes, max_bytes_digits, stdout, stderr);
            }

            stdout.writeln(path.as_bytes()).try(&mut *stderr);
        }
    }

    fn default_to(&mut self) {
        // Defaults to behavior of -lwc
        if !(self.count_lines || self.count_words || self.count_bytes) {
            self.count_lines = true;
            self.count_words = true;
            self.count_bytes = true;
        }
    }
}

fn main() {
    let mut opts = Flags::default();
    let mut first_file = String::new();
    let mut args = env::args().skip(1);
    let stdout = stdout();
    let mut stdout = stdout.lock();
    let mut stderr = stderr();

    loop { // To avoid consumption of the iter, we use loop.
        let arg = if let Some(x) = args.next() { x } else { break };

        // TODO add -m, maybe add -L

        if arg.starts_with("-") {
            match arg.as_str() {
                "--lines" | "-l" => opts.count_lines = true,
                "--words" | "-w" => opts.count_words = true,
                "--bytes" | "-c" => opts.count_bytes = true,
                "--help"  | "-h" => {
                    stdout.writeln(MAN_PAGE.as_bytes()).try(&mut stderr);
                    return;
                },
                _ => {
                    stderr.write(b"error: unknown flag, ").try(&mut stderr);
                    stderr.write(arg.as_bytes()).try(&mut stderr);
                    stderr.flush().try(&mut stderr);
                    exit(1);
                },
            }
        } else { // This is a file name
            first_file = arg;
            break;
        }
    }

    opts.default_to();

    if first_file == "" {
        let stdin = io::stdin();
        let stdin = stdin.lock();

        opts.print_counts(vec![(Counter::new(stdin), "stdin".to_owned())], &mut stdout, &mut stderr);
    } else {
        let mut files = Vec::<(Counter, String)>::new();
        let mut total_count = Counter::default();
        let single_file = args.len() == 0;

        for path in iter::once(first_file).chain(args) {
            //TODO would be easy to use stdin for - but
            //that is probably something the shell should handle?
            //unix it's all just fds so it's whatever dunno here tho
            //(also - is specific to sh/bash fwiw).

            let file = File::open(&path).try(&mut stderr);
            let file_count = Counter::new(file);
            total_count += file_count;

            files.push((file_count, path.to_owned()));
        }

        if !single_file {
            files.push((total_count, "total".to_owned()));
        }

        opts.print_counts(files, &mut stdout, &mut stderr);
    }
}

fn is_whitespace(byte: u8) -> bool {
    //FIXME this works like iswspace w/ default C locale
    //but not good enough for en_US.UTF8 among others

    byte == b'\n' // newline
    || byte == b'\t' // tab
    || byte == b'\r' // cr
    || byte == 0xc // formfeed
    || byte == 0xb // vtab
    || byte == b' ' // space
}
