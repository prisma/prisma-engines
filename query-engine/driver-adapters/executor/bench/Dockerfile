FROM postgres:15
COPY seed.sql.gz .
RUN gunzip seed.sql.gz && \
    mv seed.sql /docker-entrypoint-initdb.d/seed.sql