ALTER DEFAULT PRIVILEGES IN SCHEMA public
    GRANT SELECT ON TABLES TO user_role;

CREATE OR REPLACE PROCEDURE create_user(username VARCHAR(126), email VARCHAR(126), password VARCHAR(126),
                                        name VARCHAR(126),
                                        surname VARCHAR(126))
    LANGUAGE 'plpgsql'
    SECURITY DEFINER
AS
$$
BEGIN
    CALL dba.create_user(username, email, password, name, surname);
END
$$;


CREATE OR REPLACE FUNCTION create_file(path VARCHAR(126))
    RETURNS INT
    LANGUAGE 'plpgsql'
    SECURITY DEFINER
AS
$$
DECLARE
    file_id INT;
BEGIN
    INSERT INTO files(user_id, path) VALUES (dba.cur_user(), path) RETURNING id INTO file_id;
    RETURN file_id;
END
$$;

SELECT *
FROM create_file('aaa'::VARCHAR);

CREATE OR REPLACE FUNCTION load_file_id_for_user(p_path VARCHAR(126))
    RETURNS SETOF INT
    LANGUAGE 'sql'
    SECURITY DEFINER
AS
$$
SELECT id
FROM files
WHERE files.path = p_path
  AND files.user_id = dba.cur_user();
$$;

CREATE OR REPLACE FUNCTION load_file_id(p_path VARCHAR(126))
    RETURNS SETOF INT
    LANGUAGE 'sql'
    SECURITY DEFINER
AS
$$
SELECT id
FROM files
WHERE files.path = p_path;
$$;

CREATE TABLE hosts
(
    id   SMALLSERIAL PRIMARY KEY,
    path VARCHAR(126)
);

CREATE TABLE files
(
    id      SERIAL PRIMARY KEY,
    user_id INT          NOT NULL REFERENCES dba.users (id),
    host_id SMALLINT     NOT NULL REFERENCES hosts (id),
    path    VARCHAR(126) NOT NULL
);

CREATE TABLE file_hosts
(
    path    VARCHAR(126) PRIMARY KEY,
    host_id SMALLINT
);

-- 48 bytes
CREATE TABLE hlcv_blocks
(
    start_time    TIMESTAMP PRIMARY KEY,
    file_id       INT       NOT NULL REFERENCES files (id),
    end_time      TIMESTAMP NOT NULL,
    high_pos      BIGINT    NOT NULL,
    low_offset    INT       NOT NULL,
    close_offset  INT       NOT NULL,
    volume_offset INT       NOT NULL
);

CREATE OR REPLACE FUNCTION get_or_create_file(p_path VARCHAR(126))
    RETURNS TABLE
            (
                file_id   INT,
                host_path VARCHAR(126)
            )
    LANGUAGE 'plpgsql'
    SECURITY DEFINER
AS
$$
DECLARE
    v_host_id SMALLINT DEFAULT NULL;
    v_path    VARCHAR(126);
    pos       INT;
BEGIN
    v_path = p_path;
    SELECT id INTO file_id FROM files WHERE path = p_path LIMIT 1;
    IF file_id IS NULL THEN
        file_id = create_file(p_path);
    END IF;
    LOOP
        SELECT host_id INTO v_host_id FROM file_hosts WHERE path = v_path LIMIT 1;
        IF v_host_id IS NOT NULL THEN
            SELECT path INTO host_path FROM hosts WHERE id = v_host_id;
            RETURN NEXT;
            RETURN;
        END IF;
        pos = STRPOS(REVERSE(v_path), '/');
        SELECT LEFT(v_path, CHAR_LENGTH(v_path) - pos) INTO v_path;
        IF pos = 0 THEN
            EXIT;
        END IF;
    END LOOP;
    SELECT host_id INTO v_host_id FROM file_hosts WHERE path = v_path LIMIT 1;
    IF v_host_id IS NOT NULL THEN
        SELECT path INTO host_path FROM hosts WHERE id = v_host_id;
        RETURN NEXT;
        RETURN;
    END IF;
    host_path = '';
    RETURN NEXT;
    RETURN;
END
$$;

CREATE OR REPLACE FUNCTION create_research_result_block_table(file_id INT)
    RETURNS VARCHAR
    LANGUAGE 'plpgsql'
    SECURITY DEFINER
AS
$$
DECLARE
    table_name VARCHAR;
BEGIN
    IF file_id NOT IN (SELECT id FROM files) THEN
        RAISE INVALID_FOREIGN_KEY;
    END IF;

    table_name = CONCAT('research_result_blocks_', file_id);

    -- 72 bytes + 12 bits
    -- number at the end is file_id
    EXECUTE FORMAT('
CREATE TABLE %s
(
    combination_pos      BIGINT NOT NULL PRIMARY KEY,
    combination_min      BIGINT NOT NULL,
    combination_max      BIGINT NOT NULL,
    balances_offset      INT,
    balances_min         REAL,
    balances_max         REAL,
    max_balances_offset  INT,
    max_balances_min     REAL,
    max_balances_max     REAL,
    max_drawdowns_offset INT,
    max_drawdowns_min    REAL,
    max_drawdowns_max    REAL,
    n_trades_offset      INT,
    n_trades_min     INT,
    n_trades_max     INT
    -- add others
);', table_name);
    RETURN table_name;
END
$$;


CREATE TABLE omg
(
    seq        SERIAL NOT NULL PRIMARY KEY,
    stampthing TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO omg(stampthing)
SELECT NOW();

INSERT INTO omg(stampthing)
VALUES ('epoch'::timestamptz)
     , ('epoch'::timestamptz + 12569537329 * '1 second'::INTERVAL)
;

SELECT stampthing
     , DATE_PART('epoch', stampthing) AS original
FROM omg;

SELECT TO_TIMESTAMP(1321315200);


SELECT EXTRACT(EPOCH FROM '1996-12-19T16:39:57-08:00'::TIMESTAMP)::INTEGER;
SELECT EXTRACT(EPOCH FROM '1996-12-19 16:39:57'::TIMESTAMP)::INTEGER;
SELECT EXTRACT(EPOCH FROM '2000-11-15 00:00:00'::TIMESTAMP)::INTEGER;

SELECT SUBSTR('home/user/data', CHAR_LENGTH('home/user/data') - STRPOS(REVERSE('home/user/data'), '/'));
SELECT LEFT('home', CHAR_LENGTH('home') - STRPOS(REVERSE('home'), '/'));
SELECT REVERSE('w3resource');

SELECT RIGHT('home/user/data', 3);
