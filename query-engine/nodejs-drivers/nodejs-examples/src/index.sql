-- Create a new database and insert some data

CREATE DATABASE test;

USE test;

CREATE TABLE company (
	id INT PRIMARY KEY,
	name VARCHAR(10) NOT NULL
);

CREATE TABLE some_user (
	id   INT PRIMARY KEY,
	firstname VARCHAR(30) NOT NULL,
	lastname VARCHAR(30) NOT NULL,
	company_id INT NOT NULL,
	FOREIGN KEY (company_id) REFERENCES company(id)
);

INSERT INTO company(id, name) VALUES
	(1, 'Prisma');

INSERT INTO some_user(id, firstname, lastname, company_id) VALUES
	(1, 'Alberto', 'S', 1),
	(2, 'Tom', 'H', 1);
