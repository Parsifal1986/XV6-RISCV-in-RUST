// Shell.

#![no_std]
#![no_main]

extern crate alloc;

use alloc::boxed::Box;
use alloc::vec::Vec;
use user::*;

const MAXARGS: usize = 10;

// Parsed command representation
enum Cmd<'a> {
  Exec { argv: Vec<&'a [u8]> },
  Redir { cmd: Box<Cmd<'a>>, file: &'a [u8], mode: i32, fd: i32 },
  Pipe { left: Box<Cmd<'a>>, right: Box<Cmd<'a>> },
  List { left: Box<Cmd<'a>>, right: Box<Cmd<'a>> },
  Back { cmd: Box<Cmd<'a>> },
}

fn panic(s: &str) -> ! {
  fprintf!(2, "{}\n", s);
  exit(1);
}

// Fork but panics on failure.
fn fork1() -> i32 {
  let pid = fork();
  if pid == -1 {
    panic("fork");
  }
  pid
}

// Execute cmd.  Never returns.
fn runcmd(cmd: &Cmd) -> ! {
  match cmd {
    Cmd::Exec { argv } => {
      if argv.is_empty() {
        exit(1);
      }
      exec(argv[0], argv);
      fprintf!(2, "exec {} failed\n", as_str(argv[0]));
    }
    Cmd::Redir { cmd, file, mode, fd } => {
      close(*fd);
      if open(*file, *mode) < 0 {
        fprintf!(2, "open {} failed\n", as_str(file));
        exit(1);
      }
      runcmd(cmd);
    }
    Cmd::List { left, right } => {
      if fork1() == 0 {
        runcmd(left);
      }
      wait(None);
      runcmd(right);
    }
    Cmd::Pipe { left, right } => {
      let mut p = [0i32; 2];
      if pipe(&mut p) < 0 {
        panic("pipe");
      }
      if fork1() == 0 {
        close(1);
        dup(p[1]);
        close(p[0]);
        close(p[1]);
        runcmd(left);
      }
      if fork1() == 0 {
        close(0);
        dup(p[0]);
        close(p[0]);
        close(p[1]);
        runcmd(right);
      }
      close(p[0]);
      close(p[1]);
      wait(None);
      wait(None);
    }
    Cmd::Back { cmd } => {
      if fork1() == 0 {
        runcmd(cmd);
      }
    }
  }
  exit(0);
}

fn getcmd(buf: &mut [u8]) -> i32 {
  write(2, b"$ ");
  buf.fill(0);
  gets(buf);
  if buf[0] == 0 {
    // EOF
    return -1;
  }
  0
}

#[no_mangle]
fn main(_args: &[&str]) -> i32 {
  let mut buf = [0u8; 100];

  // Ensure that three file descriptors are open.
  loop {
    let fd = open("console", O_RDWR);
    if fd < 0 {
      break;
    }
    if fd >= 3 {
      close(fd);
      break;
    }
  }

  // Read and run input commands.
  while getcmd(&mut buf) >= 0 {
    let line = cstr(&buf);
    let mut i = 0;
    while i < line.len() && (line[i] == b' ' || line[i] == b'\t') {
      i += 1;
    }
    let cmd = &line[i..];
    if cmd.is_empty() || cmd[0] == b'\n' {
      // is a blank command
      continue;
    }
    if cmd.starts_with(b"cd ") {
      // Chdir must be called by the parent, not the child.
      let mut dir = &cmd[3..];
      if dir.last() == Some(&b'\n') {
        dir = &dir[..dir.len() - 1]; // chop \n
      }
      if chdir(dir) < 0 {
        fprintf!(2, "cannot cd {}\n", as_str(dir));
      }
    } else {
      if fork1() == 0 {
        runcmd(&parsecmd(cmd));
      }
      wait(None);
    }
  }
  0
}

// Parsing

const WHITESPACE: &[u8] = b" \t\r\n\x0b";
const SYMBOLS: &[u8] = b"<|>&;()";

struct Parser<'a> {
  s: &'a [u8],
  pos: usize,
}

impl<'a> Parser<'a> {
  fn cur(&self) -> u8 {
    if self.pos < self.s.len() {
      self.s[self.pos]
    } else {
      0
    }
  }

  fn skip_whitespace(&mut self) {
    while self.pos < self.s.len() && WHITESPACE.contains(&self.s[self.pos]) {
      self.pos += 1;
    }
  }

  // return the next token's type, and for words ('a'),
  // the word itself.
  fn gettoken(&mut self) -> (u8, &'a [u8]) {
    self.skip_whitespace();
    let start = self.pos;
    let mut ret = self.cur();
    match ret {
      0 => {}
      b'|' | b'(' | b')' | b';' | b'&' | b'<' => {
        self.pos += 1;
      }
      b'>' => {
        self.pos += 1;
        if self.cur() == b'>' {
          ret = b'+';
          self.pos += 1;
        }
      }
      _ => {
        ret = b'a';
        while self.pos < self.s.len() && !WHITESPACE.contains(&self.s[self.pos]) && !SYMBOLS.contains(&self.s[self.pos]) {
          self.pos += 1;
        }
      }
    }
    let tok = &self.s[start..self.pos];
    self.skip_whitespace();
    (ret, tok)
  }

  fn peek(&mut self, toks: &[u8]) -> bool {
    self.skip_whitespace();
    let c = self.cur();
    c != 0 && toks.contains(&c)
  }

  fn parseline(&mut self) -> Cmd<'a> {
    let mut cmd = self.parsepipe();
    while self.peek(b"&") {
      self.gettoken();
      cmd = Cmd::Back { cmd: Box::new(cmd) };
    }
    if self.peek(b";") {
      self.gettoken();
      cmd = Cmd::List { left: Box::new(cmd), right: Box::new(self.parseline()) };
    }
    cmd
  }

  fn parsepipe(&mut self) -> Cmd<'a> {
    let mut cmd = self.parseexec();
    if self.peek(b"|") {
      self.gettoken();
      cmd = Cmd::Pipe { left: Box::new(cmd), right: Box::new(self.parsepipe()) };
    }
    cmd
  }

  fn parseredirs(&mut self, mut cmd: Cmd<'a>) -> Cmd<'a> {
    while self.peek(b"<>") {
      let (tok, _) = self.gettoken();
      let (t, file) = self.gettoken();
      if t != b'a' {
        panic("missing file for redirection");
      }
      cmd = match tok {
        b'<' => Cmd::Redir { cmd: Box::new(cmd), file, mode: O_RDONLY, fd: 0 },
        b'>' => Cmd::Redir { cmd: Box::new(cmd), file, mode: O_WRONLY | O_CREATE | O_TRUNC, fd: 1 },
        _ => Cmd::Redir { cmd: Box::new(cmd), file, mode: O_WRONLY | O_CREATE, fd: 1 }, // >>
      };
    }
    cmd
  }

  fn parseblock(&mut self) -> Cmd<'a> {
    if !self.peek(b"(") {
      panic("parseblock");
    }
    self.gettoken();
    let cmd = self.parseline();
    if !self.peek(b")") {
      panic("syntax - missing )");
    }
    self.gettoken();
    self.parseredirs(cmd)
  }

  fn parseexec(&mut self) -> Cmd<'a> {
    if self.peek(b"(") {
      return self.parseblock();
    }

    // redirections apply to the command as a whole, so collect
    // the arguments and redirections separately, then wrap the
    // command in its redirections in the order they appeared.
    let mut argv: Vec<&'a [u8]> = Vec::new();
    let mut redirs: Vec<(u8, &'a [u8])> = Vec::new();

    self.collect_redirs(&mut redirs);
    while !self.peek(b"|)&;") {
      let (tok, word) = self.gettoken();
      if tok == 0 {
        break;
      }
      if tok != b'a' {
        panic("syntax");
      }
      argv.push(word);
      if argv.len() >= MAXARGS {
        panic("too many args");
      }
      self.collect_redirs(&mut redirs);
    }

    let mut cmd = Cmd::Exec { argv };
    for (tok, file) in redirs {
      cmd = match tok {
        b'<' => Cmd::Redir { cmd: Box::new(cmd), file, mode: O_RDONLY, fd: 0 },
        b'>' => Cmd::Redir { cmd: Box::new(cmd), file, mode: O_WRONLY | O_CREATE | O_TRUNC, fd: 1 },
        _ => Cmd::Redir { cmd: Box::new(cmd), file, mode: O_WRONLY | O_CREATE, fd: 1 }, // >>
      };
    }
    cmd
  }

  fn collect_redirs(&mut self, redirs: &mut Vec<(u8, &'a [u8])>) {
    while self.peek(b"<>") {
      let (tok, _) = self.gettoken();
      let (t, file) = self.gettoken();
      if t != b'a' {
        panic("missing file for redirection");
      }
      redirs.push((tok, file));
    }
  }
}

fn parsecmd(s: &[u8]) -> Cmd<'_> {
  let mut p = Parser { s, pos: 0 };
  let cmd = p.parseline();
  p.peek(b"");
  if p.pos != p.s.len() {
    fprintf!(2, "leftovers: {}\n", as_str(&p.s[p.pos..]));
    panic("syntax");
  }
  cmd
}
