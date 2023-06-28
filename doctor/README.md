From the root of the repository:

#### Install all the things!

* `cargo build -p query-engine`
* `doctor/start` (will build db and doctor images and start both)
* In a different tab `doctor/start-engine`, which will start a binary version of the query-engine
hacked to send information to `doctor`
* TODO: tyler to fill-in dashboard details.


#### Submit a query to the database

* `cd doctor_repro`
* `npm install typescript prisma ts-node`
* `prisma generate`
* `npx ts-node index.ts`

#### Interesting Info:

- Urls:
    - postgresql://postgres:prisma@localhost:5432 -> The DB
    - http://127.0.0.1:8080 -> doctor. Routes:
        - `GET /slow-queries?threshold=FLOAT&k=INT`
        - `POST /clear-stats`
        - `POST /submit-query` JSON Body:
        
        ```
        {
                "raw_query": SQL,
                "tag": string,
                "prisma_query":  "String"
        }
        ```
    - http://127.0.0.1:57581 -> The query engine





