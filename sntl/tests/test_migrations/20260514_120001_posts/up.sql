CREATE TABLE posts (id int PRIMARY KEY, user_id int NOT NULL REFERENCES users(id), title text NOT NULL);
