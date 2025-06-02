use std::{fs::{self, File}, io::{self, BufRead, ErrorKind, Read, Write}, path::Path};

pub type Savn = Vec<FileSpec>;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    #[default]
    RegularFile = 1,
    ExecutableFile = 2,
    SoftSymlink = 3,
}

pub struct FileSpec {
    pub path: Box<str>,
    pub file_type: FileType,
    pub contents: Box<[u8]>,
}

pub fn read_savn_or_empty(savn_path: impl AsRef<Path>) -> io::Result<Savn> {
    match read_savn(savn_path) {
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(Savn::new()),
        res => res,
    }
}
pub fn read_savn(savn_path: impl AsRef<Path>) -> io::Result<Savn> {
    let mut f = io::BufReader::new(File::open(savn_path)?);
    let mut savn = Savn::new();

    loop {
        let mut path = Vec::with_capacity(8);
        f.read_until(0, &mut path)?;
        // pop final zero
        path.pop();
        if path.is_empty() {
            // if the name is empty that means we've reached the end of the archive
            break;
        }
        let path = String::from_utf8(path)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?
            .into_boxed_str();
        let mut b = [0];
        f.read_exact(&mut b)?;
        let file_type = match b {
            [1] => FileType::RegularFile,
            [3] => FileType::SoftSymlink,
            _ => return Err(io::Error::new(io::ErrorKind::InvalidData, "unknown file type")),
        };
        let mut b = 0u32.to_le_bytes();
        f.read_exact(&mut b)?;
        let size = u32::from_le_bytes(b);
        let mut contents = vec![0; size as usize];
        f.read_exact(&mut contents)?;
        let contents = contents.into_boxed_slice();

        savn.push(FileSpec { path, file_type, contents })
    }

    Ok(savn)
}
pub fn write_savn(savn: &Savn, savn_path: impl AsRef<Path>) -> io::Result<()> {
    let mut f = io::BufWriter::new(File::create(savn_path)?);

    for spec in savn {
        f.write_all(spec.path.as_bytes())?;
        f.write_all(&[0])?;
        f.write_all(&[spec.file_type as u8])?;
        f.write_all(&(spec.contents.len() as u32).to_le_bytes())?;
        f.write_all(&spec.contents)?;
    }
    // write final zero to indicate archive end
    f.write_all(&[0])?;

    Ok(())
}
pub fn find_file<'a>(savn: &'a Savn, file: &str) -> Option<&'a FileSpec> {
    for entry in savn {
        if &*entry.path == file {
            return Some(entry);
        }
    }
    None
}
pub fn add_file(savn: &mut Savn, file: impl AsRef<Path>) -> io::Result<()> {
    let path = file.as_ref();

    let metadata = fs::symlink_metadata(path)?;
    let mut file_type = FileType::default();
    let contents = if metadata.is_symlink() {
        file_type = FileType::SoftSymlink;
        path.read_link()?.to_str().unwrap().as_bytes().to_vec().into_boxed_slice()
    } else if metadata.is_dir() {
        for entry in path.read_dir()? {
            let entry = entry?;
            add_file(savn, entry.path())?;
        }
        // if it's a directory, we do nothing
        return Ok(());
    } else {
        fs::read(path)?.into_boxed_slice()
    };

    savn.push(FileSpec {
        path: path.display().to_string().into_boxed_str(),
        file_type,
        contents,
    });
    Ok(())
}
