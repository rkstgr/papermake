use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use once_cell::sync::Lazy;
use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime, Dict, IntoValue};
use typst::syntax::{FileId, Source};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::Library;
use typst_kit::fonts::{FontSearcher, FontSlot};

// Define a static lazy variable to hold the cached fonts
static CACHED_FONTS: Lazy<(FontBook, Vec<Font>)> = Lazy::new(|| {
    let mut font_searcher = FontSearcher::new();
    let font_searcher = font_searcher.include_system_fonts(true);

    let fonts = match std::env::var_os("FONTS_DIR") {
        Some(fonts_dir) => {
            let fonts_dir = PathBuf::from(fonts_dir);
            font_searcher.search_with([&fonts_dir])
        }
        None => font_searcher.search(),
    };

    let book = fonts.book;
    let fonts = fonts
        .fonts
        .iter()
        .map(FontSlot::get)
        .filter_map(|f| f)
        .collect::<Vec<_>>();

    (book, fonts)
});

/// Main interface that determines the environment for Typst.
#[derive(Debug)]
pub struct TypstWorld {
    /// The content of a source.
    pub source: Source,

    /// The standard library.
    library: LazyHash<Library>,

    /// Metadata about all known fonts.
    book: LazyHash<FontBook>,

    /// Metadata about all known fonts.
    fonts: Vec<Font>,

    /// Map of all known files.
    files: Arc<Mutex<HashMap<FileId, FileEntry>>>,

    /// Cache directory (e.g. where packages are downloaded to).
    cache_directory: PathBuf,

    /// Datetime.
    time: time::OffsetDateTime,
}

impl TypstWorld {
    pub fn new(template_content: String, data: String) -> Self {
        // Use the cached fonts directly
        let (book, fonts) = CACHED_FONTS.clone();

        let mut inputs_dict = Dict::new();
        inputs_dict.insert("data".into(), data.as_str().into_value());

        let library = Library::builder().with_inputs(inputs_dict).build();

        Self {
            library: LazyHash::new(library),
            book: LazyHash::new(book),
            fonts, // Use the cached fonts
            source: Source::detached(template_content),
            time: time::OffsetDateTime::now_utc(),
            cache_directory: std::env::var_os("CACHE_DIRECTORY")
                .map(|os_path| os_path.into())
                .unwrap_or(std::env::temp_dir()),
            files: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn update_data(&mut self, data: String) -> Result<(), String> {
        // Update the data in the inputs dictionary
        let mut inputs_dict = Dict::new();
        inputs_dict.insert("data".into(), data.as_str().into_value());
        
        // Create a new library with updated inputs
        // Note: This is not optimal - ideally we'd modify the existing library
        let library = Library::builder().with_inputs(inputs_dict).build();
        self.library = LazyHash::new(library);
        
        Ok(())
    }
}

/// A File that will be stored in the HashMap.
#[derive(Clone, Debug)]
struct FileEntry {
    bytes: Bytes,
    source: Option<Source>,
}

impl FileEntry {
    fn new(bytes: Vec<u8>, source: Option<Source>) -> Self {
        Self {
            bytes: Bytes::new(bytes),
            source,
        }
    }

    fn source(&mut self, id: FileId) -> FileResult<Source> {
        let source = if let Some(source) = &self.source {
            source
        } else {
            let contents = std::str::from_utf8(&self.bytes).map_err(|_| FileError::InvalidUtf8)?;
            let contents = contents.trim_start_matches('\u{feff}');
            let source = Source::new(id, contents.into());
            self.source.insert(source)
        };
        Ok(source.clone())
    }
}

impl TypstWorld {
    /// Helper to handle file requests.
    ///
    /// Requests will be either in packages or a local file.
    fn file(&self, id: FileId) -> FileResult<FileEntry> {
        let files = self.files.lock().map_err(|_| FileError::AccessDenied)?;
        if let Some(entry) = files.get(&id) {
            return Ok(entry.clone());
        }

        // TODO: handle packages and other sources
        eprintln!("accessing file id: {id:?}");
        Err(FileError::AccessDenied)
    }

}

/// This is the interface we have to implement such that `typst` can compile it.
///
/// I have tried to keep it as minimal as possible
impl typst::World for TypstWorld {
    /// Standard library.
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    /// Metadata about all known Books.
    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }

    /// Accessing the main source file.
    fn main(&self) -> FileId {
        self.source.id()
    }

    /// Accessing a specified source file (based on `FileId`).
    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.source.id() {
            Ok(self.source.clone())
        } else {
            self.file(id)?.source(id)
        }
    }

    /// Accessing a specified file (non-file).
    fn file(&self, id: FileId) -> FileResult<Bytes> {
        self.file(id).map(|file| file.bytes.clone())
    }

    /// Accessing a specified font per index of font book.
    fn font(&self, id: usize) -> Option<Font> {
        self.fonts.get(id).cloned()
    }

    /// Get the current date.
    ///
    /// Optionally, an offset in hours is given.
    fn today(&self, offset: Option<i64>) -> Option<Datetime> {
        let offset = offset.unwrap_or(0);
        let offset = time::UtcOffset::from_hms(offset.try_into().ok()?, 0, 0).ok()?;
        let time = self.time.checked_to_offset(offset)?;
        Some(Datetime::Date(time.date()))
    }
}