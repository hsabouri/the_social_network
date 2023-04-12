CREATE TABLE IF NOT EXISTS messages (
    message_id TUPLE<UUID, TIMESTAMP>,
    user_id UUID,
    date_bucket TIMESTAMP,
    date TIMESTAMP,
    content TEXT,
    PRIMARY KEY ((user_id, date_bucket), date, message_id)
) WITH CLUSTERING ORDER BY (date DESC);

CREATE TABLE IF NOT EXISTS read_tags (
    user_id UUID,
    message_id UUID,
    PRIMARY KEY (user_id, message_id)
);

-- Tweets of Alice: a1234567-1234-5678-1234-567812345678
INSERT INTO messages ( user_id, date_bucket, date, message_id, content )
    VALUES ( 11234567-1234-5678-1234-567812345678, '2023-02-06T00:00+0000', '2023-02-09T11:23:01+0000', (11234567-1234-5678-1234-567812345678, 1681374193000), 'My first tweet on the best #socialNetwork first 2023' );
INSERT INTO messages ( user_id, date_bucket, date, message_id, content )
    VALUES ( 11234567-1234-5678-1234-567812345678, '2023-02-27T00:00+0000', '2023-03-01T14:39:47+0000', (11234567-1234-5678-1234-567812345678, 1681374393000), 'Nice day today, I have many followers ! (2) famous' );
INSERT INTO messages ( user_id, date_bucket, date, message_id, content )
    VALUES ( 11234567-1234-5678-1234-567812345678, '2023-03-06T00:00+0000', '2023-03-06T09:01:30+0000', (11234567-1234-5678-1234-567812345678, 1681374693000), 'workout for a new life fit sport' );

-- Tweets of Bob: b1234567-1234-5678-1234-567812345678
INSERT INTO messages ( user_id, date_bucket, date, message_id, content )
    VALUES ( 21234567-1234-5678-1234-567812345678, '2023-01-30T00:00+0000', '2023-01-31T12:13:29+0000', (21234567-1234-5678-1234-567812345678, 1681373693000), 'Hello the world ouai' );
INSERT INTO messages ( user_id, date_bucket, date, message_id, content )
    VALUES ( 21234567-1234-5678-1234-567812345678, '2023-02-27T00:00+0000', '2023-03-02T14:39:47+0000', (21234567-1234-5678-1234-567812345678, 1681374693000), 'I got nice shooes today shoes' );
INSERT INTO messages ( user_id, date_bucket, date, message_id, content )
    VALUES ( 21234567-1234-5678-1234-567812345678, '2023-03-13T00:00+0000', '2023-03-15T09:01:30+0000', (21234567-1234-5678-1234-567812345678, 1681374893000), 'Best music by @ChineseMan ouai' );

-- Tweets of Charlie: c1234567-1234-5678-1234-567812345678
INSERT INTO messages ( user_id, date_bucket, date, message_id, content )
    VALUES ( 31234567-1234-5678-1234-567812345678, '2023-03-06T00:00+0000', '2023-03-10T16:20:29+0000', ( 31234567-1234-5678-1234-567812345678, 1681373293000), 'Charliiiiie damn' );
INSERT INTO messages ( user_id, date_bucket, date, message_id, content )
    VALUES ( 31234567-1234-5678-1234-567812345678, '2023-03-13T00:00+0000', '2023-03-17T16:21:47+0000', ( 31234567-1234-5678-1234-567812345678, 1681374293000), 'Chaaarliie woh' );
INSERT INTO messages ( user_id, date_bucket, date, message_id, content )
    VALUES ( 31234567-1234-5678-1234-567812345678, '2023-03-20T00:00+0000', '2023-03-24T16:20:12+0000', ( 31234567-1234-5678-1234-567812345678, 1681374693000), 'I''m charliiie yeah' );

-- All messages are unseen (so no tags are added)