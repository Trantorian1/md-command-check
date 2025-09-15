use std::io::Write as _;

use crate::colors::*;

pub fn listed(file_name: &str, line_number: usize, program_and_args: &str, debug: bool) -> std::io::Result<()> {
    write!(
        std::io::stdout(),
        "\
            {YELLOW}╭[    {RESET}{BOLD}{file_name}{RESET}: \
            code block at line {line_number} - \
            {YELLOW}ACTIVE{RESET}\n\
        "
    )?;

    if debug {
        log_program(program_and_args, true, YELLOW)?;
    }

    Ok(())
}

pub fn ignored(file_name: &str, line_number: usize, program_and_args: &str, debug: bool) -> std::io::Result<()> {
    write!(
        std::io::stdout(),
        "\
            ╭[    {BOLD}{file_name}{RESET}: \
            code block at line {line_number} - \
            {FAINT}{ITALIC}IGNORED{RESET}\n\
        "
    )?;

    if debug {
        log_program(program_and_args, true, RESET)?;
    }

    Ok(())
}

pub fn err(file_name: &str) -> std::io::Result<()> {
    write!(std::io::stdout(), "{RED}╭[ ❌ {RESET}{BOLD}{file_name}{RESET}: ",)
}

pub fn err_line_directive(file_name: &str, line_number: usize, directive: &str) -> std::io::Result<()> {
    err(file_name)?;
    write!(std::io::stdout(), "{directive} directive at line {line_number} - ",)
}

pub fn err_line_code(file_name: &str, line_number: usize) -> std::io::Result<()> {
    err(file_name)?;
    write!(std::io::stdout(), "code block at line {line_number} - ",)
}

pub fn err_file_ext(file_name: &str) -> std::io::Result<()> {
    err(file_name)?;
    write!(std::io::stdout(), "{RED}File is not a .md{RESET}\n")
}

pub fn err_file_open(file_name: &str) -> std::io::Result<()> {
    err(file_name)?;
    write!(std::io::stdout(), "{RED}Failed to open file{RESET}\n")
}

pub fn err_extract_no_var(file_name: &str, line_number: usize) -> std::io::Result<()> {
    err_line_directive(file_name, line_number, "extract")?;
    write!(std::io::stdout(), "{RED}No variable name{RESET}\n")
}

pub fn err_env_no_var(file_name: &str, line_number: usize) -> std::io::Result<()> {
    err_line_directive(file_name, line_number, "env")?;
    write!(std::io::stdout(), "{RED}No variable name{RESET}\n")
}

pub fn err_env_not_set(file_name: &str, line_number: usize, var: &str) -> std::io::Result<()> {
    err_line_directive(file_name, line_number, "env")?;
    write!(
        std::io::stdout(),
        "{RED}Variable was not set:{RESET} {ITALIC}{var}{RESET}\n"
    )
}

pub fn err_alias_no_var(file_name: &str, line_number: usize) -> std::io::Result<()> {
    err_line_directive(file_name, line_number, "alias")?;
    write!(std::io::stdout(), "{RED}No variable name{RESET}\n")
}

pub fn err_alias_not_captured(file_name: &str, line_number: usize, var: &str) -> std::io::Result<()> {
    err_line_directive(file_name, line_number, "alias")?;
    write!(
        std::io::stdout(),
        "{RED}Variable has not been previously captured:{RESET} {ITALIC}{var}{RESET}\n"
    )
}

pub fn err_extract_pattern(file_name: &str, line_number: usize, pat: &str) -> std::io::Result<()> {
    err_line_directive(file_name, line_number, "extract")?;
    write!(std::io::stdout(), "{RED}Invalid pattern:{RESET}{pat}\n")
}

pub fn err_no_lang(file_name: &str, line_number: usize) -> std::io::Result<()> {
    err_line_code(file_name, line_number)?;
    write!(std::io::stdout(), "{RED}No language specified{RESET}\n")
}

pub fn err_block_close(file_name: &str, line_number: usize, delimiter: &str) -> std::io::Result<()> {
    err_line_code(file_name, line_number)?;
    if !delimiter.is_empty() {
        write!(
            std::io::stdout(),
            "{RED}Invalid closing delimiter:{RESET} {delimiter}\n"
        )
    } else {
        write!(std::io::stdout(), "{RED}Unclosed block{RESET}\n")
    }
}

pub fn err_cmd_capture(
    file_name: &str,
    line_number: usize,
    program_and_args: &str,
    stdout: &str,
    stderr: &str,
    re: &regex::Regex,
) -> std::io::Result<()> {
    err_line_code(file_name, line_number)?;
    write!(
        std::io::stdout(),
        "{RED}Failed to capture matches:{RESET} {ITALIC}\"{re}\"{RESET}\n",
    )?;

    log_program(program_and_args, stdout.is_empty() && stderr.is_empty(), RED)?;
    log_stdout(stdout.to_string(), stderr.is_empty(), RED)?;
    log_stderr(stderr.to_string(), RED)?;

    return Ok(());
}

fn log_program(program_and_args: &str, terminate: bool, accent: &str) -> std::io::Result<()> {
    let program_and_args = program_and_args
        .trim()
        .to_string()
        .replace("\n", &format!("\n{accent}│{RESET} "));
    if !terminate {
        write!(
            std::io::stdout(),
            "\
                {accent}│{RESET} ```\n\
                {accent}│{RESET} {program_and_args}\n\
                {accent}│{RESET} ```\n\
            ",
        )
    } else {
        write!(
            std::io::stdout(),
            "\
                {accent}│{RESET} ```\n\
                {accent}│{RESET} {program_and_args}\n\
                {accent}╰{RESET} ```\n\
            ",
        )
    }
}

fn log_stdout(mut stdout: String, terminate: bool, accent: &str) -> std::io::Result<()> {
    if !stdout.is_empty() {
        stdout = stdout.trim().replace("\n", &format!("\n{accent}│{RESET} >> "));
        if !terminate {
            write!(
                std::io::stdout(),
                "\
                    {accent}│{RESET} >> {BOLD}{ITALIC}stdout{RESET}\n\
                    {accent}│{RESET} >>\n\
                    {accent}│{RESET} >> {stdout}\n\
                    {accent}╰{RESET} >>\n\
                ",
            )?;
        } else {
            write!(
                std::io::stdout(),
                "\
                    {accent}│{RESET} >> {BOLD}{ITALIC}stdout{RESET}\n\
                    {accent}│{RESET} >>\n\
                    {accent}│{RESET} >> {stdout}\n\
                    {accent}╰{RESET} >>\n\
                ",
            )?;
        }
    }

    Ok(())
}

fn log_stderr(mut stderr: String, accent: &str) -> std::io::Result<()> {
    if !stderr.is_empty() {
        stderr = stderr.trim().replace("\n", &format!("\n{accent}│{RESET} >> "));
        write!(
            std::io::stdout(),
            "\
                {accent}│{RESET} >> {BOLD}{ITALIC}stderr{RESET}\n\
                {accent}│{RESET} >>\n\
                {accent}│{RESET} >> {stderr}\n\
                {accent}╰{RESET} >>\n\
            ",
        )?;
    }

    Ok(())
}
