mod colors;
mod out;

use std::io::{BufRead as _, Write};

use out::*;

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

    // buffers
    let mut line = String::with_capacity(256);
    let mut cmd = String::with_capacity(256);

    // captures variables
    let mut vars = std::collections::HashMap::<String, String>::new();

    // buffered  output
    let mut out = std::io::BufWriter::new(std::io::stdout());

    // Command
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
            return err_file_ext(&mut out, &file_name);
        }

        let Ok(file) = std::fs::File::open(path) else {
            return err_file_open(&mut out, &file_name);
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
                let mut words = line.split_whitespace().skip(1);
                match words.next() {
                    Some("extract") => {
                        if !list {
                            let Some(var) = words.next() else {
                                return err_extract_no_var(&mut out, &file_name, line_number);
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
                                return err_extract_pattern(&mut out, &file_name, line_number, pat);
                            };

                            var_local.push((var.to_string(), re));
                        }
                    }
                    Some("env") => {
                        if !list {
                            let Some(mut var) = words.next().map(String::from) else {
                                return err_env_no_var(&mut out, &file_name, line_number);
                            };
                            let Ok(env) = std::env::var(&var) else {
                                return err_env_not_set(&mut out, &file_name, line_number, &var);
                            };

                            // Capture variables must be formatted as `<VAR_NAME>` for insertion
                            var.insert(0, '<');
                            var.push('>');
                            vars.insert(var.to_string(), env);
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
                line_number_code = line_number;
                let lang = line[3..line.len()].trim_end().to_string();

                if lang.len() == 0 {
                    return err_no_lang(&mut out, &file_name, line_number);
                }

                line.clear();

                while read_line_sanitized_cmd(&mut buff, &mut line)? != 0 && !line.trim().starts_with("```") {
                    line_number_code += 1;
                    cmd.push_str(&line);
                    line.clear();
                }

                if line.trim() != "```" {
                    return err_block_close(&mut out, &file_name, line_number, &line);
                }

                // Creates commands and interpolates any known capture variables
                let mut program_and_args = cmd.to_string();
                for (var, val) in vars.iter() {
                    program_and_args = program_and_args.replace(var, val.as_ref());
                }

                if lang != "bash" && lang != "sh" || ignore_cmd {
                    ignored(&mut out, &file_name, line_number, &program_and_args, debug)?;
                    ignore_cmd = false;
                    line.clear();
                    cmd.clear();
                    line_number = line_number_code + 1;
                    continue;
                }

                if list {
                    listed(&mut out, &file_name, line_number, &program_and_args, debug)?;
                    line.clear();
                    cmd.clear();
                    line_number = line_number_code + 1;
                    continue;
                }

                // Commands are run in the specified shell.
                // Currently, only `bash` and `sh` are supported.
                writeln!(
                    &cmd_stdin,
                    "{}; echo :CMDEND 1>&2; echo \"$?:CMDEND\"",
                    program_and_args.trim_end()
                )?;

                // let Ok(output) = process.output() else {
                //     return err_cmd_spawn(&mut out, &file_name, line_number, &program_and_args);
                // };
                //
                // if !output.status.success() {
                //     return err_cmd_failure(
                //         &mut out,
                //         &file_name,
                //         line_number,
                //         &program_and_args,
                //         &output.stdout,
                //         &output.stderr,
                //     );
                // }

                let mut stdout = String::with_capacity(256);
                let mut stderr = String::with_capacity(256);

                while !stdout.ends_with(":CMDEND\n") {
                    cmd_stdout.read_line(&mut stdout)?;
                }

                let re = regex::Regex::new(r#"(\d+):CMDEND"#).unwrap();
                let code = re.captures(&stdout).unwrap().get(1).unwrap().as_str();

                let stdout = stdout.trim_end_matches(":CMDEND\n");
                let stdout = stdout.trim_end_matches(code);

                while !stderr.ends_with(":CMDEND\n") {
                    cmd_stderr.read_line(&mut stderr)?;
                }

                let stderr = stderr.trim_end_matches(":CMDEND\n");

                // Looks for capture variables in the output of the command.
                // By default we look for captures in `stdout`. If none are found we look in
                // `stderr`. If no capture is found this counts as an error.
                for (mut var, re) in var_local {
                    let Some(cap) = re
                        .captures(stdout)
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

                    // Capture variables must be formatted as `<VAR_NAME>` for insertion
                    var.insert(0, '<');
                    var.push('>');

                    vars.insert(var, cap);
                }
                var_local = Vec::with_capacity(8);

                success(
                    &mut out,
                    &file_name,
                    line_number,
                    &program_and_args,
                    &stdout,
                    &stderr,
                    debug,
                )?;

                out.flush()?;
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
