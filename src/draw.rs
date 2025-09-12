use std::io::Write;

use crate::colors::*;

#[derive(Clone, Copy)]
pub enum Status {
    Running,
    PASS,
    FAIL,
}

pub fn draw(
    out: &mut Vec<u8>,
    shell: &mut std::process::Child,
    file_name: &str,
    line_number: usize,
    lang: &str,
    program_and_args: &str,
    cmd_stdout: &mut impl std::io::BufRead,
    cmd_stderr: &mut impl std::io::BufRead,
    debug: bool,
) -> std::io::Result<(String, String)> {
    let mut line_count = 0;

    let mut stdout = String::with_capacity(256);
    let mut stderr = String::with_capacity(256);
    let code;

    write!(out, "{WRAP_DISABLE}")?;

    line_count += draw_file_info(out, Status::Running, file_name, line_number)?;
    line_count += draw_code(out, Status::Running, lang, program_and_args, false)?;
    line_count += draw_output(out, Status::Running, &stdout, "stdout", true)?;
    flush(out)?;

    while !stdout.ends_with(":CMDEND\n") && shell.try_wait()?.is_none() {
        cmd_stdout.read_line(&mut stdout)?;

        erase(out, line_count)?;
        line_count = draw_file_info(out, Status::Running, file_name, line_number)?;
        line_count += draw_code(out, Status::Running, lang, program_and_args, false)?;
        line_count += draw_output(out, Status::Running, &stdout, "stdout", true)?;
        flush(out)?;
    }

    while !stderr.ends_with(":CMDEND\n") && shell.try_wait()?.is_none() {
        cmd_stderr.read_line(&mut stderr)?;

        erase(out, line_count)?;
        line_count = draw_file_info(out, Status::Running, file_name, line_number)?;
        line_count += draw_code(out, Status::Running, lang, program_and_args, false)?;
        line_count += draw_output(out, Status::Running, &stdout, "stdout", false)?;
        line_count += draw_output(out, Status::Running, &stderr, "stdout", true)?;
        flush(out)?;
    }

    match shell.try_wait()? {
        Some(code_raw) => {
            code = code_raw.code().unwrap();
        }
        None => {
            let re = regex::Regex::new(r#"(\d+):CMDEND"#).unwrap();
            let code_raw = re.captures(&stdout).unwrap().get(1).unwrap().as_str();

            code = code_raw.parse::<i32>().unwrap();

            stdout = stdout
                .trim_end_matches(":CMDEND\n")
                .trim_end_matches(code_raw)
                .to_string();

            stderr = stderr.trim_end_matches(":CMDEND\n").to_string();
        }
    }

    let status = if code == 0 { Status::PASS } else { Status::FAIL };

    erase(out, line_count)?;
    if !debug {
        draw_file_info(out, status, file_name, line_number)?;
        draw_code(out, status, lang, program_and_args, true)?;
    } else {
        draw_file_info(out, status, file_name, line_number)?;
        draw_code(out, status, lang, program_and_args, false)?;
        draw_output(out, status, &stdout, "sdtout", false)?;
        draw_output(out, status, &stderr, "sdterr", true)?;
    }

    write!(out, "{WRAP_ENABLE}")?;
    flush(out)?;

    Ok((stdout, stderr))
}

fn erase(out: &mut impl std::io::Write, line_count: usize) -> std::io::Result<()> {
    write!(out, "\x1b[{line_count}F\x1b[0J")
}

fn draw_file_info(
    out: &mut impl std::io::Write,
    status: Status,
    file_name: &str,
    line_number: usize,
) -> std::io::Result<usize> {
    match status {
        Status::Running => {
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
    }?;

    Ok(1)
}

fn draw_code(
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

fn draw_output(
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

fn accent(status: Status) -> &'static str {
    match status {
        Status::Running => YELLOW,
        Status::PASS => GREEN,
        Status::FAIL => RED,
    }
}

fn count_lines(mut data: &str) -> usize {
    let mut count = 0;

    while let Some(n) = data.find('\n') {
        data = &data[n + 1..];
        count += 1;
    }

    count
}

fn flush(out: &mut Vec<u8>) -> std::io::Result<()> {
    let mut stdout = std::io::stdout();
    stdout.write_all(out)?;
    out.clear();
    stdout.flush()
}
