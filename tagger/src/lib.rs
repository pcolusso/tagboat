use include_dir::{include_dir, Dir};
use lazy_static::lazy_static;
use rusqlite::Connection;
use rusqlite_migration::Migrations;
use std::path::Path;
use thiserror::Error;
use time::OffsetDateTime;

#[repr(C)]
pub struct App {
    connection: Connection,
}

static MIGRATIONS_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/migrations");

// Define migrations. These are applied atomically.
lazy_static! {
    static ref MIGRATIONS: Migrations<'static> =
        Migrations::from_directory(&MIGRATIONS_DIR).unwrap();
}

impl App {
    pub fn init<P: AsRef<Path>>(path: P) -> Result<Self, TaggerError> {
        let mut connection = Connection::open(path)?;
        connection.pragma_update_and_check(None, "journal_mode", &"WAL", |_| Ok(()))?;

        MIGRATIONS
            .to_latest(&mut connection)
            .map_err(|_e| TaggerError::DirectoryError())?;

        Ok(Self { connection })
    }

    // TODO: Handle duplicate files?
    pub fn create_file(&mut self, filename: &str) -> Result<FileID, TaggerError> {
        self.connection
            .execute("INSERT INTO files (filename) VALUES (?1)", [filename])?;
        let id = self.connection.last_insert_rowid();
        Ok(FileID(id))
    }

    pub fn get_file(&mut self, filename: &str) -> Option<FileID> {
        match self.connection.query_row(
            "SELECT id FROM files WHERE filename = ?1",
            [filename],
            |r| r.get(0),
        ) {
            Ok(id) => Some(FileID(id)),
            Err(rusqlite::Error::QueryReturnedNoRows) => None,
            Err(e) => panic!("SQL Error, {}", e),
        }
    }

    pub fn create_tag(&mut self, tag_name: &str) -> Result<TagID, TaggerError> {
        // Unsure if this is more or less idiomatic than the above. Maybe this can handle the
        // collision case better?
        let res = self.connection.query_row(
            "INSERT INTO tags (name) VALUES (?1) RETURNING id",
            [&tag_name],
            |r| r.get(0),
        );
        match res {
            Ok(id) => Ok(TagID(id)),
            Err(e) => Err(TaggerError::DatabaseError(e)),
        }
    }

    pub fn get_tag(&mut self, tag_name: &str) -> Option<TagID> {
        match self
            .connection
            .query_row("SELECT id FROM tags WHERE name = ?1", [&tag_name], |r| {
                r.get(0)
            }) {
            Ok(id) => Some(TagID(id)),
            Err(rusqlite::Error::QueryReturnedNoRows) => None,
            Err(e) => panic!("SQL Error, {}", e),
        }
    }

    pub fn tag_file(&mut self, tag: TagID, file: FileID) {
        let res = self.connection.execute(
            "INSERT INTO file_tags (file_id, tag_id) VALUES (?1, ?2)",
            [file.0, tag.0],
        );
        if let Err(e) = res {
            println!("SQL ERROR {}", e);
            panic!("SQL Error, {}", e);
        }
    }

    pub fn get_files_for_tag(&mut self, tag: TagID) -> Result<Vec<File>, TaggerError> {
        let query = "
            SELECT files.id,
                files.filename,
                files.last_seen_at,
                files.orphaned_at,
                files.updated_at,
                files.created_at 
            FROM file_tags 
            INNER JOIN files ON files.id = file_tags.file_id
            WHERE file_tags.tag_id = ?1";
        let mut statement = self.connection.prepare(query)?;
        let files: Vec<_> = statement
            .query_map([tag.0], |row| {
                Ok(File {
                    id: FileID(row.get(0).expect("SQL Error")),
                    file_name: row.get(1).expect("SQL Error"),
                    last_seen_at: row.get(2).expect("SQL Error"),
                    orphaned_at: row.get(3).expect("SQL Error"),
                    updated_at: row.get(4).expect("SQL Error"),
                    created_at: row.get(5).expect("SQL Error"),
                })
            })?
            .into_iter()
            .map(|result| result.ok())
            .flatten()
            .collect();
        Ok(files)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct FileID(i64);
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TagID(i64);

#[derive(Debug)]
pub struct File {
    pub id: FileID,
    pub file_name: String,
    pub last_seen_at: Option<OffsetDateTime>,
    pub orphaned_at: Option<OffsetDateTime>,
    pub updated_at: OffsetDateTime,
    pub created_at: OffsetDateTime,
}

#[derive(Error, Debug)]
pub enum TaggerError {
    #[error("Can't access directory.")]
    DirectoryError(),
    #[error("SQLite Issue")]
    DatabaseError(#[from] rusqlite::Error),
}

#[cfg(test)]
mod tests {
    use crate::*;
    type TR = Result<(), TaggerError>;

    fn test_app() -> Result<App, TaggerError> {
        let mut connection = Connection::open_in_memory()?;
        //let mut connection = Connection::open("test.db")?;
        // This has it's own dedicated test anyway.
        MIGRATIONS.to_latest(&mut connection).unwrap();
        Ok(App { connection })
    }

    #[test]
    fn test_get_file_id() -> TR {
        let mut app = test_app()?;
        let filename = "abc";
        assert!(app.get_file(filename).is_none());
        app.create_file(filename).unwrap();
        assert!(app.get_file(filename).is_some());
        Ok(())
    }

    #[test]
    fn test_get_tag_id() -> TR {
        let mut app = test_app()?;
        let tagname = "abc";
        assert!(app.get_tag(tagname).is_none());
        app.create_tag(tagname).unwrap();
        assert!(app.get_tag(tagname).is_some());
        Ok(())
    }

    #[test]
    fn tag_a_file() -> TR {
        let mut app = test_app()?;
        let filename = "abc";
        let tagname = "cba";
        let file_id = app.create_file(tagname)?;
        let tag_id = app.create_tag(filename)?;
        app.tag_file(tag_id, file_id);
        Ok(())
    }

    #[test]
    fn get_files_for_tag() -> TR {
        let mut app = test_app()?;
        let a = app.create_file("a")?;
        println!("A ID: {:?}", a);
        let b = app.create_file("b")?;
        println!("B ID: {:?}", b);
        let c = app.create_file("c")?;
        println!("C ID: {:?}", c);
        let tag = app.create_tag("tag")?;

        app.tag_file(tag, a);
        app.tag_file(tag, b);

        let res = app.get_files_for_tag(tag)?;
        println!("Res: {:?}", res);
        let ids: Vec<FileID> = res.into_iter().map(|f| f.id).collect();
        println!("Ids: {:?}", ids);
        assert!(ids.contains(&a));
        assert!(ids.contains(&b));
        assert!(ids.contains(&c) == false);

        Ok(())
    }

    #[test]
    fn migrations_test() {
        assert!(MIGRATIONS.validate().is_ok());
    }
}
