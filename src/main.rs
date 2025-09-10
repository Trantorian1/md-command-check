use std::io::{BufRead, Write};

enum Error {
    IoOpenError(std::io::Error),
    IoReadError(std::io::Error),
    IoWriteError(std::io::Error),
    MdUncloseCodeBlockError(usize),
    ExecNoProgram(usize),
    ExecRunError(std::io::Error, String, usize),
    ExecFailure(String, usize),
}

const RED: &str = "\x1b[0;31m";
const GREEN: &str = "\x1b[0;32m";
const BOLD: &str = "\x1b[1m";
const FAINT: &str = "\x1b[2;37m";
const RESET: &str = "\x1b[m";

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IoOpenError(err) => write!(f, "Failed to open file: {err:?}"),
            Self::IoReadError(err) => write!(f, "Failed to read file: {err:?}"),
            Self::IoWriteError(err) => write!(f, "Faield to write to outpout: {err:?}"),
            Self::MdUncloseCodeBlockError(ln) => {
                write!(f, "Unclosed code block starting at line {ln}")
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
                line.clear();

                while buff.read_line(&mut line).map_err(Error::IoReadError)? != 0 && !line.starts_with("```") {
                    line_number_code += 1;
                    cmd.push_str(&line);
                    line.clear();
                }

                if !line.starts_with("```") {
                    return Err(Error::MdUncloseCodeBlockError(line_number));
                }

                let mut args = cmd.split_whitespace();
                let program = args.next().ok_or(Error::ExecNoProgram(line_number))?;

                let mut process = std::process::Command::new(program);
                process.args(args);

                let output = process
                    .output()
                    .map_err(|err| Error::ExecRunError(err, program_and_args(&mut process), line_number))?;

                if !output.status.success() {
                    let program_and_args = program_and_args(&mut process);
                    return failure(&mut out, &file_name, line_number, &program_and_args);
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

fn program_and_args(process: &mut std::process::Command) -> String {
    let program = process.get_program();
    let mut program_and_args = process.get_args().fold(String::new(), |mut acc, arg| {
        acc.push(' ');
        acc.push_str(&arg.to_string_lossy());
        acc
    });

    program_and_args.insert_str(0, &program.to_string_lossy());
    program_and_args
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
        "
    )
    .map_err(Error::IoWriteError)
}
