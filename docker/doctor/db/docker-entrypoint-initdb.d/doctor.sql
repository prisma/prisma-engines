CREATE EXTENSION pg_stat_statements;

-- Create the "users" table
    CREATE TABLE users (
        id INT PRIMARY KEY,
        username VARCHAR(255) NOT NULL,
        latest_purchase TIMESTAMP DEFAULT NOW()
    );

    -- Create the "purchases" table
    CREATE TABLE purchases (
        purchase_id INT PRIMARY KEY,
        item VARCHAR(255) NOT NULL,
        price DECIMAL NOT NULL DEFAULT 0,
        user_id INT,        
        FOREIGN KEY (user_id) REFERENCES users(id)
    );

-- Insert random data into the "users" table
INSERT INTO users (id, username)
SELECT generate_series(1, 1000000), 'User ' || generate_series(1, 1000000);
-- 
INSERT INTO users (id, username) VALUES(1000001, 'Miguel');
SELECT pg_sleep(1);
INSERT INTO users (id, username) VALUES(1000002, 'Tyler');
SELECT pg_sleep(1);
INSERT INTO users (id, username) VALUES(1000003, 'Petra');
SELECT pg_sleep(1);
INSERT INTO users (id, username) VALUES(1000004, 'Marcus');

-- Insert random data into the "purchases" table
INSERT INTO purchases (purchase_id, item, user_id)
SELECT generate_series(1, 2000000), 'Random item', floor(random() * 1000000) + 1;

INSERT INTO purchases (purchase_id, item, user_id, price) VALUES(2000001, 'Book: Hack like there is no tomorrow', 1000001, 29.99);
INSERT INTO purchases (purchase_id, item, user_id, price) VALUES(2000002, 'Film: Paradigm Lost', 1000001, 9.99);
INSERT INTO purchases (purchase_id, item, user_id, price) VALUES(2000003, 'Book: Build APIs quickly with rust and actix', 1000001, 49.95);
INSERT INTO purchases (purchase_id, item, user_id, price) VALUES(2000004, 'Book: Deep-dive to remix', 1000002, 10.0);
INSERT INTO purchases (purchase_id, item, user_id, price) VALUES(2000005, 'Book: Pro ChatGPT prompt engineering', 1000002, 120.00);