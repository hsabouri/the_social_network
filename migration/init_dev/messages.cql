CREATE TABLE IF NOT EXISTS messages (
    message_id UUID,
    user_id UUID,
    date_bucket TIMESTAMP,
    date TIMESTAMP,
    content TEXT,
    PRIMARY KEY ((user_id, date_bucket), date, tweet_id)
) WITH CLUSTERING ORDER BY (date DESC);

CREATE TABLE IF NOT EXISTS read_tag (
    user_id UUID,
    message_id UUID,
    PRIMARY KEY (user_id, tweet_id)
);

# Tweets of Alice: a1234567-1234-5678-1234-567812345678
INSERT INTO messages ( user_id, date_bucket, date, message_id, content )
    VALUES ( 'a1234567-1234-5678-1234-567812345678', '2023-02-06T00:00+0000', '2023-02-09T11:23:01+0000', 't1234567-1234-5678-1234-567812345678', 'My first tweet on the best #socialNetwork #first #2023' );
INSERT INTO messages ( user_id, date_bucket, date, message_id, content )
    VALUES ( 'a1234567-1234-5678-1234-567812345678', '2023-02-27T00:00+0000', '2023-03-01T14:39:47+0000', 't2234567-1234-5678-1234-567812345678', 'Nice day today, I have many followers ! (2) #famous' );
INSERT INTO messages ( user_id, date_bucket, date, message_id, content )
    VALUES ( 'a1234567-1234-5678-1234-567812345678', '2023-03-06T00:00+0000', '2023-03-06T09:01:30+0000', 't3234567-1234-5678-1234-567812345678', '#workout for a new life #fit #sport' );

# Tweets of Bob: b1234567-1234-5678-1234-567812345678
INSERT INTO messages ( user_id, date_bucket, date, message_id, content )
    VALUES ( 'b1234567-1234-5678-1234-567812345678', '2023-01-30T00:00+0000', '2023-01-31T12:13:29+0000', 't4234567-1234-5678-1234-567812345678', 'Hello the #world ouai' );
INSERT INTO messages ( user_id, date_bucket, date, message_id, content )
    VALUES ( 'b1234567-1234-5678-1234-567812345678', '2023-02-27T00:00+0000', '2023-03-02T14:39:47+0000', 't5234567-1234-5678-1234-567812345678', 'I got nice shooes today #shoes' );
INSERT INTO messages ( user_id, date_bucket, date, message_id, content )
    VALUES ( 'b1234567-1234-5678-1234-567812345678', '2023-03-13T00:00+0000', '2023-03-15T09:01:30+0000', 't6234567-1234-5678-1234-567812345678', 'Best music by @ChineseMan ouai' );

# Tweets of Charlie: c1234567-1234-5678-1234-567812345678
INSERT INTO messages ( user_id, date_bucket, date, message_id, content )
    VALUES ( 'c1234567-1234-5678-1234-567812345678', '2023-03-06T00:00+0000', '2023-03-10T16:20:29+0000', 't7234567-1234-5678-1234-567812345678', 'Charliiiiie #damn' );
INSERT INTO messages ( user_id, date_bucket, date, message_id, content )
    VALUES ( 'c1234567-1234-5678-1234-567812345678', '2023-03-13T00:00+0000', '2023-03-17T16:21:47+0000', 't8234567-1234-5678-1234-567812345678', 'Chaaarliie #woh' );
INSERT INTO messages ( user_id, date_bucket, date, message_id, content )
    VALUES ( 'c1234567-1234-5678-1234-567812345678', '2023-03-20T00:00+0000', '2023-03-24T16:20:12+0000', 't9234567-1234-5678-1234-567812345678', 'I\'m charliiie #yeah' );

# All messages are unseen (so no tags are added)