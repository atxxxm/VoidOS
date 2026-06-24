use std::{
    fs::{File, OpenOptions},
    io::Write,
    process::{Command, Stdio},
};

use crate::utils::{ExecNode, Token, TokenParse, Tokenize, expand, get_current_path};
use anyhow;

pub struct Executor {
    prompt: String,
    tokens: Vec<Token>,
    last_exit: i32,
}

impl Executor {
    pub fn new(prompt: &str, last_exit: i32) -> Self {
        Self {
            prompt: prompt.to_string(),
            tokens: Vec::new(),
            last_exit,
        }
    }

    // Run command
    pub fn run(&mut self) -> anyhow::Result<i32> {
        self.cmd_to_token();
        let parser = TokenParse::new(self.tokens.clone());
        let root = parser.parse();
        self.execute(&root)
    }

    // Tokenize command
    pub fn cmd_to_token(&mut self) {
        self.tokens = Tokenize::new(&self.prompt).tokenize();
    }

    // AST //

    // Execute AST
    pub fn execute(&self, node: &ExecNode) -> anyhow::Result<i32> {
        match node {
            ExecNode::Command {
                program,
                args,
                stdin,
                stdout,
                append,
            } => {
                return Ok(self.exec_command(program, args, stdin, stdout, *append)?);
            }
            ExecNode::Pipe { left, right } => {
                return self.exec_pipe(left, right);
            }
            ExecNode::LogicalAnd { left, right } => {
                let code = self.execute(left)?;
                if code == 0 {
                    return Ok(self.execute(right)?);
                } else {
                    return Ok(code);
                }
            }
            ExecNode::LogicalOr { left, right } => {
                let code = self.execute(left)?;
                if code != 0 {
                    return Ok(self.execute(right)?);
                } else {
                    return Ok(code);
                }
            }
            ExecNode::Sequence { left, right } => {
                self.execute(left)?;
                return Ok(self.execute(right)?);
            }
        }
    }

    // Execute pipe
    fn exec_pipe(&self, left: &ExecNode, right: &ExecNode) -> anyhow::Result<i32> {
        let mut left_child = match left {
            ExecNode::Command { program, args, .. } => {
                let prog = expand(program, self.last_exit);
                let args: Vec<String> = args.iter().map(|a| expand(a, self.last_exit)).collect();
                let mut c = Command::new(&prog);
                c.args(&args);
                c.stdout(Stdio::piped());
                c.spawn()?
            }
            _ => anyhow::bail!("left side of pipe must be a command"),
        };

        let left_out = left_child.stdout.take().unwrap();

        let mut right_child = match right {
            ExecNode::Command { program, args, .. } => {
                let prog = expand(program, self.last_exit);
                let args: Vec<String> = args.iter().map(|a| expand(a, self.last_exit)).collect();
                let mut c = Command::new(&prog);
                c.args(&args);
                c.stdin(left_out);
                c.spawn()?
            }
            _ => anyhow::bail!("right side of pipe must be a command"),
        };

        left_child.wait()?;
        let status = right_child.wait()?;
        Ok(status.code().unwrap_or(1))
    }

    // Execute command
    fn exec_command(
        &self,
        program: &str,
        args: &[String],
        stdin: &Option<String>,
        stdout: &Option<String>,
        append: bool,
    ) -> anyhow::Result<i32> {
        let program = expand(program, self.last_exit);
        let args: Vec<String> = args.iter().map(|a| expand(a, self.last_exit)).collect();
        let stdin: Option<String> = stdin.as_ref().map(|s| expand(s, self.last_exit));
        let stdout: Option<String> = stdout.as_ref().map(|s| expand(s, self.last_exit));

        // Variable assignment: NAME=value (no command after it)
        if args.is_empty() {
            if let Some(eq) = program.find('=') {
                let name = &program[..eq];
                if !name.is_empty()
                    && !name.chars().next().unwrap().is_ascii_digit()
                    && name.chars().all(|c| c.is_alphanumeric() || c == '_')
                {
                    unsafe { std::env::set_var(name, &program[eq + 1..]); }
                    return Ok(0);
                }
            }
        }

        if program == "cd" || program == "pwd" || program == "echo" || program == "clear" || program == "exit" {
            return Ok(self.exec_builtin(&program, &args, &stdout, append));
        }

        let mut cmd = Command::new(&program);
        cmd.args(&args);

        if let Some(file) = &stdin {
            if let Ok(f) = File::open(file) {
                cmd.stdin(Stdio::from(f));
            }
        }

        if let Some(file) = &stdout {
            let f = if append {
                OpenOptions::new().append(true).create(true).open(file)
            } else {
                OpenOptions::new().write(true).create(true).truncate(true).open(file)
            };
            if let Ok(f) = f {
                cmd.stdout(Stdio::from(f));
            }
        }

        match cmd.status() {
            Ok(s) => Ok(s.code().unwrap_or(1)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                eprintln!("vsh: {program}: command not found");
                Ok(127)
            }
            Err(e) => {
                eprintln!("vsh: {program}: {e}");
                Ok(1)
            }
        }
    }

    // Exec builtin commands
    fn exec_builtin(
        &self,
        program: &str,
        args: &[String],
        stdout: &Option<String>,
        append: bool,
    ) -> i32 {
        let mut ouput: Box<dyn Write> = if let Some(file) = stdout {
            let f = if append {
                OpenOptions::new().append(true).create(true).open(file)
            } else {
                OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .open(file)
            }
            .expect("cannot open file for redirect");

            Box::new(f)
        } else {
            Box::new(std::io::stdout())
        };

        match program {
            "echo" => {
                writeln!(ouput, "{}", args.join(" ")).unwrap();
            }
            "pwd" => {
                let cur_path = get_current_path();
                writeln!(ouput, "{}", cur_path).unwrap();
            }
            "cd" => {
                let path = args.get(0).cloned().unwrap_or("/".into());
                if let Err(e) = std::env::set_current_dir(&path) {
                    eprintln!("cd error: {e}");
                    return 1;
                }
            }
            "clear" => {
                writeln!(ouput, "\x1B[2J\x1B[H").unwrap();
            }
            "exit" => {
                let code = args.first().and_then(|a| a.parse::<i32>().ok()).unwrap_or(0);
                std::process::exit(code);
            }
            _ => {}
        }

        0
    }
}
