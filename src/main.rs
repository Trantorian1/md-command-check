mod colors;
mod draw;
mod err;

use std::io::{BufRead as _, Write};

use colors::*;
use draw::*;
use err::*;

fn main() -> std::io::Result<()> {
    let mut args = std::env::args().skip(1).peekable();

    // options
    let mut debug = false;
    let mut list = false;

    while args.peek().is_some_and(|arg| arg.starts_with("--")) {
        match args.next().expect("Checked above").as_ref() {
            "--debug" => debug = true,
            "--list" => list = true,
            _ => {}
        }
    }

    // readline buffers
    let mut line = String::with_capacity(256);
    let mut cmd = String::with_capacity(256);

    // capture variables
    let mut vars = std::collections::HashMap::<String, String>::new();

    // manual output
    let mut out = Vec::with_capacity(8192); // 8kb

    // Long-running shell process. We spawn our commands in here
    let mut shell = std::process::Command::new("sh")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    let cmd_stdin = shell.stdin.take().unwrap();
    let mut cmd_stdout = std::io::BufReader::new(shell.stdout.take().unwrap());
    let mut cmd_stderr = std::io::BufReader::new(shell.stderr.take().unwrap());

    while let Some(file_name) = args.next() {
        let path = std::path::PathBuf::from(&file_name);
        if !path.extension().is_some_and(|ext| ext == "md") {
            return err_file_ext(&file_name);
        }

        let Ok(file) = std::fs::File::open(path) else {
            return err_file_open(&file_name);
        };

        let mut buff = std::io::BufReader::new(file);
        let mut line_number = 0;
        let mut line_number_code;
        let mut ignore_cmd = false;

        // list of variables to be captures from the next code block output
        let mut var_local = Vec::with_capacity(8);

        while read_line_sanitized(&mut buff, &mut line)? != 0 {
            line_number += 1;

            // We've found a comment!
            if line.starts_with("<!--") {
                // ============================================================================== //
                //                              DIRECTIVE EXTRACTION                              //
                // ============================================================================== //

                let mut words = line
                    .trim_start_matches("<!--")
                    .trim_end_matches("-->\n")
                    .split_whitespace();
                match words.next() {
                    Some("extract") => {
                        if !list {
                            let Some(var) = words.next() else {
                                return err_extract_no_var(&file_name, line_number);
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
                                return err_extract_pattern(&file_name, line_number, pat);
                            };

                            var_local.push((var.to_string(), re));
                        }
                    }
                    Some("env") => {
                        if !list {
                            let Some(mut var) = words.next().map(String::from) else {
                                return err_env_no_var(&file_name, line_number);
                            };
                            let Some(key) = words.next().map(String::from) else {
                                return err_env_no_var(&file_name, line_number);
                            };
                            let Ok(env) = std::env::var(&key) else {
                                return err_env_not_set(&file_name, line_number, &key);
                            };

                            // Capture variables must be formatted as `<VAR_NAME>` for insertion
                            var.insert(0, '<');
                            var.push('>');
                            vars.insert(var.to_string(), env);
                        }
                    }
                    Some("alias") => {
                        if !list {
                            let Some(mut var) = words.next().map(String::from) else {
                                return err_alias_no_var(&file_name, line_number);
                            };

                            var.insert(0, '<');
                            var.push('>');

                            let Some(mut alias) = words.next().map(String::from) else {
                                return err_alias_no_var(&file_name, line_number);
                            };
                            let Some(val) = vars.get(&var).map(String::from) else {
                                return err_alias_not_captured(&file_name, line_number, &var);
                            };

                            alias.insert(0, '<');
                            alias.push('>');
                            vars.insert(alias, val);
                        }
                    }
                    Some("ignore") => {
                        ignore_cmd = true;
                    }
                    _ => {}
                }
            }
            // We've found a code block!
            else if line.starts_with("```") {
                // ============================================================================== //
                //                               COMMAND EXTRACTION                               //
                // ============================================================================== //
                line_number_code = line_number;
                let lang = line[3..line.len()].trim_end().to_string();

                if lang.len() == 0 {
                    return err_no_lang(&file_name, line_number);
                }

                line.clear();

                while read_line_sanitized_cmd(&mut buff, &mut line)? != 0 && !line.trim().starts_with("```") {
                    line_number_code += 1;
                    cmd.push_str(&line);
                    line.clear();
                }

                if line.trim() != "```" {
                    return err_block_close(&file_name, line_number, &line);
                }

                // Creates commands and interpolates any known capture variables
                let mut program_and_args = cmd.to_string();
                for (var, val) in vars.iter() {
                    program_and_args = program_and_args.replace(var, val.as_ref());
                }

                if lang != "bash" && lang != "sh" || ignore_cmd {
                    ignored(&file_name, line_number, &program_and_args, debug)?;
                    ignore_cmd = false;
                    out.flush()?;
                    line.clear();
                    cmd.clear();
                    line_number = line_number_code + 1;
                    continue;
                }

                if list {
                    listed(&file_name, line_number, &program_and_args, debug)?;

                    let mut stdout = std::io::stdout();
                    stdout.write_all(&out)?;
                    out.clear();
                    stdout.flush()?;

                    line.clear();
                    cmd.clear();
                    line_number = line_number_code + 1;
                    continue;
                }

                // Commands are run in the specified shell.
                // Currently, only `bash` and `sh` are supported.
                writeln!(
                    &cmd_stdin,
                    "{}; echo \"$?:CMDEND\"; echo :CMDEND 1>&2",
                    program_and_args.trim_end()
                )?;

                // ============================================================================== //
                //                                  DRAW ROUTINE                                  //
                // ============================================================================== //

                let mut line_count = 0;

                let mut stdout = String::with_capacity(256);
                let mut stderr = String::with_capacity(256);
                let code;

                write!(out, "{WRAP_DISABLE}")?;

                line_count += draw_file_info(&mut out, Status::Running, &file_name, line_number)?;
                line_count += draw_code(&mut out, Status::Running, &lang, &program_and_args, false)?;
                line_count += draw_output(&mut out, Status::Running, &stdout, "stdout", true)?;
                flush(&mut out)?;

                while !stdout.ends_with(":CMDEND\n") && shell.try_wait()?.is_none() {
                    cmd_stdout.read_line(&mut stdout)?;

                    erase(&mut out, line_count)?;
                    line_count = draw_file_info(&mut out, Status::Running, &file_name, line_number)?;
                    line_count += draw_code(&mut out, Status::Running, &lang, &program_and_args, false)?;
                    line_count += draw_output(&mut out, Status::Running, &stdout, "stdout", true)?;
                    flush(&mut out)?;
                }

                while !stderr.ends_with(":CMDEND\n") && shell.try_wait()?.is_none() {
                    cmd_stderr.read_line(&mut stderr)?;

                    erase(&mut out, line_count)?;
                    line_count = draw_file_info(&mut out, Status::Running, &file_name, line_number)?;
                    line_count += draw_code(&mut out, Status::Running, &lang, &program_and_args, false)?;
                    line_count += draw_output(&mut out, Status::Running, &stdout, "stdout", false)?;
                    line_count += draw_output(&mut out, Status::Running, &stderr, "stdout", true)?;
                    flush(&mut out)?;
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

                erase(&mut out, line_count)?;
                if !debug {
                    draw_file_info(&mut out, status, &file_name, line_number)?;
                    draw_code(&mut out, status, &lang, &program_and_args, true)?;
                } else {
                    draw_file_info(&mut out, status, &file_name, line_number)?;
                    draw_code(&mut out, status, &lang, &program_and_args, false)?;
                    draw_output(&mut out, status, &stdout, "sdtout", false)?;
                    draw_output(&mut out, status, &stderr, "sdterr", true)?;
                }

                write!(out, "{WRAP_ENABLE}")?;
                flush(&mut out)?;

                // ============================================================================== //
                //                                 OUTPUT CAPTURE                                 //
                // ============================================================================== //

                // Looks for capture variables in the output of the command.
                // By default we look for captures in `stdout`. If none are found we look in
                // `stderr`. If no capture is found this counts as an error.
                for (mut var, re) in var_local {
                    let Some(cap) = re
                        .captures(&stdout)
                        .or_else(|| re.captures(&stderr))
                        .and_then(|cap| cap.get(1))
                        .map(|cap| cap.as_str().to_string())
                    else {
                        return err_cmd_capture(&file_name, line_number, &program_and_args, &stdout, &stderr, &re);
                    };

                    // Capture variables must be formatted as `<VAR_NAME>` for insertion
                    var.insert(0, '<');
                    var.push('>');

                    vars.insert(var, cap);
                }
                var_local = Vec::with_capacity(8);

                cmd.clear();
                line_number = line_number_code + 1;
            }

            line.clear();
        }
    }

    shell.kill()?;

    Ok(())
}

fn read_line_sanitized(buff: &mut impl std::io::BufRead, line: &mut String) -> std::io::Result<usize> {
    let n = buff.read_line(line)?;
    *line = line.strip_prefix('>').unwrap_or(&line).trim_start().to_string();
    Ok(n)
}

fn read_line_sanitized_cmd(buff: &mut impl std::io::BufRead, line: &mut String) -> std::io::Result<usize> {
    let n = buff.read_line(line)?;
    *line = line.strip_prefix('>').unwrap_or(&line).to_string();
    Ok(n)
}
