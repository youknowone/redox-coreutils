#![deny(warnings)]

extern crate coreutils;
extern crate extra;
extern crate syscall;

use std::{env, fmt, fs};
use std::io::{stdout, stderr, Write};
use coreutils::ArgParser;
use extra::option::OptionalExt;

const MAN_PAGE: &'static str = /* @MANSTART{stat} */ r#"
NAME
    stat - display file status

SYNOPSIS
    stat [ -h | --help ] FILE...

DESCRIPTION
    Displays file status.

OPTIONS
    --help, -h
        print this message
"#; /* @MANEND */


struct Perms(u16);

impl fmt::Display for Perms {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "(0{:o}/", self.0 & 0o777)?;
        let perm = |i, c| {
            if self.0 & ((1 << i) as u16) != 0 {
                c
            } else {
                "-"
            }
        };
        write!(f, "{}{}{}", perm(8, "r"), perm(7, "w"), perm(6, "x"))?;
        write!(f, "{}{}{}", perm(5, "r"), perm(4, "w"), perm(3, "x"))?;
        write!(f, "{}{}{}", perm(2, "r"), perm(1, "w"), perm(0, "x"))?;
        write!(f, ")")?;
        Ok(())
    }
}

fn main() {
    let stdout = stdout();
    let mut stdout = stdout.lock();
    let mut stderr = stderr();
    let mut parser = ArgParser::new(1)
        .add_flag(&["h", "help"]);
    parser.parse(env::args());

    if parser.found("help") {
        stdout.write_all(MAN_PAGE.as_bytes()).try(&mut stderr);
        stdout.flush().try(&mut stderr);
        return;
    }

    for path in &parser.args[0..] {
        let mut st = syscall::Stat::default();
        let fd = syscall::open(path, syscall::O_CLOEXEC | syscall::O_STAT | syscall::O_NOFOLLOW).unwrap();
        syscall::fstat(fd, &mut st).unwrap();
        syscall::close(fd).unwrap();
        let file_type = match st.st_mode & syscall::MODE_TYPE {
            syscall::MODE_FILE => "regular file",
            syscall::MODE_DIR => "directory",
            syscall::MODE_SYMLINK => "symbolic link",
            _ => ""
        };
        if st.st_mode & syscall::MODE_SYMLINK == syscall::MODE_SYMLINK {
            println!("File: {} -> {}", path, fs::read_link(path).unwrap().display());
        } else {
            println!("File: {}", path);
        }
        println!("Size: {}  Blocks: {}  IO Block: {} {}", st.st_size, st.st_blocks, st.st_blksize, file_type);
        println!("Device: {}  Inode: {}  Links: {}", st.st_dev, st.st_ino, st.st_nlink);
        println!("Access: {}  Uid: {}  Gid: {}", Perms(st.st_mode), st.st_uid, st.st_gid);
        println!("Access: {}.{:09}", st.st_atime, st.st_atime_nsec);
        println!("Modify: {}.{:09}", st.st_mtime, st.st_mtime_nsec);
        println!("Change: {}.{:09}", st.st_ctime, st.st_ctime_nsec);
    }
}