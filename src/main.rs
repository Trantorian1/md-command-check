use std::{io::BufRead, ops::Deref};

enum Error {
    IoOpenError(std::io::Error),
    IoReadError(std::io::Error),
    IoWriteError(std::io::Error),
    MdUncloseCodeBlockError(String, usize),
    MdNoShellError(usize),
    ExecNoProgram(usize),
    ExecRunError(std::io::Error, String, usize),
    ExecFailure(String, usize),
}

const RED: &str = "\x1b[0;31m";
const GREEN: &str = "\x1b[0;32m";
const BOLD: &str = "\x1b[1m";
const FAINT: &str = "\x1b[2m";
const ITALIC: &str = "\x1b[3m";
const RESET: &str = "\x1b[m";

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IoOpenError(err) => write!(f, "Failed to open file: {err:?}"),
            Self::IoReadError(err) => write!(f, "Failed to read file: {err:?}"),
            Self::IoWriteError(err) => write!(f, "Faield to write to outpout: {err:?}"),
            Self::MdUncloseCodeBlockError(line, ln) => {
                write!(
                    f,
                    "Invalid closing delimiter for code block starting at line {ln}: {line}"
                )
            }
            Self::MdNoShellError(ln) => {
                write!(f, "Missing shell name for code block starting at line {ln}")
            }
            Self::ExecNoProgram(ln) => {
                write!(f, "No program specified in code block starting at line {ln}")
            }
            Self::ExecRunError(err, program_and_args, ln) => {
                write!(
                    f,
                    "Failed to run code block\n```\n{program_and_args}\n```\nFrom code block at line {ln}: {err:?}"
                )
            }
            Self::ExecFailure(program_and_args, ln) => {
                write!(
                    f,
                    "Failed to run code block\n```\n{program_and_args}\n```\nFrom code block at line {ln}"
                )
            }
        }
    }
}

fn main() -> Result<(), Error> {
    let mut files = std::env::args().skip(1);
    let mut line = String::with_capacity(256);
    let mut cmd = line.clone();

    let mut vars = std::collections::HashMap::<String, String>::new();

    let mut out = std::io::BufWriter::new(std::io::stdout());

    while let Some(file_name) = files.next() {
        let path = std::path::PathBuf::from(&file_name);
        if !path.extension().is_some_and(|ext| ext == "md") {
            err_file_ext(&mut out, &file_name);
            return Ok(());
        }

        let Ok(file) = std::fs::File::open(path) else {
            err_file_open(&mut out, &file_name);
            return Ok(());
        };

        let mut buff = std::io::BufReader::new(file);
        let mut line_number = 0;
        let mut line_number_code = 0;

        let mut var_local = Vec::with_capacity(8);

        while buff.read_line(&mut line).map_err(Error::IoReadError)? != 0 {
            line_number += 1;

            // We've found a comment!
            if line.starts_with("<!--") {
                let mut words = line.trim().split_whitespace().skip(1);
                if let Some("extract") = words.next() {
                    let Some(var) = words.next() else {
                        err_extract_no_var(&mut out, &file_name, line_number);
                        return Ok(());
                    };

                    let mut pat = String::with_capacity((line.len() - 4 - var.len()).saturating_sub(4));
                    while let Some(word) = words.next() {
                        if word == "-->" {
                            break;
                        }
                        if !pat.is_empty() {
                            pat.push(' ');
                        }
                        pat.push_str(word);
                    }

                    let pat = pat.trim_matches('"');
                    let Ok(re) = regex::Regex::new(pat) else {
                        err_extract_pattern(&mut out, &file_name, line_number, pat);
                        return Ok(());
                    };

                    var_local.push((var.to_string(), re));
                }
            }
            // We've found a code block!
            else if line.starts_with("```") {
                line_number_code = line_number;
                let shell = line[3..line.len() - 1].to_string();

                if shell.len() == 0 {
                    err_no_shell(&mut out, &file_name, line_number);
                    return Ok(());
                }

                line.clear();

                while buff.read_line(&mut line).map_err(Error::IoReadError)? != 0 && !line.starts_with("```") {
                    line_number_code += 1;
                    cmd.push_str(&line);
                    line.clear();
                }

                if line != "```\n" {
                    let delimiter = &line[..line.len().saturating_sub(1)];
                    err_block_close(&mut out, &file_name, line_number, delimiter);
                    return Ok(());
                }

                if shell != "bash" && shell != "sh" {
                    line.clear();
                    cmd.clear();
                    line_number += line_number_code;
                    continue;
                }

                let mut process = std::process::Command::new(shell);
                let mut program_and_args = cmd[..cmd.len() - 1].to_string();
                for (var, val) in vars.iter() {
                    program_and_args = program_and_args.replace(var, val.as_ref());
                }

                process
                    .arg("-c")
                    .arg(&program_and_args)
                    .stdin(std::process::Stdio::null());

                let Ok(output) = process.output() else {
                    err_cmd_spawn(&mut out, &file_name, line_number, &program_and_args);
                    return Ok(());
                };

                if !output.status.success() {
                    return err_cmd_failure(
                        &mut out,
                        &file_name,
                        line_number,
                        &program_and_args,
                        &output.stdout,
                        &output.stderr,
                    );
                }

                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                for (mut var, re) in var_local {
                    let Some(cap) = re
                        .captures(&stdout)
                        .or_else(|| re.captures(&stderr))
                        .and_then(|cap| cap.get(1))
                        .map(|cap| cap.as_str().to_string())
                    else {
                        return err_cmd_capture(
                            &mut out,
                            &file_name,
                            line_number,
                            &program_and_args,
                            &stdout,
                            &stderr,
                            &re,
                        );
                    };

                    var.insert(0, '<');
                    var.push('>');

                    vars.insert(var, cap);
                }

                var_local = Vec::with_capacity(8);

                success(&mut out, &file_name, line_number)?;

                cmd.clear();
                line_number += line_number_code;
            }

            line.clear();
        }
    }

    Ok(())
}

fn success(out: &mut impl std::io::Write, file_name: &str, line_number: usize) -> Result<(), Error> {
    write!(
        out,
        "\
            ✅{BOLD}{file_name}{RESET}: \
            code block at line {line_number} - \
            {GREEN}PASS{RESET}\n\
        "
    )
    .map_err(Error::IoWriteError)
}

fn err(out: &mut impl std::io::Write, file_name: &str) {
    let _ = write!(out, "❌{BOLD}{file_name}{RESET}: ",);
}

fn err_line_comment(out: &mut impl std::io::Write, file_name: &str, line_number: usize) {
    err(out, file_name);
    let _ = write!(out, "comment at line {line_number} - ",);
}

fn err_line_directive(out: &mut impl std::io::Write, file_name: &str, line_number: usize, directive: &str) {
    err(out, file_name);
    let _ = write!(out, "{directive} directive at line {line_number} - ",);
}

fn err_line_code(out: &mut impl std::io::Write, file_name: &str, line_number: usize) {
    err(out, file_name);
    let _ = write!(out, "code block at line {line_number} - ",);
}

fn err_file_ext(out: &mut impl std::io::Write, file_name: &str) {
    err(out, file_name);
    let _ = write!(out, "{RED}File is not a .md{RESET}\n");
}

fn err_file_open(out: &mut impl std::io::Write, file_name: &str) {
    err(out, file_name);
    let _ = write!(out, "{RED}Failed to open file{RESET}\n");
}

fn err_comment_close(out: &mut impl std::io::Write, file_name: &str, line_number: usize, delimiter: &str) {
    err_line_comment(out, file_name, line_number);
    if !delimiter.is_empty() {
        let _ = write!(out, "{RED}Invalid closing delimiter:{RESET} {delimiter}\n");
    } else {
        let _ = write!(out, "{RED}Unclosed comment{RESET}\n");
    }
}

fn err_extract_no_var(out: &mut impl std::io::Write, file_name: &str, line_number: usize) {
    err_line_directive(out, file_name, line_number, "extract");
    let _ = write!(out, "{RED}No variable name{RESET}\n");
}

fn err_extract_pattern(out: &mut impl std::io::Write, file_name: &str, line_number: usize, pat: &str) {
    err_line_directive(out, file_name, line_number, "extract");
    let _ = write!(out, "{RED}Invalid pattern:{RESET}{pat}\n");
}

fn err_no_shell(out: &mut impl std::io::Write, file_name: &str, line_number: usize) {
    err_line_code(out, file_name, line_number);
    let _ = write!(out, "{RED}Missing shell type{RESET}\n");
}

fn err_block_close(out: &mut impl std::io::Write, file_name: &str, line_number: usize, delimiter: &str) {
    err_line_code(out, file_name, line_number);
    if !delimiter.is_empty() {
        let _ = write!(out, "{RED}Invalid closing delimiter:{RESET} {delimiter}\n");
    } else {
        let _ = write!(out, "{RED}Unclosed block{RESET}\n");
    }
}

fn err_cmd_spawn(out: &mut impl std::io::Write, file_name: &str, line_number: usize, program_and_args: &str) {
    err_line_code(out, file_name, line_number);
    let _ = write!(
        out,
        "\
            {RED}Could not run command{RESET}\n\
            {FAINT}\
            ```\n\
            {program_and_args}\n\
            ```\n\
            {RESET}\
        ",
    );
}

fn err_cmd_failure(
    out: &mut impl std::io::Write,
    file_name: &str,
    line_number: usize,
    program_and_args: &str,
    stdout: &[u8],
    stderr: &[u8],
) -> Result<(), Error> {
    err_line_code(out, file_name, line_number);
    write!(
        out,
        "\
            {RED}FAIL{RESET}\n\
            {FAINT}\
            ```\n\
            {program_and_args}\n\
            ```\n\
            {RESET}\
        ",
    )
    .map_err(Error::IoWriteError)?;

    if !stdout.is_empty() {
        let mut stdout = String::from_utf8_lossy(&stdout[..stdout.len() - 1]).replace("\n", "\n>> ");
        stdout.insert_str(0, ">> ");
        write!(
            out,
            "\
            >> {BOLD}{ITALIC}stdout{RESET}\n\
            >>\n\
            {stdout}\n\
            >>\n\
        ",
        )
        .map_err(Error::IoWriteError)?;
    }

    if !stderr.is_empty() {
        let mut stderr = String::from_utf8_lossy(&stderr[..stderr.len() - 1]).replace("\n", "\n>> ");
        stderr.insert_str(0, ">> ");
        write!(
            out,
            "\
            >> {BOLD}{ITALIC}stderr{RESET}\n\
            >>\n\
            {stderr}\n\
            >>\n\
        ",
        )
        .map_err(Error::IoWriteError)?;
    }

    Ok(())
}

fn err_cmd_capture(
    out: &mut impl std::io::Write,
    file_name: &str,
    line_number: usize,
    program_and_args: &str,
    stdout: &str,
    stderr: &str,
    re: &regex::Regex,
) -> Result<(), Error> {
    err_line_code(out, file_name, line_number);
    write!(
        out,
        "\
            {RED}Failed to capture matches:{RESET} {ITALIC}\"{re}\"{RESET}\n\
            {FAINT}\
            ```\n\
            {program_and_args}\n\
            ```\n\
            {RESET}\
        ",
    )
    .map_err(Error::IoWriteError)?;

    if !stdout.is_empty() {
        let mut stdout = stdout.replace("\n", "\n>> ");
        stdout.insert_str(0, ">> ");
        write!(
            out,
            "\
            >> {BOLD}{ITALIC}stdout{RESET}\n\
            >>\n\
            {stdout}\n\
            >>\n\
        ",
        )
        .map_err(Error::IoWriteError)?;
    }

    if !stderr.is_empty() {
        let mut stderr = stderr.replace("\n", "\n>> ");
        stderr.insert_str(0, ">> ");
        write!(
            out,
            "\
            >> {BOLD}{ITALIC}stderr{RESET}\n\
            >>\n\
            {stderr}\n\
            >>\n\
        ",
        )
        .map_err(Error::IoWriteError)?;
    }

    return Ok(());
}
