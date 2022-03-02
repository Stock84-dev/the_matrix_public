CREATE TABLE updates
(
    ts    TIMESTAMP PRIMARY KEY,
    major int2  NOT NULL,
    minor int2  NOT NULL,
    patch int2  NOT NULL,
    data  bytea NOT NULL
);

CREATE TABLE logs
(
    ts      TIMESTAMP PRIMARY KEY,
    kind    int2         NOT NULL,
    host    VARCHAR(255) NOT NULL,
    message TEXT         NOT NULL
);
