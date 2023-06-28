CREATE EXTENSION pg_stat_statements;

-- Create the "users" table
    CREATE TABLE users (
        id INT PRIMARY KEY,
        username VARCHAR(255)
    );

    -- Create the "posts" table
    CREATE TABLE posts (
        post_id INT PRIMARY KEY,
        post_content VARCHAR(255),
        user_id INT,
        FOREIGN KEY (user_id) REFERENCES users(id)
    );

-- Insert random data into the "users" table
INSERT INTO users (id, username)
SELECT generate_series(1, 1000000), 'User ' || generate_series(1, 1000000);

-- Insert random data into the "posts" table
INSERT INTO posts (post_id, post_content, user_id)
SELECT generate_series(1, 2000000), 'Post ' || generate_series(1, 1000000), floor(random() * 1000000) + 1;