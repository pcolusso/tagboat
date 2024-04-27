CREATE TABLE file_tags(
    file_id INTEGER NOT NULL,
    tag_id INTEGER NOT NULL,
    PRIMARY KEY(file_id, tag_id),
    FOREIGN KEY(file_id) REFERENCES files(id)
    FOREIGN KEY(tag_id) REFERENCES tags(id)
);
