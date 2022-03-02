CREATE SCHEMA IF NOT EXISTS zion DEFAULT CHARACTER SET utf8;
USE zion;

CREATE TABLE IF NOT EXISTS zion.topic_layouts
(
    id   INT UNSIGNED     NOT NULL AUTO_INCREMENT,
    modified_s          BIGINT NOT NULL,
    bits TINYINT UNSIGNED NOT NULL,
    PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS zion.system_layouts
(
    id                  INT UNSIGNED     NOT NULL AUTO_INCREMENT,
    modified_s          BIGINT NOT NULL,
    kind                TINYINT UNSIGNED NOT NULL,
    ram_usage_bytes     BIGINT UNSIGNED  NULL,
    thread_usage        TINYINT UNSIGNED NULL,
    io_read_bytes       BIGINT UNSIGNED  NULL,
    io_write_bytes      BIGINT UNSIGNED  NULL,
    network_read_bytes  BIGINT UNSIGNED  NULL,
    network_write_bytes BIGINT UNSIGNED  NULL,
    PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS zion.workflows
(
    id INT UNSIGNED NOT NULL AUTO_INCREMENT,
    PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS zion.system_spawn_configs
(
    workflows_id      INT UNSIGNED     NOT NULL,
    system_layouts_id INT UNSIGNED     NOT NULL,
    input_topics      VARBINARY(65535) NOT NULL,
    output_topics     VARBINARY(65535) NOT NULL,
    consts            VARBINARY(65535) NULL,
    PRIMARY KEY (workflows_id, system_layouts_id),
    INDEX fk_system_spawn_configs_workflows1_idx (workflows_id ASC) VISIBLE,
    CONSTRAINT fk_system_spawn_configs_system_layouts
        FOREIGN KEY (system_layouts_id)
            REFERENCES zion.system_layouts (id)
            ON DELETE NO ACTION
            ON UPDATE NO ACTION,
    CONSTRAINT fk_system_spawn_configs_workflows1
        FOREIGN KEY (workflows_id)
            REFERENCES zion.workflows (id)
            ON DELETE NO ACTION
            ON UPDATE NO ACTION
);

CREATE TABLE IF NOT EXISTS zion.nodes
(
    id INT UNSIGNED NOT NULL AUTO_INCREMENT,
    name VARCHAR(255),
    PRIMARY KEY (id)
);
