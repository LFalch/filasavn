use std::{env::args, fs, path::PathBuf};

use filasavn::{add_file, read_savn_or_empty, write_savn, FileSpec, FileType::{ExecutableFile, RegularFile, SoftSymlink}};

enum Command<I: Iterator<Item=String>> {
    List,
    Add(I),
    Remove(I),
    Extract(String),
}

fn main() -> Result<(), String> {
    let mut args = args();
    let this_exe = args.next().unwrap();
    let archive = args.next().ok_or_else(|| format!("no archive given"))?;
    let command = args.next().ok_or_else(|| format!("no command given"))?;

    let cmd = match &*command {
        "-h" | "help" => {
            println!("syntax:");
            println!("\t{this_exe} <archive> list");
            println!("\t{this_exe} <archive> add <files..>");
            println!("\t{this_exe} <archive> remove <files..>");
            println!("\t{this_exe} <archive> extract <target folder>");
            return Ok(())
        },
        "list" => {
            if args.len() > 0 {
                return Err(format!("list does not take any arguments"));
            }
            Command::List
        }
        "add" => Command::Add(args),
        "remove" => Command::Remove(args),
        "extract" => {
            let target = args.next().ok_or_else(|| format!("no target folder given"))?;
            if args.len() > 0 {
                return Err(format!("list does not take any arguments"));
            }
            Command::Extract(target)
        }
        _ => return Err(format!("unknown command")),
    };
    drop(command);
    drop(this_exe);
    let mut savn = read_savn_or_empty(&archive).map_err(|e| e.to_string())?;

    match cmd {
        Command::List => {
            let mut max_length_width = 0;
            for FileSpec{contents, ..} in &savn {
                let length_width = contents.len().max(1).ilog10() as usize + 1;
                max_length_width = max_length_width.max(length_width);
            }
            for spec in &savn {
                match spec.file_type {
                    RegularFile => print!("f"),
                    ExecutableFile => print!("x"),
                    SoftSymlink => print!("l"),
                }
                print!(" {:width$} ", spec.contents.len(), width = max_length_width);
                println!("{}", spec.path);
            }
        }
        Command::Add(paths) => {
            for path in paths {
                add_file(&mut savn, path).map_err(|e| e.to_string())?;
            }
            write_savn(&savn, archive).map_err(|e| e.to_string())?;
        }
        Command::Remove(paths) => {
            let set: Vec<_> = paths.collect();
            savn.retain(|spec| !set.iter().any(|p| **p == *spec.path));
            write_savn(&savn, archive).map_err(|e| e.to_string())?;
        }
        Command::Extract(target) => {
            let target = PathBuf::from(target);
            
            for file_spec in savn {
                let path = target.join(&*file_spec.path);
                let mut path2 = path.clone();
                path2.pop();
                fs::create_dir_all(path2).map_err(|e| e.to_string())?;
                match file_spec.file_type {
                    RegularFile => fs::write(path, file_spec.contents).map_err(|e| e.to_string())?,
                    SoftSymlink => {
                        #[cfg(windows)]
                        {
                            compile_error!("todo");
                        }
                        #[cfg(unix)]
                        {
                            use std::ffi::OsStr;

                            let original: &OsStr = std::os::unix::ffi::OsStrExt::from_bytes(&file_spec.contents);
                            std::os::unix::fs::symlink(original, path).map_err(|e| e.to_string())?;
                        }
                    },
                    ExecutableFile => todo!(),
                }
            }
        }
    }

    Ok(())
}
