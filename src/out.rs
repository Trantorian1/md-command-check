use crate::colors::*;

pub fn success(
    out: &mut impl std::io::Write,
    file_name: &str,
    line_number: usize,
    program_and_args: &str,
    stdout: &str,
    stderr: &str,
    debug: bool,
) -> std::io::Result<()> {
    write!(
        out,
        "\
            {GREEN}╭[ ✅ {RESET}{BOLD}{file_name}{RESET}: \
            code block at line {line_number} - \
            {GREEN}PASS{RESET}\n\
        "
    )?;

    if debug {
        log_program(out, program_and_args, stdout.is_empty() && stderr.is_empty(), GREEN)?;
        log_stdout(out, stdout.to_string(), stderr.is_empty(), GREEN)?;
        log_stderr(out, stderr.to_string(), GREEN)?;
    }

    Ok(())
}

pub fn listed(
    out: &mut impl std::io::Write,
    file_name: &str,
    line_number: usize,
    program_and_args: &str,
    debug: bool,
) -> std::io::Result<()> {
    write!(
        out,
        "\
            {YELLOW}╭[    {RESET}{BOLD}{file_name}{RESET}: \
            code block at line {line_number} - \
            {YELLOW}ACTIVE{RESET}\n\
        "
    )?;

    if debug {
        log_program(out, program_and_args, true, YELLOW)?;
    }

    Ok(())
}

pub fn ignored(
    out: &mut impl std::io::Write,
    file_name: &str,
    line_number: usize,
    program_and_args: &str,
    debug: bool,
) -> std::io::Result<()> {
    write!(
        out,
        "\
            ╭[    {BOLD}{file_name}{RESET}: \
            code block at line {line_number} - \
            {FAINT}{ITALIC}IGNORED{RESET}\n\
        "
    )?;

    if debug {
        log_program(out, program_and_args, true, RESET)?;
    }

    Ok(())
}

pub fn err(out: &mut impl std::io::Write, file_name: &str) -> std::io::Result<()> {
    write!(out, "{RED}╭[ ❌ {RESET}{BOLD}{file_name}{RESET}: ",)
}

pub fn err_line_directive(
    out: &mut impl std::io::Write,
    file_name: &str,
    line_number: usize,
    directive: &str,
) -> std::io::Result<()> {
    err(out, file_name)?;
    write!(out, "{directive} directive at line {line_number} - ",)
}

pub fn err_line_code(out: &mut impl std::io::Write, file_name: &str, line_number: usize) -> std::io::Result<()> {
    err(out, file_name)?;
    write!(out, "code block at line {line_number} - ",)
}

pub fn err_file_ext(out: &mut impl std::io::Write, file_name: &str) -> std::io::Result<()> {
    err(out, file_name)?;
    write!(out, "{RED}File is not a .md{RESET}\n")
}

pub fn err_file_open(out: &mut impl std::io::Write, file_name: &str) -> std::io::Result<()> {
    err(out, file_name)?;
    write!(out, "{RED}Failed to open file{RESET}\n")
}

pub fn err_extract_no_var(out: &mut impl std::io::Write, file_name: &str, line_number: usize) -> std::io::Result<()> {
    err_line_directive(out, file_name, line_number, "extract")?;
    write!(out, "{RED}No variable name{RESET}\n")
}

pub fn err_env_no_var(out: &mut impl std::io::Write, file_name: &str, line_number: usize) -> std::io::Result<()> {
    err_line_directive(out, file_name, line_number, "env")?;
    write!(out, "{RED}No variable name{RESET}\n")
}

pub fn err_env_not_set(
    out: &mut impl std::io::Write,
    file_name: &str,
    line_number: usize,
    var: &str,
) -> std::io::Result<()> {
    err_line_directive(out, file_name, line_number, "env")?;
    write!(out, "{RED}Variable was not set:{RESET} {ITALIC}{var}{RESET}\n")
}

pub fn err_extract_pattern(
    out: &mut impl std::io::Write,
    file_name: &str,
    line_number: usize,
    pat: &str,
) -> std::io::Result<()> {
    err_line_directive(out, file_name, line_number, "extract")?;
    write!(out, "{RED}Invalid pattern:{RESET}{pat}\n")
}

pub fn err_no_lang(out: &mut impl std::io::Write, file_name: &str, line_number: usize) -> std::io::Result<()> {
    err_line_code(out, file_name, line_number)?;
    write!(out, "{RED}No language specified{RESET}\n")
}

pub fn err_block_close(
    out: &mut impl std::io::Write,
    file_name: &str,
    line_number: usize,
    delimiter: &str,
) -> std::io::Result<()> {
    err_line_code(out, file_name, line_number)?;
    if !delimiter.is_empty() {
        write!(out, "{RED}Invalid closing delimiter:{RESET} {delimiter}\n")
    } else {
        write!(out, "{RED}Unclosed block{RESET}\n")
    }
}

pub fn err_cmd_spawn(
    out: &mut impl std::io::Write,
    file_name: &str,
    line_number: usize,
    program_and_args: &str,
) -> std::io::Result<()> {
    err_line_code(out, file_name, line_number)?;
    write!(
        out,
        "\
            {RED}Could not run command{RESET}\n\
            {FAINT}\
            ```\n\
            {program_and_args}\n\
            ```\n\
            {RESET}\
        ",
    )
}

pub fn err_cmd_failure(
    out: &mut impl std::io::Write,
    file_name: &str,
    line_number: usize,
    program_and_args: &str,
    stdout: &str,
    stderr: &str,
) -> std::io::Result<()> {
    err_line_code(out, file_name, line_number)?;
    write!(out, "{RED}FAIL{RESET}\n",)?;

    log_program(out, program_and_args, stdout.is_empty() && stderr.is_empty(), RED)?;
    log_stdout(out, stdout.to_string(), stderr.is_empty(), RED)?;
    log_stderr(out, stderr.to_string(), RED)?;

    Ok(())
}

pub fn err_cmd_capture(
    out: &mut impl std::io::Write,
    file_name: &str,
    line_number: usize,
    program_and_args: &str,
    stdout: &str,
    stderr: &str,
    re: &regex::Regex,
) -> std::io::Result<()> {
    err_line_code(out, file_name, line_number)?;
    write!(out, "{RED}Failed to capture matches:{RESET} {ITALIC}\"{re}\"{RESET}\n",)?;

    log_program(out, program_and_args, stdout.is_empty() && stderr.is_empty(), RED)?;
    log_stdout(out, stdout.to_string(), stderr.is_empty(), RED)?;
    log_stderr(out, stderr.to_string(), RED)?;

    return Ok(());
}

fn log_program(
    out: &mut impl std::io::Write,
    program_and_args: &str,
    terminate: bool,
    accent: &str,
) -> std::io::Result<()> {
    let program_and_args = program_and_args
        .trim()
        .to_string()
        .replace("\n", &format!("\n{accent}│{RESET} "));
    if !terminate {
        write!(
            out,
            "\
                {accent}│{RESET} ```\n\
                {accent}│{RESET} {program_and_args}\n\
                {accent}│{RESET} ```\n\
            ",
        )
    } else {
        write!(
            out,
            "\
                {accent}│{RESET} ```\n\
                {accent}│{RESET} {program_and_args}\n\
                {accent}╰{RESET} ```\n\
            ",
        )
    }
}

fn log_stdout(out: &mut impl std::io::Write, mut stdout: String, terminate: bool, accent: &str) -> std::io::Result<()> {
    if !stdout.is_empty() {
        stdout = stdout.trim().replace("\n", &format!("\n{accent}│{RESET} >> "));
        if !terminate {
            write!(
                out,
                "\
                    {accent}│{RESET} >> {BOLD}{ITALIC}stdout{RESET}\n\
                    {accent}│{RESET} >>\n\
                    {accent}│{RESET} >> {stdout}\n\
                    {accent}╰{RESET} >>\n\
                ",
            )?;
        } else {
            write!(
                out,
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

fn log_stderr(out: &mut impl std::io::Write, mut stderr: String, accent: &str) -> std::io::Result<()> {
    if !stderr.is_empty() {
        stderr = stderr.trim().replace("\n", &format!("\n{accent}│{RESET} >> "));
        write!(
            out,
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
