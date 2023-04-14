CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE IF NOT EXISTS friendships (
    friendship_id SERIAL,
    user_id UUID NOT NULL REFERENCES users(user_id),
    friend_id UUID NOT NULL REFERENCES users(user_id),
    PRIMARY KEY(friendship_id)
);

CREATE INDEX friendship_user_id_index ON friendships USING HASH (user_id);
CREATE INDEX friendship_friend_id_index ON friendships USING HASH (friend_id);

INSERT INTO friendships (user_id, friend_id) VALUES ('11234567-1234-5678-1234-567812345678', '21234567-1234-5678-1234-567812345678');
INSERT INTO friendships (user_id, friend_id) VALUES ('11234567-1234-5678-1234-567812345678', '31234567-1234-5678-1234-567812345678');