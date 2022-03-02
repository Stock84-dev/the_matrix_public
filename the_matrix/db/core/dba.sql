REVOKE CREATE ON SCHEMA public FROM PUBLIC;
REVOKE ALL ON DATABASE dev FROM PUBLIC;

CREATE ROLE user_role;

GRANT CONNECT ON DATABASE dev TO user_role;
GRANT USAGE ON SCHEMA public TO user_role;
GRANT SELECT, INSERT ON ALL TABLES IN SCHEMA public TO user_role;
GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA public TO user_role;
GRANT EXECUTE ON ALL PROCEDURES IN SCHEMA public TO user_role;

CREATE USER user3 WITH PASSWORD 'pass';
GRANT user_role TO user3;

DROP OWNED BY user2;
DROP USER user2;

CREATE TABLE dba.users
(
    id           SERIAL PRIMARY KEY,
    username     name         NOT NULL, -- REFERENCES pg_roles (rolname),
    email        VARCHAR(126) NOT NULL,
    name         VARCHAR(126) NOT NULL,
    surname      VARCHAR(126) NOT NULL,
    date_created DATE         NOT NULL
);



CREATE OR REPLACE PROCEDURE dba.create_user(username VARCHAR(126), email VARCHAR(126), password VARCHAR(126),
                                            name VARCHAR(126),
                                            surname VARCHAR(126))
    LANGUAGE 'plpgsql'
AS
$$
BEGIN
    IF username !~ '^[a-zA-Z0-9_]+$' THEN
        RAISE EXCEPTION 'invalid username';
    END IF;

    IF password ~ '[\\''"]+' THEN
        RAISE EXCEPTION 'invalid password, non valid characters are: \ '' "' ;
    END IF;

    EXECUTE FORMAT(E'CREATE USER %1$s WITH PASSWORD \'%2$s\'; GRANT user_role TO %1$s;', username, password);
    INSERT INTO dba.users(username, email, name, surname, date_created)
    VALUES (username, email, name, surname, CURRENT_DATE);
END
$$;

CREATE OR REPLACE PROCEDURE dba.delete_user(p_id INT)
    LANGUAGE 'plpgsql'
AS
$$
DECLARE
    v_username name;
BEGIN
    SELECT username INTO v_username FROM dba.users WHERE dba.users.id = p_id;
    DELETE FROM dba.users WHERE id = p_id;
    EXECUTE FORMAT(E'DROP OWNED BY %1$s; DROP USER %1$s;', v_username);
END
$$;


CREATE OR REPLACE FUNCTION dba.current_user()
    returns int
    LANGUAGE 'plpgsql'
AS
$$
BEGIN
    return (select id from dba.users where username = "current_user"());
END
$$;
