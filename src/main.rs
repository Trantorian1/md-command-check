use std::io::BufRead;

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

    let mut out = std::io::BufWriter::new(std::io::stdout());

    while let Some(file_name) = files.next() {
        let file = std::fs::File::open(&file_name).map_err(Error::IoOpenError)?;
        let mut buff = std::io::BufReader::new(file);
        let mut line_number = 0;
        let mut line_number_code = 0;

        while buff.read_line(&mut line).map_err(Error::IoReadError)? != 0 {
            line_number += 1;

            // We've found a code block!
            if line.starts_with("```") {
                line_number_code = line_number;
                let shell = line[3..line.len() - 1].to_string();

                if shell.len() == 0 {
                    return Err(Error::MdNoShellError(line_number));
                }

                line.clear();

                while buff.read_line(&mut line).map_err(Error::IoReadError)? != 0 && !line.starts_with("```") {
                    line_number_code += 1;
                    cmd.push_str(&line);
                    line.clear();
                }

                if line != "```\n" {
                    return Err(Error::MdUncloseCodeBlockError(
                        line[..line.len() - 1].to_string(),
                        line_number,
                    ));
                }

                let mut process = std::process::Command::new(shell);
                let program_and_args = &cmd[..cmd.len() - 1];
                process.arg("-c").arg(program_and_args);

                let output = process
                    .output()
                    .map_err(|err| Error::ExecRunError(err, program_and_args.to_string(), line_number))?;

                if !output.status.success() {
                    return failure(
                        &mut out,
                        &file_name,
                        line_number,
                        program_and_args,
                        &output.stdout,
                        &output.stderr,
                    );
                } else {
                    success(&mut out, &file_name, line_number)?;
                }

                cmd.clear();
            }

            line.clear();
            line_number += line_number_code;
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

fn failure(
    out: &mut impl std::io::Write,
    file_name: &str,
    line_number: usize,
    program_and_args: &str,
    stdout: &[u8],
    stderr: &[u8],
) -> Result<(), Error> {
    write!(
        out,
        "\
            ❌{BOLD}{file_name}{RESET}: \
            code block at line {line_number} - \
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
