
use std::{fs::{File, OpenOptions}, io::Write, process::{Command, Stdio}};

use crate::utils::{ExecNode, Token, TokenParse, Tokenize, get_current_path};
use anyhow;

pub struct Executor {
    prompt: String,
    tokens: Vec<Token>,
}

impl Executor {
    pub fn new(prompt: &str) -> Self {
        Self { prompt: prompt.to_string(), tokens: Vec::new() }
    }

    // Run command
    pub fn run(&mut self) -> anyhow::Result<()> {
        self.cmd_to_token();

        let parser = TokenParse::new(self.tokens.clone());
        let root = parser.parse();

        self.execute(&root)?;

        Ok(())
    }

    // Tokenize command
    pub fn cmd_to_token(&mut self) {
        self.tokens = Tokenize::new(&self.prompt).tokenize();
    }

    // AST //

    // Execute AST
    pub fn execute(&self, node: &ExecNode) -> anyhow::Result<i32> {
        match node {
            ExecNode::Command { program, args, stdin, stdout, append } => {
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
        let mut left_cmd = match left {
            ExecNode::Command { program, args, ..} => {
                let mut c = Command::new(program);
                c.args(args);
                c.stdout(Stdio::piped());
                c.spawn()?
            }

            _ => anyhow::bail!("Left of pipe must be a command"),
        };

        let left_out = left_cmd.stdout.take().unwrap();

        let mut right_cmd = match right {
            ExecNode::Command { program, args, ..} => {
                let mut c = Command::new(program);
                c.args(args);
                c.stdin(left_out);
                c.spawn()?
            }

            _ => anyhow::bail!("Right of pipe must be a command"),
        };

        left_cmd.wait()?;
        let status =  right_cmd.wait()?;
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

        if program == "cd" || program == "pwd" || program == "echo" || program == "clear" {
            return (Ok(self.exec_builtin(program, args, stdout, append)));
        }

        let mut cmd = Command::new(program);
        cmd.args(args);

        // stdin <
        if let Some(file) = stdin {
            if let Ok(f) = File::open(file) {
                cmd.stdin(Stdio::from(f));
            }
        }

        // stdout > / >>
        if let Some(file) = stdout {
            let f = if append {
                OpenOptions::new().append(true).create(true).open(file)
            } else {
                OpenOptions::new().write(true).create(true).truncate(true).open(file)
            };

            if let Ok(f) = f {
                cmd.stdout(Stdio::from(f));
            }
        }

        let status = cmd.status();

        Ok(status.map(|s| s.code().unwrap_or(1)).unwrap_or(1))
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
                OpenOptions::new().write(true).create(true).truncate(true).open(file)
            }.expect("cannot open file for redirect");

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
            _ => {}
        }

        0
    }
    
}

