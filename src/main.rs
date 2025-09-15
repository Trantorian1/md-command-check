mod colors;
mod draw;
mod err;

use std::io::{BufRead as _, Write};

use colors::*;
use draw::*;
use err::*;

fn main() -> std::io::Result<std::process::ExitCode> {
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

    let mut cmd_stdin = shell.stdin.take().unwrap();
    let mut cmd_stdout = std::io::BufReader::new(shell.stdout.take().unwrap());
    let mut cmd_stderr = std::io::BufReader::new(shell.stderr.take().unwrap());

    while let Some(file_name) = args.next() {
        let path = std::path::PathBuf::from(&file_name);
        if !path.extension().is_some_and(|ext| ext == "md") {
            return err_file_ext(&file_name);
        }

        let Ok(file) = std::fs::File::open(&path) else {
            return err_file_open(&file_name);
        };

        let mut buff = std::io::BufReader::new(file);
        let mut line_number = 0;
        let mut line_number_code;
        let mut cmd_ignore = false;
        let mut cmd_file = None;

        // list of variables to be captures from the next code block output
        let mut var_local = Vec::with_capacity(8);
        let mut kill_local = Vec::with_capacity(8);

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

                            let mut pat = String::new();
                            while let Some(word) = words.next() {
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
                    Some("kill") => {
                        if !list {
                            let mut pat = String::new();
                            while let Some(word) = words.next() {
                                if !pat.is_empty() {
                                    pat.push(' ');
                                }
                                pat.push_str(word);
                            }

                            let pat = pat.trim_matches('"');
                            let Ok(re) = regex::Regex::new(pat) else {
                                return err_kill_pattern(&file_name, line_number, pat);
                            };

                            kill_local.push(re);
                        }
                    }
                    Some("teardown") => {
                        let mut cmd = String::new();
                        while let Some(word) = words.next() {
                            if !cmd.is_empty() {
                                cmd.push(' ');
                            }
                            cmd.push_str(word);
                        }

                        let cmd = cmd.trim_matches('"');

                        teardown(&file_name, line_number, cmd)?;

                        if !list {
                            writeln!(&cmd_stdin, "{cmd}")?;
                        }
                    }
                    Some("file") => {
                        let Some(file) = words.next().map(String::from) else {
                            return err_file_name(&file_name, line_number);
                        };
                        cmd_file = Some(file);
                    }
                    Some("ignore") => {
                        cmd_ignore = true;
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

                if let Some(file) = cmd_file {
                    let path = path.with_file_name(file);
                    let path_str = path.to_string_lossy();

                    draw_file_info(&mut out, Status::NEWFILE, &path_str, line_number)?;
                    draw_code(&mut out, Status::NEWFILE, &lang, &program_and_args, true)?;
                    flush(&mut out)?;

                    if !list {
                        let mut file = std::fs::File::create(path)?;
                        file.write_all(program_and_args.as_bytes())?;
                    }

                    cmd_file = None;

                    line.clear();
                    cmd.clear();
                    line_number = line_number_code + 1;
                    continue;
                } else if lang != "bash" && lang != "sh" || cmd_ignore {
                    ignored(&file_name, line_number, &program_and_args, debug)?;

                    cmd_ignore = false;

                    line.clear();
                    cmd.clear();
                    line_number = line_number_code + 1;
                    continue;
                } else if list {
                    listed(&file_name, line_number, &program_and_args, debug)?;
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

                line_count += draw_file_info(&mut out, Status::RUNNING, &file_name, line_number)?;
                line_count += draw_code(&mut out, Status::RUNNING, &lang, &program_and_args, false)?;
                line_count += draw_output(&mut out, Status::RUNNING, &stdout, "stdout", true)?;
                flush(&mut out)?;

                while !stdout.ends_with(":CMDEND\n") && shell.try_wait()?.is_none() {
                    cmd_stdout.read_line(&mut stdout)?;

                    for re in kill_local.iter() {
                        if re.is_match(&stdout) {
                            shell.kill()?;
                            shell.wait()?;
                            break;
                        }
                    }

                    erase(&mut out, line_count)?;
                    line_count = draw_file_info(&mut out, Status::RUNNING, &file_name, line_number)?;
                    line_count += draw_code(&mut out, Status::RUNNING, &lang, &program_and_args, false)?;
                    line_count += draw_output(&mut out, Status::RUNNING, &stdout, "stdout", true)?;
                    flush(&mut out)?;
                }

                while !stderr.ends_with(":CMDEND\n") && shell.try_wait()?.is_none() {
                    cmd_stderr.read_line(&mut stderr)?;

                    for re in kill_local.iter() {
                        if re.is_match(&stderr) {
                            shell.kill()?;
                            shell.wait()?;
                            break;
                        }
                    }

                    erase(&mut out, line_count)?;
                    line_count = draw_file_info(&mut out, Status::RUNNING, &file_name, line_number)?;
                    line_count += draw_code(&mut out, Status::RUNNING, &lang, &program_and_args, false)?;
                    line_count += draw_output(&mut out, Status::RUNNING, &stdout, "stdout", false)?;
                    line_count += draw_output(&mut out, Status::RUNNING, &stderr, "stdout", true)?;
                    flush(&mut out)?;
                }

                match shell.try_wait()? {
                    Some(code_raw) => {
                        code = match code_raw.code() {
                            Some(n) => n,
                            None => {
                                shell = std::process::Command::new("sh")
                                    .stdin(std::process::Stdio::piped())
                                    .stdout(std::process::Stdio::piped())
                                    .stderr(std::process::Stdio::piped())
                                    .spawn()?;
                                cmd_stdin = shell.stdin.take().unwrap();
                                cmd_stdout = std::io::BufReader::new(shell.stdout.take().unwrap());
                                cmd_stderr = std::io::BufReader::new(shell.stderr.take().unwrap());
                                0
                            }
                        };
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

                if code != 0 {
                    return Ok(std::process::ExitCode::FAILURE);
                }

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

    Ok(std::process::ExitCode::SUCCESS)
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
