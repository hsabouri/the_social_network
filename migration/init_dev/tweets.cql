CREATE TABLE IF NOT EXISTS tweets (
    user_id UUID,
    date_bucket TIMESTAMP,
    date TIMESTAMP,
    tweet_id UUID,
    content TEXT,
    PRIMARY KEY ((user_id, date_bucket), date, tweet_id)
) WITH CLUSTERING ORDER BY (date DESC, tweet_id DESC);