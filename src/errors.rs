use crate::colors::*;

pub fn success(out: &mut impl std::io::Write, file_name: &str, line_number: usize) -> std::io::Result<()> {
    write!(
        out,
        "\
            ✅{BOLD}{file_name}{RESET}: \
            code block at line {line_number} - \
            {GREEN}PASS{RESET}\n\
        "
    )
}

pub fn ignored(out: &mut impl std::io::Write, file_name: &str, line_number: usize) -> std::io::Result<()> {
    write!(
        out,
        "  \
            {BOLD}{file_name}{RESET}: \
            code block at line {line_number} - \
            {FAINT}{ITALIC}IGNORED{RESET}\n\
        "
    )
}

pub fn err(out: &mut impl std::io::Write, file_name: &str) -> std::io::Result<()> {
    write!(out, "❌{BOLD}{file_name}{RESET}: ",)
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
    stdout: &[u8],
    stderr: &[u8],
) -> std::io::Result<()> {
    err_line_code(out, file_name, line_number)?;
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
    )?;

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
        )?;
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
        )?;
    }

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
    )?;

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
        )?;
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
        )?;
    }

    return Ok(());
}
