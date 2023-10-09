CREATE TABLE post (
    id INTEGER PRIMARY KEY,
    title VARCHAR(255) NOT NULL,
    timestamp INTEGER NOT NULL,
    slug VARCHAR(255) UNIQUE NOT NULL,
    text_content TEXT,
    text_rendered TEXT
);
