-- Movies: auditorium layout for a title (bookings reference movies.slug)
CREATE TABLE movies (
    id INTEGER PRIMARY KEY,
    slug TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL,
    row_count INTEGER NOT NULL,
    seats_per_row INTEGER NOT NULL
);

CREATE INDEX movies_slug_idx ON movies (slug);
