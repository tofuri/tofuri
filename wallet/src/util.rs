use std::fs::create_dir_all;
use std::fs::read_dir;
use std::path::Path;
#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
}
pub fn default_path() -> &'static Path {
    Path::new("./tofuri-wallet")
}
pub fn filenames() -> Result<Vec<String>, Error> {
    let path = default_path();
    if !path.exists() {
        create_dir_all(path).unwrap();
    }
    let mut filenames: Vec<String> = vec![];
    for entry in read_dir(path).unwrap() {
        filenames.push(
            entry
                .map_err(Error::Io)?
                .path()
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned(),
        );
    }
    Ok(filenames)
}
