use std::{io, path::Path};

use oxc_resolver::FileSystem;

use crate::TestUnit;

pub struct TestFileSystem<'a> {
    pub unit: &'a TestUnit<'a>,
}

impl FileSystem for &TestFileSystem<'_> {
    fn read_to_string(&self, path: &std::path::Path) -> std::io::Result<String> {
        let Some(file_id) = self.unit.file_names.position(|x| Path::new(x) == path) else {
            return Err(io::Error::from(io::ErrorKind::NotFound));
        };

        Ok(self.unit.file_contents[file_id].to_string())
    }

    #[expect(clippy::todo)]
    fn metadata(&self, _path: &std::path::Path) -> std::io::Result<oxc_resolver::FileMetadata> {
        todo!()
    }

    #[expect(clippy::todo)]
    fn symlink_metadata(
        &self,
        _path: &std::path::Path,
    ) -> std::io::Result<oxc_resolver::FileMetadata> {
        todo!()
    }

    #[expect(clippy::todo)]
    fn read_link(&self, _path: &Path) -> io::Result<std::path::PathBuf> {
        todo!()
    }
}
