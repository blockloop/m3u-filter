use std::fs::File;
use linereader::LineReader;

pub(crate) struct FileReader  {
    reader: LineReader<File>,
}

impl FileReader {
    pub(crate) fn new(file: File) -> FileReader {
        FileReader {
            reader: LineReader::new(file),
        }
    }
}

impl Iterator for FileReader {
    type Item = String;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(Ok(buf)) = self.reader.next_line() {
           return Some(String::from_utf8_lossy(buf).trim_end_matches(char::is_control).to_string());
        }
        None
    }
}