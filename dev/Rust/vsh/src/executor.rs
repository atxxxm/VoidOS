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
                stderr,
                err_append,
                stderr_to_stdout,
            } => {
                return Ok(self.exec_command(
                    program, args, stdin, stdout, *append,
                    stderr, *err_append, *stderr_to_stdout,
                )?);
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

    // Execute pipe. `Pipe` nodes nest right-heavy for chains like `a | b | c`
    // (built as Pipe{left: Pipe{a,b}, right: c}), so this flattens the tree
    // into an ordered list of stages first instead of assuming exactly two
    // -- a bare two-way match here is what previously made any pipe with
    // more than one `|` fail with "left side of pipe must be a command".
    fn exec_pipe(&self, left: &ExecNode, right: &ExecNode) -> anyhow::Result<i32> {
        let mut stages = Vec::new();
        self.flatten_pipe(left, &mut stages)?;
        self.flatten_pipe(right, &mut stages)?;

        let last = stages.len() - 1;
        let mut children = Vec::with_capacity(stages.len());
        let mut prev_stdout: Option<std::process::ChildStdout> = None;

        for (i, node) in stages.iter().enumerate() {
            let ExecNode::Command {
                program,
                args,
                stdin,
                stdout,
                append,
                stderr,
                err_append,
                stderr_to_stdout,
            } = node
            else {
                anyhow::bail!("pipe segments must be plain commands");
            };

            let prog = expand(program, self.last_exit);
            let prog_args: Vec<String> = args.iter().map(|a| expand(a, self.last_exit)).collect();
            let mut cmd = Command::new(&prog);
            cmd.args(&prog_args);

            // The previous stage's output takes priority over this stage's
            // own `<` redirect (which only really applies to the first
            // stage, but is honored wherever it's written).
            if let Some(out) = prev_stdout.take() {
                cmd.stdin(Stdio::from(out));
            } else if let Some(file) = stdin {
                let file = expand(file, self.last_exit);
                if let Ok(f) = File::open(&file) {
                    cmd.stdin(Stdio::from(f));
                }
            }

            if i == last {
                self.configure_stdout(&mut cmd, stdout, *append);
                self.configure_stderr(&mut cmd, stdout, *append, stderr, *err_append, *stderr_to_stdout);
            } else {
                cmd.stdout(Stdio::piped());
                // `2>&1` merging into a mid-pipeline stdout isn't supported
                // here (that stdout is the pipe, not a file) -- only a
                // plain `2>file` redirect is honored on non-last stages.
                self.configure_stderr(&mut cmd, &None, false, stderr, *err_append, false);
            }

            let mut child = cmd.spawn()?;
            prev_stdout = child.stdout.take();
            children.push(child);
        }

        let mut last_status = 1;
        for child in children.iter_mut() {
            let status = child.wait()?;
            last_status = status.code().unwrap_or(1);
        }
        Ok(last_status)
    }

    fn flatten_pipe<'a>(&self, node: &'a ExecNode, out: &mut Vec<&'a ExecNode>) -> anyhow::Result<()> {
        match node {
            ExecNode::Pipe { left, right } => {
                self.flatten_pipe(left, out)?;
                self.flatten_pipe(right, out)?;
            }
            ExecNode::Command { .. } => out.push(node),
            _ => anyhow::bail!("pipe segments must be plain commands"),
        }
        Ok(())
    }

    // Execute command
    fn exec_command(
        &self,
        program: &str,
        args: &[String],
        stdin: &Option<String>,
        stdout: &Option<String>,
        append: bool,
        stderr: &Option<String>,
        err_append: bool,
        stderr_to_stdout: bool,
    ) -> anyhow::Result<i32> {
        let program = expand(program, self.last_exit);
        let args: Vec<String> = args.iter().map(|a| expand(a, self.last_exit)).collect();
        let stdin: Option<String> = stdin.as_ref().map(|s| expand(s, self.last_exit));
        let stdout: Option<String> = stdout.as_ref().map(|s| expand(s, self.last_exit));
        let stderr: Option<String> = stderr.as_ref().map(|s| expand(s, self.last_exit));

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

        self.configure_stdout(&mut cmd, &stdout, append);
        self.configure_stderr(&mut cmd, &stdout, append, &stderr, err_append, stderr_to_stdout);

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

    fn configure_stdout(&self, cmd: &mut Command, stdout: &Option<String>, append: bool) {
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
    }

    // stderr redirect: 2>file / 2>>file / 2>&1
    fn configure_stderr(
        &self,
        cmd: &mut Command,
        stdout: &Option<String>,
        append: bool,
        stderr: &Option<String>,
        err_append: bool,
        stderr_to_stdout: bool,
    ) {
        if stderr_to_stdout {
            // Redirect stderr to the same destination as stdout; if stdout
            // isn't redirected to a file, stderr naturally goes to the
            // terminal same as stdout does.
            if let Some(out_file) = stdout {
                let f = if append {
                    OpenOptions::new().append(true).create(true).open(out_file)
                } else {
                    OpenOptions::new().write(true).create(true).truncate(true).open(out_file)
                };
                if let Ok(f) = f {
                    cmd.stderr(Stdio::from(f));
                }
            }
        } else if let Some(err_file) = stderr {
            let f = if err_append {
                OpenOptions::new().append(true).create(true).open(err_file)
            } else {
                OpenOptions::new().write(true).create(true).truncate(true).open(err_file)
            };
            if let Ok(f) = f {
                cmd.stderr(Stdio::from(f));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::{TokenParse, Tokenize};

    fn parse(input: &str) -> ExecNode {
        let tokens = Tokenize::new(input).tokenize();
        TokenParse::new(tokens).parse()
    }

    #[test]
    fn flattens_a_three_stage_pipe_in_left_to_right_order() {
        // Regression test: exec_pipe used to assume a Pipe node's `left`
        // side was always a bare Command, so `a | b | c` (parsed as
        // Pipe{left: Pipe{a,b}, right: c}) failed with "left side of pipe
        // must be a command" instead of running all three stages.
        let exec = Executor::new("", 0);
        let root = parse("a | b | c");
        let ExecNode::Pipe { left, right } = &root else {
            panic!("expected a Pipe node");
        };

        let mut stages = Vec::new();
        exec.flatten_pipe(left, &mut stages).unwrap();
        exec.flatten_pipe(right, &mut stages).unwrap();

        let programs: Vec<&str> = stages
            .iter()
            .map(|node| match node {
                ExecNode::Command { program, .. } => program.as_str(),
                _ => panic!("expected a Command node"),
            })
            .collect();
        assert_eq!(programs, vec!["a", "b", "c"]);
    }
}
