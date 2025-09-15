use std::io::Write;

use crate::colors::*;

#[derive(Clone, Copy)]
pub enum Status {
    RUNNING,
    PASS,
    FAIL,
    NEWFILE,
}

pub fn erase(out: &mut impl std::io::Write, line_count: usize) -> std::io::Result<()> {
    write!(out, "\x1b[{line_count}F\x1b[0J")
}

pub fn draw_file_info(
    out: &mut impl std::io::Write,
    status: Status,
    file_name: &str,
    line_number: usize,
) -> std::io::Result<usize> {
    match status {
        Status::RUNNING => {
            writeln!(
                out,
                "\
                    {YELLOW}╭[    {RESET}{BOLD}{file_name}{RESET}: \
                    code block at line {line_number} - \
                    {YELLOW}{ITALIC}RUNNING{RESET}\
                "
            )
        }
        Status::PASS => {
            writeln!(
                out,
                "\
                    {GREEN}╭[ ✅ {RESET}{BOLD}{file_name}{RESET}: \
                    code block at line {line_number} - \
                    {GREEN}PASS{RESET}\
                "
            )
        }
        Status::FAIL => {
            writeln!(
                out,
                "\
                    {RED}╭[ ❌ {RESET}{BOLD}{file_name}{RESET}: \
                    code block at line {line_number} - \
                    {RED}FAIL{RESET}\
                "
            )
        }
        Status::NEWFILE => {
            writeln!(
                out,
                "\
                    {PURPLE}╭[ 📁 {RESET}{BOLD}{file_name}{RESET}: \
                    code block at line {line_number} - \
                    {PURPLE}NEW FILE{RESET}\
                "
            )
        }
    }?;

    Ok(1)
}

pub fn draw_code(
    out: &mut impl std::io::Write,
    status: Status,
    lang: &str,
    program_and_args: &str,
    terminate: bool,
) -> std::io::Result<usize> {
    let accent = accent(status);
    let program_and_args = program_and_args
        .trim()
        .to_string()
        .replace("\n", &format!("\n{accent}│{RESET} "));
    if !terminate {
        write!(
            out,
            "\
                {accent}│{RESET} ```{ITALIC}{lang}{RESET}\n\
                {accent}│{RESET} {program_and_args}\n\
                {accent}│{RESET} ```\n\
            ",
        )?;
    } else {
        write!(
            out,
            "\
                {accent}│{RESET} ```{ITALIC}{lang}{RESET}\n\
                {accent}│{RESET} {program_and_args}\n\
                {accent}╰{RESET} ```\n\
            ",
        )?;
    }
    Ok(count_lines(&program_and_args) + 3)
}

pub fn draw_output(
    out: &mut impl std::io::Write,
    status: Status,
    cmd_out: &str,
    output: &str,
    terminate: bool,
) -> std::io::Result<usize> {
    let accent = accent(status);
    let cmd_out = cmd_out.trim().replace("\n", &format!("\n{accent}│{RESET} >> "));
    if !terminate {
        if !cmd_out.is_empty() {
            write!(
                out,
                "\
                    {accent}│{RESET} >> {BOLD}{ITALIC}{output}{RESET}\n\
                    {accent}│{RESET} >>\n\
                    {accent}│{RESET} >> {cmd_out}\n\
                    {accent}│{RESET} >>\n\
                ",
            )?;
            Ok(count_lines(&cmd_out) + 4)
        } else {
            write!(
                out,
                "\
                    {accent}│{RESET} >> {BOLD}{ITALIC}{output}{RESET}\n\
                    {accent}│{RESET} >> ...\n\
                    {accent}│{RESET} >>\n\
                "
            )?;
            Ok(count_lines(&cmd_out) + 3)
        }
    } else {
        if !cmd_out.is_empty() {
            write!(
                out,
                "\
                    {accent}│{RESET} >> {BOLD}{ITALIC}{output}{RESET}\n\
                    {accent}│{RESET} >>\n\
                    {accent}│{RESET} >> {cmd_out}\n\
                    {accent}╰{RESET} >>\n\
                ",
            )?;
            Ok(count_lines(&cmd_out) + 4)
        } else {
            write!(
                out,
                "\
                    {accent}│{RESET} >> {BOLD}{ITALIC}{output}{RESET}\n\
                    {accent}│{RESET} >> ...\n\
                    {accent}╰{RESET} >>\n\
                ",
            )?;
            Ok(count_lines(&cmd_out) + 3)
        }
    }
}

pub fn accent(status: Status) -> &'static str {
    match status {
        Status::RUNNING => YELLOW,
        Status::PASS => GREEN,
        Status::FAIL => RED,
        Status::NEWFILE => PURPLE,
    }
}

pub fn count_lines(mut data: &str) -> usize {
    let mut count = 0;

    while let Some(n) = data.find('\n') {
        data = &data[n + 1..];
        count += 1;
    }

    count
}

pub fn flush(out: &mut Vec<u8>) -> std::io::Result<()> {
    let mut stdout = std::io::stdout();
    stdout.write_all(out)?;
    out.clear();
    stdout.flush()
}
